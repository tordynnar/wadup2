# WADUP Go Build Image
# Builds Go modules to WebAssembly using standard Go

FROM golang:1.24-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create builder user
RUN useradd -m -s /bin/bash builder

# Create wadup directory structure for Go guest library
RUN mkdir -p /wadup/guest/go && chown -R builder:builder /wadup

# Copy Go guest library
# Built with: docker build -f docker/go/Dockerfile .
COPY --chown=builder:builder guest/go /wadup/guest/go

# Create build directories
RUN mkdir -p /build/src /build/output && chown -R builder:builder /build

# Copy build script
COPY --chown=builder:builder docker/go/build.sh /usr/local/bin/build.sh
RUN chmod +x /usr/local/bin/build.sh

# Set up Go cache directory
RUN mkdir -p /home/builder/go && chown -R builder:builder /home/builder/go
ENV GOPATH=/home/builder/go
ENV GOCACHE=/home/builder/.cache/go-build

USER builder
WORKDIR /build/src

# Default command runs the build script
CMD ["/usr/local/bin/build.sh"]
