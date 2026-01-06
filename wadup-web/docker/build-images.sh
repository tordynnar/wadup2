#!/bin/bash
# Build all WADUP Docker images
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building WADUP Docker Images ==="

# Build Rust image
echo ""
echo "Building wadup-build-rust:latest..."
docker build -t wadup-build-rust:latest ./rust

# Build Go image
echo ""
echo "Building wadup-build-go:latest..."
docker build -t wadup-build-go:latest ./go

# Build Python image
echo ""
echo "Building wadup-build-python:latest..."
docker build -t wadup-build-python:latest ./python

echo ""
echo "=== All images built successfully ==="
docker images | grep wadup-build
