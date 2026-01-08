# WADUP Test Runner Image
# Executes WASM modules against sample files using the Rust wadup CLI

# Build stage - compile wadup for Linux
FROM rust:1.83-bookworm AS builder

WORKDIR /wadup

# Copy only the files needed for compilation
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the wadup binary
RUN cargo build --release -p wadup-cli

# Runtime stage - minimal image with just the binary
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create runner user
RUN useradd -m -s /bin/bash runner

# Copy the compiled binary from builder
COPY --from=builder --chown=runner:runner /wadup/target/release/wadup /usr/local/bin/wadup
RUN chmod +x /usr/local/bin/wadup

# Create directories
RUN mkdir -p /test && chown -R runner:runner /test

USER runner
WORKDIR /test

# Default entrypoint runs the test subcommand
ENTRYPOINT ["wadup", "test"]
