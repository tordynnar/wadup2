# WADUP Python Build Image
# Builds Python modules to WebAssembly using componentize-py

FROM python:3.12-slim-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install componentize-py
RUN pip install --no-cache-dir componentize-py==0.16.0

# Create builder user
RUN useradd -m -s /bin/bash builder

# Create wadup directory structure
RUN mkdir -p /wadup/guest/python && chown -R builder:builder /wadup

# Copy Python guest library (stub wadup module)
COPY --chown=builder:builder wadup-guest-python /wadup/guest/python

# Create build directories
RUN mkdir -p /build/src /build/output && chown -R builder:builder /build

# Copy build script
COPY --chown=builder:builder build.sh /usr/local/bin/build.sh
RUN chmod +x /usr/local/bin/build.sh

USER builder
WORKDIR /build/src

# Add wadup guest library to PYTHONPATH
ENV PYTHONPATH=/wadup/guest/python

# Default command runs the build script
CMD ["/usr/local/bin/build.sh"]
