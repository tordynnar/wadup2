# WADUP Python Build Image
# Builds Python modules to WebAssembly using CPython + WASI SDK

FROM python:3.13-slim-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    xz-utils \
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

# Create builder user
RUN useradd -m -s /bin/bash builder

# Create wadup directory structure
RUN mkdir -p /wadup/deps /wadup/guest/python && chown -R builder:builder /wadup

# Copy pre-built WASI libraries (from deps/ directory)
# Built with: docker build -f docker/python/Dockerfile .
# These are cross-platform since they're wasm32-wasi object files
COPY --chown=builder:builder docker/python/wasi-deps/wasi-python /wadup/deps/wasi-python
COPY --chown=builder:builder docker/python/wasi-deps/wasi-zlib /wadup/deps/wasi-zlib
COPY --chown=builder:builder docker/python/wasi-deps/wasi-bzip2 /wadup/deps/wasi-bzip2
COPY --chown=builder:builder docker/python/wasi-deps/wasi-xz /wadup/deps/wasi-xz
COPY --chown=builder:builder docker/python/wasi-deps/wasi-sqlite /wadup/deps/wasi-sqlite
COPY --chown=builder:builder docker/python/wasi-deps/wasi-lxml /wadup/deps/wasi-lxml
COPY --chown=builder:builder docker/python/wasi-deps/wasi-libxml2 /wadup/deps/wasi-libxml2
COPY --chown=builder:builder docker/python/wasi-deps/wasi-libxslt /wadup/deps/wasi-libxslt
COPY --chown=builder:builder docker/python/wasi-deps/wasi-pydantic /wadup/deps/wasi-pydantic

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
