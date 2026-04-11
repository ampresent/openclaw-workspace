# Nextcloud NixOS Module for nix-evo
#
# Managed Nextcloud instance with automatic SSL, database, and Redis caching.
# Usage in configuration.nix:
#   imports = [ ./modules/nextcloud.nix ];
#   services.nix-evo.nextcloud.enable = true;

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.nix-evo.nextcloud;
in {
  options.services.nix-evo.nextcloud = {
    enable = mkEnableOption "nix-evo managed Nextcloud";

    hostname = mkOption {
      type = types.str;
      default = "cloud.example.com";
      description = "Domain name for Nextcloud instance";
    };

    adminUser = mkOption {
      type = types.str;
      default = "admin";
      description = "Nextcloud admin username";
    };

    adminPassFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to file containing admin password (for initial setup)";
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/nextcloud";
      description = "Nextcloud data directory";
    };

    maxUploadSize = mkOption {
      type = types.str;
      default = "512M";
      description = "Maximum upload file size";
    };

    enableCollabora = mkOption {
      type = types.bool;
      default = false;
      description = "Enable Collabora Online (LibreOffice Online) integration";
    };

    enableTalk = mkOption {
      type = types.bool;
      default = true;
      description = "Enable Nextcloud Talk for video calls";
    };

    extraApps = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Extra Nextcloud apps to install (app IDs from app store)";
      example = [ "deck" "notes" "calendar" ];
    };

    autoUpdate = mkOption {
      type = types.bool;
      default = true;
      description = "Auto-update Nextcloud apps";
    };

    backupEnable = mkOption {
      type = types.bool;
      default = false;
      description = "Enable automated backups via nix-evo";
    };

    backupSchedule = mkOption {
      type = types.str;
      default = "daily";
      description = "Backup schedule (systemd calendar format)";
    };
  };

  config = mkIf cfg.enable {
    services.nextcloud = {
      enable = true;
      hostName = cfg.hostname;
      home = cfg.dataDir;

      # HTTPS with auto Let's Encrypt
      https = true;

      config = {
        dbtype = "pgsql";
        dbuser = "nextcloud";
        dbhost = "/run/postgresql";
        dbname = "nextcloud";
        adminuser = cfg.adminUser;
        adminpassFile = cfg.adminPassFile;
      };

      settings = {
        "overwrite.cli.url" = "https://${cfg.hostname}";
        "overwriteprotocol" = "https";
        default_phone_region = "CN";
        log_type = "file";
        loglevel = 2;
        maintenance_window_start = 1; # 1 AM UTC maintenance window
      };

      maxUploadSize = cfg.maxUploadSize;

      extraAppsEnable = cfg.extraApps != [];
      extraApps = builtins.listToAttrs (map (app: {
        name = app;
        value = { };
      }) cfg.extraApps);

      autoUpdateApps.enable = cfg.autoUpdate;

      phpOptions = {
        "opcache.interned_strings_buffer" = "16";
        "opcache.max_accelerated_files" = "10000";
        "opcache.memory_consumption" = "128";
      };
    };

    # PostgreSQL for Nextcloud
    services.postgresql = {
      enable = true;
      ensureDatabases = [ "nextcloud" ];
      ensureUsers = [{
        name = "nextcloud";
        ensureDBOwnership = true;
      }];
    };

    # Redis for file locking and caching
    services.redis.servers."nextcloud" = {
      enable = true;
      port = 0;
      unixSocket = "/run/redis-nextcloud/redis.sock";
      unixSocketPerm = 770;
    };

    # Ensure Nextcloud waits for DB
    systemd.services.nextcloud-setup = {
      requires = [ "postgresql.service" ];
      after = [ "postgresql.service" ];
    };

    # Nginx reverse proxy with security headers
    services.nginx = {
      enable = true;
      virtualHosts.${cfg.hostname} = {
        forceSSL = true;
        enableACME = true;
        extraConfig = ''
          client_max_body_size ${cfg.maxUploadSize};
          fastcgi_buffers 64 4K;

          # Security headers
          add_header X-Content-Type-Options "nosniff" always;
          add_header X-Frame-Options "SAMEORIGIN" always;
          add_header X-XSS-Protection "1; mode=block" always;
          add_header Referrer-Policy "no-referrer" always;
          add_header Strict-Transport-Security "max-age=15552000; includeSubDomains" always;
        '';
      };
    };

    # Firewall
    networking.firewall.allowedTCPPorts = [ 80 443 ];

    # Collabora Online
    services.collabora-online = mkIf cfg.enableCollabora {
      enable = true;
      settings = {
        server_name = "collabora.${cfg.hostname}";
        ssl = {
          enable = true;
          termination = true;
        };
        storage.wopi = {
          host = cfg.hostname;
        };
      };
    };

    # Backup service (nix-evo managed)
    systemd.services."nix-evo-nextcloud-backup" = mkIf cfg.backupEnable {
      description = "nix-evo Nextcloud backup";
      script = ''
        set -eu
        TIMESTAMP=$(date +%Y%m%d_%H%M%S)
        BACKUP_DIR="/var/backup/nextcloud"
        mkdir -p "$BACKUP_DIR"

        # Maintenance mode on
        ${config.services.nextcloud.occ}/bin/nextcloud-occ maintenance:mode --on || true

        # Backup database
        ${config.services.postgresql.package}/bin/pg_dump nextcloud | \
          gzip > "$BACKUP_DIR/db_$TIMESTAMP.sql.gz"

        # Backup config and data
        tar czf "$BACKUP_DIR/data_$TIMESTAMP.tar.gz" \
          -C / ${cfg.dataDir} \
          --exclude='*/appdata_*/preview/*' 2>/dev/null || true

        # Maintenance mode off
        ${config.services.nextcloud.occ}/bin/nextcloud-occ maintenance:mode --off || true

        # Cleanup old backups (keep 7 days)
        find "$BACKUP_DIR" -name "*.gz" -mtime +7 -delete || true

        echo "Backup complete: $TIMESTAMP"
      '';
      serviceConfig = {
        Type = "oneshot";
        User = "root";
      };
    };

    systemd.timers."nix-evo-nextcloud-backup" = mkIf cfg.backupEnable {
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnCalendar = cfg.backupSchedule;
        Persistent = true;
      };
    };

    # nix-evo metadata
    environment.etc."nix-evo/services/nextcloud.json".text = builtins.toJSON {
      name = "nextcloud";
      type = "web-app";
      hostname = cfg.hostname;
      endpoints = {
        health = "https://${cfg.hostname}/status.php";
        login = "https://${cfg.hostname}/login";
      };
      services = [ "nextcloud-setup" "postgresql" "nginx" ];
      dataDir = cfg.dataDir;
      backup = cfg.backupEnable;
      nix-evo.managed = true;
    };
  };
}
