# Jellyfin NixOS Module for nix-evo
#
# Managed Jellyfin media server with hardware transcoding, Nginx reverse proxy,
# and optional integration with other nix-evo services.
#
# Usage in configuration.nix:
#   imports = [ ./modules/jellyfin.nix ];
#   services.nix-evo.jellyfin.enable = true;

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.nix-evo.jellyfin;
in {
  options.services.nix-evo.jellyfin = {
    enable = mkEnableOption "nix-evo managed Jellyfin media server";

    hostname = mkOption {
      type = types.str;
      default = "media.example.com";
      description = "Domain name for Jellyfin web interface";
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/jellyfin";
      description = "Jellyfin data/config directory";
    };

    mediaDirs = mkOption {
      type = types.listOf types.path;
      default = [ "/media/movies" "/media/tv" "/media/music" ];
      description = "Media library directories";
      example = [ "/mnt/nas/movies" "/mnt/nas/tv" ];
    };

    enableHardwareTranscoding = mkOption {
      type = types.bool;
      default = false;
      description = "Enable hardware-accelerated transcoding (VA-API/VDPAU)";
    };

    gpuDevice = mkOption {
      type = types.str;
      default = "/dev/dri/renderD128";
      description = "GPU device path for hardware transcoding";
    };

    enableLiveTv = mkOption {
      type = types.bool;
      default = false;
      description = "Enable Live TV / DVR functionality";
    };

    port = mkOption {
      type = types.port;
      default = 8096;
      description = "Jellyfin HTTP port (internal, proxied by Nginx)";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open firewall for Jellyfin port directly (not needed when using Nginx)";
    };

    extraEnv = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = "Extra environment variables for Jellyfin";
    };

    enableAutoScan = mkOption {
      type = types.bool;
      default = true;
      description = "Enable periodic media library scans";
    };

    scanSchedule = mkOption {
      type = types.str;
      default = "hourly";
      description = "Media scan schedule (systemd calendar format)";
    };
  };

  config = mkIf cfg.enable {
    services.jellyfin = {
      enable = true;
      dataDir = cfg.dataDir;
      group = "media";
      user = "jellyfin";
    };

    # User/group for media access
    users.groups.media = {};
    users.users.jellyfin = {
      isSystemUser = true;
      group = "media";
      extraGroups = [ "video" "render" ]; # for GPU access
    };

    # Grant Jellyfin access to media directories
    systemd.services.jellyfin.serviceConfig = {
      SupplementaryGroups = mkIf cfg.enableHardwareTranscoding [ "video" "render" ];
      DeviceAllow = mkIf cfg.enableHardwareTranscoding [
        "char-renderD rw"
        "char-card rw"
      ];
    };

    # Hardware transcoding
    hardware.opengl = mkIf cfg.enableHardwareTranscoding {
      enable = true;
      driSupport = true;
    };

    environment.sessionVariables = mkIf cfg.enableHardwareTranscoding {
      LIBVA_DRIVER_NAME = "iHD"; # Intel; change to "radeonsi" for AMD
    };

    # Nginx reverse proxy
    services.nginx = {
      enable = true;
      virtualHosts.${cfg.hostname} = {
        forceSSL = true;
        enableACME = true;
        extraConfig = ''
          # Jellyfin requires large buffers for video streaming
          proxy_buffering off;
          proxy_max_temp_file_size 0;

          # WebSocket support for Jellyfin notifications
          proxy_set_header Upgrade $http_upgrade;
          proxy_set_header Connection $http_connection;
        '';
        locations."/" = {
          proxyPass = "http://127.0.0.1:${toString cfg.port}";
          extraConfig = ''
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
          '';
        };
        # Direct streaming endpoint (bypasses some buffering)
        locations."=/socket" = {
          proxyPass = "http://127.0.0.1:${toString cfg.port}/socket";
          extraConfig = ''
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
          '';
        };
      };
    };

    # Firewall
    networking.firewall.allowedTCPPorts = [ 80 443 ]
      ++ (optional cfg.openFirewall cfg.port);

    # File permissions for media directories
    systemd.tmpfiles.rules = map (dir:
      "d ${dir} 0775 jellyfin media - -"
    ) cfg.mediaDirs;

    # Periodic library scan
    systemd.services."jellyfin-scan" = mkIf cfg.enableAutoScan {
      description = "Jellyfin media library scan";
      script = ''
        # Trigger a library scan via Jellyfin API
        curl -s -X POST \
          "http://127.0.0.1:${toString cfg.port}/Library/Refresh" \
          -H "Authorization: MediaBrowser Token=$(cat ${cfg.dataDir}/api-token 2>/dev/null || echo '')" \
          || echo "Scan trigger failed (no API token or server not ready)"
      '';
      serviceConfig = {
        Type = "oneshot";
        User = "jellyfin";
      };
    };

    systemd.timers."jellyfin-scan" = mkIf cfg.enableAutoScan {
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnCalendar = cfg.scanSchedule;
        Persistent = true;
      };
    };

    # nix-evo metadata
    environment.etc."nix-evo/services/jellyfin.json".text = builtins.toJSON {
      name = "jellyfin";
      type = "media-server";
      hostname = cfg.hostname;
      port = cfg.port;
      endpoints = {
        health = "https://${cfg.hostname}/health";
        web = "https://${cfg.hostname}";
        api = "https://${cfg.hostname}/System/Info/Public";
      };
      services = [ "jellyfin" "nginx" ];
      dataDir = cfg.dataDir;
      mediaDirs = cfg.mediaDirs;
      features = {
        hardwareTranscoding = cfg.enableHardwareTranscoding;
        liveTv = cfg.enableLiveTv;
      };
      nix-evo.managed = true;
    };
  };
}
