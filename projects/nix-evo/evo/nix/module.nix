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
      description = "Path to file containing API token";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.nix-evo-agent = {
      description = "nix-evo-agent — NixOS diagnostic agent";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/nix-evo-agent"
          + " --host ${cfg.host}"
          + " --port ${toString cfg.port}"
          + " --nixos-dir ${cfg.nixosDir}";
        Restart = "on-failure";
        RestartSec = 5;

        # Security hardening
        DynamicUser = true;
        SupplementaryGroups = [ "systemd-journal" ];  # for reading logs
        ProtectSystem = "strict";
        ReadWritePaths = [ "/tmp" cfg.nixosDir ];
        PrivateTmp = true;
        NoNewPrivileges = true;

        # Allow nixos-rebuild
        CapabilityBoundingSet = [ "CAP_SYS_ADMIN" "CAP_NET_ADMIN" ];
      };

      path = with pkgs; [
        nixos-rebuild
        nix
        coreutils
        gnused
        gnugrep
        diffutils
        jq
      ];
    };

    networking.firewall.allowedTCPPorts = mkIf cfg.openFirewall [ cfg.port ];
  };
}
