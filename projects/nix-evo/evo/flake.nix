# nix-evo-agent flake
{
  description = "nix-evo-agent — NixOS diagnostic agent for AI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        nix-evo-agent = pkgs.callPackage ./nix/package.nix {};
      in
      {
        packages = {
          default = nix-evo-agent;
          nix-evo-agent = nix-evo-agent;
        };

        apps.default = {
          type = "app";
          program = "${nix-evo-agent}/bin/nix-evo-agent";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ nix-evo-agent ];
          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            pkg-config
            systemd
          ];
        };

        # Formatter for `nix fmt`
        formatter = pkgs.nixpkgs-fmt;
      })
    // {
      # NixOS module (system-independent)
      nixosModules.default = import ./nix/module.nix;
      nixosModules.nix-evo = import ./nix/module.nix;
    };
}
