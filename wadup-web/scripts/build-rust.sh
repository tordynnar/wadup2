#!/bin/bash
set -e

echo "Starting Rust WASM build..."

cd /build/src

# Build for wasm32-wasip1 target
cargo build --target wasm32-wasip1 --release

# Copy the WASM file to output
cp target/wasm32-wasip1/release/*.wasm /build/output/module.wasm 2>/dev/null || {
    echo "ERROR: No .wasm file found in target directory"
    exit 1
}

echo "Build complete! WASM file written to /build/output/module.wasm"
