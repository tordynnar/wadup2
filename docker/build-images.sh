#!/bin/bash
# Build all WADUP Docker images
# Must be run from the project root directory
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== Building WADUP Docker Images ==="
echo "Building from: $PROJECT_ROOT"

# Build Rust image
echo ""
echo "Building wadup-build-rust:latest..."
docker build -t wadup-build-rust:latest -f docker/rust/Dockerfile .

# Build Go image
echo ""
echo "Building wadup-build-go:latest..."
docker build -t wadup-build-go:latest -f docker/go/Dockerfile .

# Build Python image
echo ""
echo "Building wadup-build-python:latest..."
docker build -t wadup-build-python:latest -f docker/python/Dockerfile .

# Build Test Runner image
echo ""
echo "Building wadup-test-runner:latest..."
docker build -t wadup-test-runner:latest -f docker/test/Dockerfile .

echo ""
echo "=== All images built successfully ==="
docker images | grep -E 'wadup-build|wadup-test'
