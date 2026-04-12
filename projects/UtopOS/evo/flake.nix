# UtopOS-agent flake
{
  description = "UtopOS-agent — NixOS diagnostic agent for AI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        UtopOS-agent = pkgs.callPackage ./nix/package.nix {};
      in
      {
        packages = {
          default = UtopOS-agent;
          UtopOS-agent = UtopOS-agent;
        };

        apps.default = {
          type = "app";
          program = "${UtopOS-agent}/bin/UtopOS-agent";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ UtopOS-agent ];
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
      nixosModules.UtopOS = import ./nix/module.nix;
    };
}
