# WADUP Rust Build Image
# Builds Rust modules to WebAssembly (wasm32-wasip1)

FROM rust:1.83-slim-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Add wasm32-wasip1 target
RUN rustup target add wasm32-wasip1

# Create builder user
RUN useradd -m -s /bin/bash builder

# Create wadup directory structure
RUN mkdir -p /wadup/crates/wadup-guest && chown -R builder:builder /wadup

# Copy wadup-guest crate (needed as a dependency)
COPY --chown=builder:builder wadup-guest /wadup/crates/wadup-guest

# Create build directories
RUN mkdir -p /build/src /build/output && chown -R builder:builder /build

# Copy build script
COPY --chown=builder:builder build.sh /usr/local/bin/build.sh
RUN chmod +x /usr/local/bin/build.sh

# Set up cargo cache directory
RUN mkdir -p /home/builder/.cargo && chown -R builder:builder /home/builder/.cargo

USER builder
WORKDIR /build/src

# Default command runs the build script
CMD ["/usr/local/bin/build.sh"]
