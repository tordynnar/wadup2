# WADUP Python Build Image
# Builds Python modules to WebAssembly using CPython + WASI SDK
#
# This image downloads and builds all WASI dependencies from source:
# - C libraries: zlib, bzip2, xz, sqlite, libxml2, libxslt
# - CPython 3.13 for WASI
# - lxml C extension
# - pydantic_core Rust extension

FROM python:3.13-slim-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    xz-utils \
    make \
    patch \
    gcc \
    g++ \
    unzip \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Download and install WASI SDK (auto-detect architecture)
ARG WASI_SDK_VERSION=24.0
RUN ARCH=$(uname -m) && \
    if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then \
        WASI_ARCH="arm64"; \
    else \
        WASI_ARCH="x86_64"; \
    fi && \
    echo "Downloading WASI SDK for $WASI_ARCH" && \
    curl -fsSL -o wasi-sdk.tar.gz "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-${WASI_SDK_VERSION}-${WASI_ARCH}-linux.tar.gz" && \
    tar xf wasi-sdk.tar.gz -C /opt && \
    rm wasi-sdk.tar.gz && \
    mv /opt/wasi-sdk-${WASI_SDK_VERSION}-${WASI_ARCH}-linux /opt/wasi-sdk

ENV WASI_SDK_PATH=/opt/wasi-sdk

# Install Rust with wasm32-wasip1 target (required for pydantic_core)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add wasm32-wasip1

# Create wadup directory structure
RUN mkdir -p /wadup/deps /wadup/guest/python

# Copy build script and patches for WASI dependencies
COPY docker/python/download-and-build-deps.sh /tmp/
COPY docker/python/patches/ /tmp/patches/
COPY docker/python/config.site-wasm32-wasi /tmp/

# Build all WASI dependencies from source
# This downloads and compiles: zlib, bzip2, xz, sqlite, libxml2, libxslt, CPython, lxml, pydantic
RUN chmod +x /tmp/download-and-build-deps.sh && \
    /tmp/download-and-build-deps.sh && \
    rm -rf /tmp/patches /tmp/*.sh /tmp/*.site*

# Create builder user
RUN useradd -m -s /bin/bash builder && \
    chown -R builder:builder /wadup

# Copy Python guest library
COPY --chown=builder:builder guest/python /wadup/guest/python

# Copy WASI stubs
COPY --chown=builder:builder docker/python/wasi-stubs /wadup/wasi-stubs

# Copy extensions registry
COPY --chown=builder:builder docker/python/extensions /wadup/extensions

# Create build directories
RUN mkdir -p /build/src /build/output && chown -R builder:builder /build

# Copy build script
COPY --chown=builder:builder docker/python/build.sh /usr/local/bin/build.sh
COPY --chown=builder:builder docker/python/build_module.py /usr/local/bin/build_module.py
RUN chmod +x /usr/local/bin/build.sh /usr/local/bin/build_module.py

USER builder
WORKDIR /build/src

# Default command runs the build script
CMD ["/usr/local/bin/build.sh"]
