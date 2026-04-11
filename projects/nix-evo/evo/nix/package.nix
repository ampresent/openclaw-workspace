{ lib, rustPlatform, pkg-config, systemd }:

rustPlatform.buildRustPackage {
  pname = "nix-evo-agent";
  version = "0.1.0";
  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ systemd ];

  meta = with lib; {
    description = "NixOS diagnostic and management agent for AI agents";
    license = licenses.mit;
  };
}
