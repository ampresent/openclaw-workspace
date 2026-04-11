# Dockerfile for nix-evo-agent development/testing
# Note: Full NixOS features (nixos-rebuild, nix-store) only work on NixOS.
# This Dockerfile is for API structure testing only.
FROM rust:1.82-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libsystemd-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY evo/Cargo.toml evo/Cargo.lock* ./
# Create dummy src for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release 2>/dev/null || true
# Copy real source
COPY evo/ ./
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libsystemd0 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/nix-evo-agent /usr/local/bin/

# Create mock directories for testing
RUN mkdir -p /etc/nixos /nix/var/nix/profiles /var/lib/nix-evo

EXPOSE 7890
HEALTHCHECK --interval=30s --timeout=5s CMD curl -f http://localhost:7890/health || exit 1

ENTRYPOINT ["nix-evo-agent"]
CMD ["--host", "0.0.0.0", "--port", "7890"]
