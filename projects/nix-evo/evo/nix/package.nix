{ lib, rustPlatform, pkg-config, systemd }:

rustPlatform.buildRustPackage {
  pname = "nix-evo-agent";
  version = "0.3.1";
  src = lib.cleanSource ../.;

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ systemd ];

  # Skip tests that need NixOS-specific environment
  doCheck = false;

  meta = with lib; {
    description = "NixOS diagnostic and management agent for AI agents";
    homepage = "https://github.com/ampresent/nix-evo";
    license = licenses.mit;
    platforms = platforms.linux;
  };
}
