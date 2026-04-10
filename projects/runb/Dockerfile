# Build stage: use official Rust on Alpine
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/

RUN cargo build --release --target x86_64-unknown-linux-musl

# Test stage: minimal Alpine with the binary + test fixtures
FROM alpine:3.21

RUN apk add --no-cache bash

# Copy the built binary
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/runb /usr/local/bin/runb

# Create a minimal Alpine rootfs for testing
RUN mkdir -p /test-rootfs/bin /test-rootfs/usr/bin /test-rootfs/proc /test-rootfs/dev && \
    cp /bin/sh /test-rootfs/bin/sh && \
    cp /bin/busybox /test-rootfs/bin/busybox && \
    cd /test-rootfs/bin && ln -sf busybox cat && ln -sf busybox echo && \
    ln -sf busybox ls && ln -sf busybox pwd && ln -sf busybox env && \
    ln -sf busybox mkdir && ln -sf busybox true && ln -sf busybox sleep

# Test script
COPY tests/run-tests.sh /tests/run-tests.sh
RUN chmod +x /tests/run-tests.sh

ENTRYPOINT ["/tests/run-tests.sh"]
