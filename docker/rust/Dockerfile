# WADUP Rust Build Image
# Builds Rust modules to WebAssembly (wasm32-wasip1)

FROM rust:1.83-slim-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Add wasm32-wasip1 target
RUN rustup target add wasm32-wasip1

# Install WASI SDK for compiling C code (needed for rusqlite bundled feature)
ARG WASI_SDK_VERSION=24.0
ARG TARGETARCH
RUN ARCH=$(case "$TARGETARCH" in "arm64") echo "arm64" ;; *) echo "x86_64" ;; esac) && \
    curl -fsSL "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_SDK_VERSION%%.*}/wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-linux.tar.gz" | \
    tar xz -C /opt && \
    mv /opt/wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-linux /opt/wasi-sdk

# Set WASI SDK environment variables
ENV WASI_SDK_PATH=/opt/wasi-sdk
ENV CC_wasm32_wasip1=/opt/wasi-sdk/bin/clang
ENV CFLAGS_wasm32_wasip1="--sysroot=/opt/wasi-sdk/share/wasi-sysroot"

# SQLite compile flags for WASI (disable threading)
ENV LIBSQLITE3_FLAGS="-DSQLITE_THREADSAFE=0"

# Create builder user
RUN useradd -m -s /bin/bash builder

# Create wadup directory structure
RUN mkdir -p /wadup/crates/wadup-guest && chown -R builder:builder /wadup

# Copy wadup-guest crate (needed as a dependency)
# Built with: docker build -f docker/rust/Dockerfile .
COPY --chown=builder:builder crates/wadup-guest /wadup/crates/wadup-guest

# Create build directories
RUN mkdir -p /build/src /build/output && chown -R builder:builder /build

# Copy build script
COPY --chown=builder:builder docker/rust/build.sh /usr/local/bin/build.sh
RUN chmod +x /usr/local/bin/build.sh

# Set up cargo cache directory
RUN mkdir -p /home/builder/.cargo && chown -R builder:builder /home/builder/.cargo

USER builder
WORKDIR /build/src

# Default command runs the build script
CMD ["/usr/local/bin/build.sh"]
