# Experiment: Nix flakes wrapping conda environments
#
# Question: Can we use Nix to pin micromamba itself, then micromamba to pin Python packages?
# Answer: YES — this is a powerful hybrid approach.
#
# The flake pins micromamba (and conda-lock) as Nix packages, ensuring the toolchain
# is reproducible at the OS level. micromamba then manages the Python packages,
# which Nix struggles with (especially for ML/CUDA packages).
#
# Usage:
#   nix develop   → get a shell with micromamba + pre-configured env
#   nix run       → provision the conda environment
#   nix build     → (experimental) build a derivation wrapping the env

{
  description = "Nix-managed conda environments via micromamba";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Pin micromamba via Nix — this is the key insight
        micromamba = pkgs.micromamba;

        # Our conda-lock version
        conda-lock = pkgs.conda-lock or null;

        # The environment.yml for our test environment
        environmentYml = pkgs.writeText "environment.yml" ''
          name: nix-conda-test
          channels:
            - defaults
            - conda-forge
          dependencies:
            - python=3.11
            - numpy>=1.24
            - pandas
            - scipy
            - pip:
              - tqdm
        '';

        # Shell hook that sets up micromamba and the conda env
        shellHook = ''
          export MAMBA_ROOT_PREFIX="''${PWD}/.mamba-root"
          export CONDA_ENVS_PATH="''${PWD}/.conda-envs"

          # Create envs directory
          mkdir -p "$CONDA_ENVS_PATH"

          # Auto-provision if env doesn't exist
          if [ ! -d "$CONDA_ENVS_PATH/nix-conda-test" ]; then
            echo "🔧 Provisioning conda environment from Nix-pinned micromamba..."
            micromamba env create -f ${environmentYml} \
              --prefix "$CONDA_ENVS_PATH/nix-conda-test" \
              -y 2>/dev/null || echo "⚠️  Environment creation failed (no network?)"
          fi

          # Activate
          eval "$(micromamba shell hook --shell bash)"
          if [ -d "$CONDA_ENVS_PATH/nix-conda-test" ]; then
            micromamba activate "$CONDA_ENVS_PATH/nix-conda-test" 2>/dev/null || true
          fi

          echo ""
          echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
          echo "  🧪 Nix+Conda Hybrid Environment"
          echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
          echo "  Nix manages: micromamba, system tools"
          echo "  conda manages: Python, numpy, pandas, etc."
          echo ""
          echo "  micromamba: $(micromamba --version 2>/dev/null || echo 'not available')"
          echo "  python:     $(python3 --version 2>/dev/null || echo 'not available')"
          echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
          echo ""
        '';

      in
      {
        # Development shell with micromamba
        devShells.default = pkgs.mkShell {
          name = "nix-conda-hybrid";

          buildInputs = [
            micromamba
            pkgs.git
            pkgs.curl
          ] ++ (if conda-lock != null then [ conda-lock ] else []);

          inherit shellHook;

          # Environment variables
          MAMBA_ROOT_PREFIX = ".mamba-root";
          MAMBA_ALWAYS_YES = "true";
        };

        # Experimental: run script that provisions the env
        apps.default = {
          type = "app";
          program = toString (pkgs.writeShellScript "provision-conda-env" ''
            set -euo pipefail
            export MAMBA_ROOT_PREFIX="''${PWD}/.mamba-root"
            export MAMBA_ALWAYS_YES="true"

            echo "Provisioning conda environment..."
            micromamba env create -f ${environmentYml} \
              --prefix "./conda-env" -y

            echo "✅ Environment provisioned at ./conda-env"
            echo "Activate with: micromamba activate ./conda-env"
          '');
        };

        # Package: a wrapper that runs Python from the conda env
        # (experimental — this is where it gets interesting)
        packages.default = pkgs.stdenv.mkDerivation {
          name = "nix-conda-python";
          src = ./.;

          buildInputs = [ micromamba ];

          installPhase = ''
            mkdir -p $out/bin
            cat > $out/bin/python-from-conda << 'SCRIPT'
            #!/usr/bin/env bash
            export MAMBA_ROOT_PREFIX="''${NIX_CONDA_ROOT:-/tmp/mamba-root}"
            eval "$(micromamba shell hook --shell bash)"
            micromamba activate "''${NIX_CONDA_ENV:-$MAMBA_ROOT_PREFIX/envs/default}" 2>/dev/null
            exec python3 "$@"
            SCRIPT
            chmod +x $out/bin/python-from-conda
          '';
        };

        # Checks
        checks.micromamba-available = pkgs.runCommand "check-micromamba" {} ''
          ${micromamba}/bin/micromamba --version
          echo "micromamba is available" > $out
        '';
      }
    );
}
