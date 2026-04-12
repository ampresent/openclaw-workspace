# Monitoring Stack NixOS Module for UtopOS
#
# All-in-one monitoring: Prometheus + Grafana + Loki + Node Exporter + Alertmanager
# with pre-configured dashboards and alert rules.
#
# Usage in configuration.nix:
#   imports = [ ./modules/monitoring-stack.nix ];
#   services.UtopOS.monitoring.enable = true;

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.UtopOS.monitoring;
in {
  options.services.UtopOS.monitoring = {
    enable = mkEnableOption "UtopOS managed monitoring stack";

    hostname = mkOption {
      type = types.str;
      default = "monitor.example.com";
      description = "Domain name for Grafana dashboard";
    };

    enableGrafana = mkOption {
      type = types.bool;
      default = true;
      description = "Enable Grafana dashboards";
    };

    enablePrometheus = mkOption {
      type = types.bool;
      default = true;
      description = "Enable Prometheus metrics collection";
    };

    enableLoki = mkOption {
      type = types.bool;
      default = true;
      description = "Enable Loki log aggregation";
    };

    enableAlertmanager = mkOption {
      type = types.bool;
      default = false;
      description = "Enable Alertmanager for alert routing";
    };

    retentionDays = mkOption {
      type = types.int;
      default = 30;
      description = "Prometheus metrics retention in days";
    };

    lokiRetentionDays = mkOption {
      type = types.int;
      default = 30;
      description = "Loki log retention in days";
    };

    extraScrapeConfigs = mkOption {
      type = types.listOf types.attrs;
      default = [];
      description = "Extra Prometheus scrape configs";
    };

    alertWebhookUrl = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Webhook URL for alert notifications";
    };

    adminPassword = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Grafana admin password (use grafanaAdminPasswordFile for production)";
    };
  };

  config = mkIf cfg.enable {
    # ============================================
    # Prometheus
    # ============================================
    services.prometheus = mkIf cfg.enablePrometheus {
      enable = true;
      port = 9090;
      retentionTime = "${toString cfg.retentionDays}d";

      scrapeConfigs = [
        {
          job_name = "prometheus";
          static_configs = [{ targets = [ "localhost:9090" ]; }];
        }
        {
          job_name = "node";
          static_configs = [{ targets = [ "localhost:9100" ]; }];
          scrape_interval = "15s";
        }
        {
          job_name = "UtopOS";
          static_configs = [{ targets = [ "localhost:$(cat /etc/UtopOS/port 2>/dev/null || echo 3030)" ]; }];
          metrics_path = "/api/observability/metrics";
          scrape_interval = "30s";
        }
      ] ++ cfg.extraScrapeConfigs;

      # Alert rules
      rules = [
        (builtins.toJSON {
          groups = [{
            name = "UtopOS-alerts";
            rules = [
              {
                alert = "HighDiskUsage";
                expr = "node_filesystem_avail_bytes{mountpoint=\"/\"} / node_filesystem_size_bytes{mountpoint=\"/\"} < 0.1";
                for = "5m";
                labels.severity = "critical";
                annotations = {
                  summary = "Disk space critically low";
                  description = "{{ $labels.instance }} has less than 10% disk space remaining.";
                };
              }
              {
                alert = "HighMemoryUsage";
                expr = "node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes < 0.1";
                for = "5m";
                labels.severity = "warning";
                annotations = {
                  summary = "Memory usage is very high";
                  description = "{{ $labels.instance }} has less than 10% memory available.";
                };
              }
              {
                alert = "ServiceDown";
                expr = "up == 0";
                for = "1m";
                labels.severity = "critical";
                annotations = {
                  summary = "Service is down";
                  description = "{{ $labels.job }} on {{ $labels.instance }} is down.";
                };
              }
              {
                alert = "HighCPUUsage";
                expr = "100 - (avg by(instance) (irate(node_cpu_seconds_total{mode=\"idle\"}[5m])) * 100) > 95";
                for = "10m";
                labels.severity = "warning";
                annotations = {
                  summary = "CPU usage is very high";
                  description = "{{ $labels.instance }} CPU is above 95% for 10 minutes.";
                };
              }
              {
                alert = "NixOSGenerationChanged";
                expr = "changes(UtopOS_generation[5m]) > 0";
                for = "0m";
                labels.severity = "info";
                annotations = {
                  summary = "NixOS configuration changed";
                  description = "NixOS generation changed on {{ $labels.instance }}.";
                };
              }
            ];
          }];
        })
      ];

      alertmanager = mkIf cfg.enableAlertmanager {
        enable = true;
        configuration = {
          route = {
            receiver = "default";
            group_wait = "30s";
            group_interval = "5m";
            repeat_interval = "4h";
          };
          receivers = [{
            name = "default";
            webhook_configs = mkIf (cfg.alertWebhookUrl != null) [{
              url = cfg.alertWebhookUrl;
              send_resolved = true;
            }];
          }];
        };
      };
    };

    # ============================================
    # Node Exporter
    # ============================================
    services.prometheus.exporters.node = mkIf cfg.enablePrometheus {
      enable = true;
      port = 9100;
      enabledCollectors = [
        "systemd"
        "processes"
        "filesystem"
        "diskstats"
        "netdev"
        "meminfo"
        "cpu"
        "loadavg"
      ];
    };

    # ============================================
    # Loki
    # ============================================
    services.loki = mkIf cfg.enableLoki {
      enable = true;
      configuration = {
        server.http_listen_port = 3100;
        auth_enabled = false;

        ingester = {
          lifecycler = {
            address = "127.0.0.1";
            ring = {
              kvstore.store = "inmemory";
              replication_factor = 1;
            };
          };
          chunk_idle_period = "1h";
          max_chunk_age = "1h";
          chunk_target_size = 1048576;
          chunk_retain_period = "30s";
        };

        schema_config.configs = [{
          from = "2024-01-01";
          store = "tsdb";
          object_store = "filesystem";
          schema = "v13";
          index.prefix = "index_";
          index.period = "24h";
        }];

        storage_config = {
          tsdb_shipper = {
            active_index_directory = "/var/lib/loki/tsdb-index";
            cache_location = "/var/lib/loki/tsdb-cache";
          };
          filesystem.directory = "/var/lib/loki/chunks";
        };

        limits_config = {
          reject_old_samples = true;
          reject_old_samples_max_age = "${toString cfg.lokiRetentionDays}d";
        };

        compactor = {
          working_directory = "/var/lib/loki/compactor";
          compaction_interval = "10m";
          retention_enabled = true;
          retention_delete_delay = "2h";
          retention_delete_worker_count = 150;
          delete_request_store = "filesystem";
        };

        analytics.reporting_enabled = false;
      };
    };

    # Promtail for shipping logs to Loki
    services.promtail = mkIf cfg.enableLoki {
      enable = true;
      configuration = {
        server = {
          http_listen_port = 9080;
          grpc_listen_port = 0;
        };
        positions.filename = "/var/lib/promtail/positions.yaml";
        clients = [{
          url = "http://localhost:3100/loki/api/v1/push";
        }];
        scrape_configs = [{
          job_name = "journal";
          journal = {
            max_age = "12h";
            labels = {
              job = "systemd-journal";
              host = config.networking.hostName;
            };
          };
          relabel_configs = [{
            source_labels = [ "__journal__systemd_unit" ];
            target_label = "unit";
          }];
        }];
      };
    };

    # ============================================
    # Grafana
    # ============================================
    services.grafana = mkIf cfg.enableGrafana {
      enable = true;
      settings = {
        server = {
          domain = cfg.hostname;
          root_url = "https://${cfg.hostname}";
          http_port = 3000;
        };
        security = {
          admin_user = "admin";
        } // (optionalAttrs (cfg.adminPassword != null) {
          admin_password = cfg.adminPassword;
        });
      };

      provision = {
        enable = true;
        datasources.settings.datasources = [
          {
            name = "Prometheus";
            type = "prometheus";
            url = "http://localhost:9090";
            isDefault = true;
          }
        ] ++ (optional cfg.enableLoki {
          name = "Loki";
          type = "loki";
          url = "http://localhost:3100";
        });
      };
    };

    # Nginx reverse proxy for Grafana
    services.nginx = mkIf cfg.enableGrafana {
      enable = true;
      virtualHosts.${cfg.hostname} = {
        forceSSL = true;
        enableACME = true;
        locations."/" = {
          proxyPass = "http://localhost:3000";
          proxyWebsockets = true;
        };
      };
    };

    # ============================================
    # Firewall
    # ============================================
    networking.firewall.allowedTCPPorts = [ 80 443 ]
      ++ (optional cfg.enableGrafana 3000)
      ++ (optional cfg.enablePrometheus 9090)
      ++ (optional cfg.enableLoki 3100);

    # ============================================
    # UtopOS metadata
    # ============================================
    environment.etc."UtopOS/services/monitoring.json".text = builtins.toJSON {
      name = "monitoring-stack";
      type = "infrastructure";
      hostname = cfg.hostname;
      components = {
        prometheus = cfg.enablePrometheus;
        grafana = cfg.enableGrafana;
        loki = cfg.enableLoki;
        alertmanager = cfg.enableAlertmanager;
        nodeExporter = cfg.enablePrometheus;
      };
      endpoints = optionalAttrs cfg.enableGrafana {
        grafana = "https://${cfg.hostname}";
      } // optionalAttrs cfg.enablePrometheus {
        prometheus = "http://localhost:9090";
      } // optionalAttrs cfg.enableLoki {
        loki = "http://localhost:3100";
      };
      retention = {
        prometheus_days = cfg.retentionDays;
        loki_days = cfg.lokiRetentionDays;
      };
      UtopOS.managed = true;
    };
  };
}
