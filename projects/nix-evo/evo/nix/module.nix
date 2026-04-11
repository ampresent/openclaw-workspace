# nix-evo-agent NixOS module
# Add to your configuration.nix or as a flake input

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.nix-evo-agent;
in
{
  options.services.nix-evo-agent = {
    enable = mkEnableOption "nix-evo-agent — NixOS diagnostic agent for AI";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ./package.nix {};
      description = "nix-evo-agent package";
    };

    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Bind address (127.0.0.1 for local only, 0.0.0.0 for all interfaces)";
    };

    port = mkOption {
      type = types.port;
      default = 7890;
      description = "Bind port";
    };

    nixosDir = mkOption {
      type = types.str;
      default = "/etc/nixos";
      description = "NixOS configuration directory";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open firewall port (not recommended; use SSH tunnel instead)";
    };

    tokenFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to file containing API token (one line, no trailing newline)";
    };

    maxLogLines = mkOption {
      type = types.int;
      default = 200;
      description = "Maximum log lines to return per request";
    };

    logLevel = mkOption {
      type = types.enum [ "trace" "debug" "info" "warn" "error" ];
      default = "info";
      description = "Logging level";
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Extra CLI arguments passed to nix-evo-agent";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.nix-evo-agent = {
      description = "nix-evo-agent — NixOS diagnostic agent for AI";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        RUST_LOG = "nix_evo_agent=${cfg.logLevel},tower_http=${cfg.logLevel}";
      };

      serviceConfig = {
        ExecStart = concatStringsSep " " ([
          "${cfg.package}/bin/nix-evo-agent"
          "--host ${cfg.host}"
          "--port ${toString cfg.port}"
          "--nixos-dir ${cfg.nixosDir}"
          "--max-log-lines ${toString cfg.maxLogLines}"
        ] ++ optional (cfg.tokenFile != null) "--api-token $(cat ${cfg.tokenFile})"
          ++ cfg.extraArgs);

        Restart = "on-failure";
        RestartSec = 5;
        StartLimitIntervalSec = 60;
        StartLimitBurst = 5;

        # Security hardening
        DynamicUser = true;
        SupplementaryGroups = [ "systemd-journal" ];
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/tmp" cfg.nixosDir "/nix/var/nix/profiles" "/var/lib/nix-evo" ];
        PrivateTmp = true;
        NoNewPrivileges = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        MemoryDenyWriteExecute = true;
        LockPersonality = true;
        SystemCallFilter = [ "@system-service" "~@privileged" ];
        CapabilityBoundingSet = [ "" ];
        AmbientCapabilities = [ "" ];
      };

      path = with pkgs; [
        nixos-rebuild
        nix
        coreutils
        gnused
        gnugrep
        diffutils
        jq
        hostname
        util-linux
        rsync
        findutils
      ];
    };

    networking.firewall.allowedTCPPorts = mkIf cfg.openFirewall [ cfg.port ];
  };
}
