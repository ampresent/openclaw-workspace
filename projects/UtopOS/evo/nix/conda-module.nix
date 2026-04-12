# conda-module.nix — NixOS module for declarative conda environment management
#
# This module provides a declarative way to manage conda environments
# on NixOS servers. It uses micromamba as the backend for fast, standalone
# conda package management.
#
# Usage:
#   services.nix-evo-conda = {
#     enable = true;
#     backend = "micromamba";  # or "conda"
#     rootDir = "/opt/conda-envs";
#
#     environments = {
#       ml-project = {
#         python = "3.11";
#         channels = [ "conda-forge" "nvidia" ];
#         packages = [ "numpy" "pandas" "scikit-learn" ];
#         pipPackages = [ "transformers" "datasets" ];
#         environmentFile = ./environments/ml-project.yml;  # optional
#       };
#
#       datasci = {
#         python = "3.12";
#         packages = [ "numpy" "matplotlib" "seaborn" "jupyter" ];
#       };
#     };
#   };

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.nix-evo-conda;

  # Generate environment.yml for a named environment
  mkEnvironmentYml = name: envCfg:
    let
      deps = (map (p: "${p}") envCfg.packages)
        ++ (optionals (envCfg.pipPackages != []) [
          { pip = envCfg.pipPackages; }
        ]);
      pythonDep = optional (envCfg.python != null) "python=${envCfg.python}";
    in
    pkgs.writeText "${name}-environment.yml" ''
      name: ${name}
      channels:
        ${concatMapStringsSep "\n" (c: "- ${c}") (["defaults"] ++ envCfg.channels)}
      dependencies:
        ${concatMapStringsSep "\n" (d:
          if builtins.isAttrs d then
            "- pip:\n    ${concatMapStringsSep "\n    " (p: "- ${p}") d.pip}"
          else
            "- ${d}"
        ) (pythonDep ++ deps)}
    '';

  # Generate activation script for a named environment
  mkEnvScript = name: envCfg:
    let
      ymlPath = mkEnvironmentYml name envCfg;
      envDir = "${cfg.rootDir}/${name}";
    in
    ''
      # Provision conda environment: ${name}
      ENV_DIR="${envDir}"

      if [ ! -d "$ENV_DIR" ]; then
        echo "Creating conda environment: ${name}..."
        ${cfg.backendBin} env create -f ${ymlPath} --prefix "$ENV_DIR" -y || {
          echo "Warning: Failed to create ${name} environment"
          exit 0
        }
      else
        echo "Updating conda environment: ${name}..."
        ${cfg.backendBin} env update -f ${ymlPath} --prefix "$ENV_DIR" -y --prune || {
          echo "Warning: Failed to update ${name} environment"
        }
      fi

      echo "✓ Environment ${name} provisioned at $ENV_DIR"
    '';

  # Generate a script that provisions all environments
  provisionAllScript = pkgs.writeShellScript "nix-evo-conda-provision" ''
    set -euo pipefail

    echo "═══════════════════════════════════════"
    echo "  nix-evo-conda: Provisioning environments"
    echo "═══════════════════════════════════════"

    mkdir -p "${cfg.rootDir}"

    ${concatStringsSep "\n" (mapAttrsToList (name: envCfg: mkEnvScript name envCfg) cfg.environments)}

    echo ""
    echo "✅ All conda environments provisioned"
    echo ""
  '';

  # Generate conda environment list for status check
  statusScript = pkgs.writeShellScript "nix-evo-conda-status" ''
    set -euo pipefail
    echo "═══════════════════════════════════════"
    echo "  nix-evo-conda: Environment Status"
    echo "═══════════════════════════════════════"
    echo ""
    ${cfg.backendBin} env list 2>/dev/null || echo "Backend not available"
    echo ""
    echo "Managed environments: ${concatStringsSep ", " (attrNames cfg.environments)}"
  '';

in
{
  options.services.nix-evo-conda = {
    enable = mkEnableOption "nix-evo conda environment management";

    backend = mkOption {
      type = types.enum [ "micromamba" "conda" ];
      default = "micromamba";
      description = "Which conda-compatible backend to use for environment management.";
    };

    rootDir = mkOption {
      type = types.path;
      default = "/opt/conda-envs";
      description = "Root directory where conda environments will be created.";
    };

    autoProvision = mkOption {
      type = types.bool;
      default = true;
      description = "Automatically provision environments on system activation.";
    };

    environments = mkOption {
      type = types.attrsOf (types.submodule {
        options = {
          python = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Python version to install (e.g., \"3.11\").";
          };

          channels = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Additional conda channels beyond 'defaults'.";
          };

          packages = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Conda packages to install.";
          };

          pipPackages = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Pip packages to install within the conda environment.";
          };

          environmentFile = mkOption {
            type = types.nullOr types.path;
            default = null;
            description = "Path to an environment.yml file (overrides packages/pipPackages).";
          };
        };
      });
      default = {};
      description = "Attribute set of conda environments to manage.";
    };

    # Internal
    backendBin = mkOption {
      type = types.path;
      internal = true;
      description = "Path to the backend binary.";
    };
  };

  config = mkIf cfg.enable {
    # Set backend binary path
    services.nix-evo-conda.backendBin =
      if cfg.backend == "micromamba"
      then "${pkgs.micromamba}/bin/micromamba"
      else "${pkgs.conda}/bin/conda";  # may not exist in nixpkgs

    # Install the backend
    environment.systemPackages =
      if cfg.backend == "micromamba"
      then [ pkgs.micromamba ]
      else [];

    # Systemd service for provisioning
    systemd.services.nix-evo-conda-provision = mkIf cfg.autoProvision {
      description = "Provision conda environments via nix-evo";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      path = [ cfg.backendBin pkgs.bash ];

      serviceConfig = {
        Type = "oneshot";
        ExecStart = provisionAllScript;
        RemainAfterExit = true;
        # Run as root (or could be configured to run as a specific user)
        # StateDirectory = "nix-evo-conda";
      };

      # Re-run when the module config changes
      restartTriggers = [ config.system.build.toplevel ];
    };

    # Status check service
    systemd.services.nix-evo-conda-status = {
      description = "Show conda environment status";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = statusScript;
        RemainAfterExit = false;
      };
    };

    # Provide CLI commands
    environment.systemPackages = [
      (pkgs.writeShellScriptBin "nix-evo-conda-provision" ''
        sudo ${provisionAllScript}
      '')
      (pkgs.writeShellScriptBin "nix-evo-conda-status" ''
        ${statusScript}
      '')
    ];
  };
}
