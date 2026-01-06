#!/bin/bash
# WADUP Rust Module Build Script
set -e

echo "=== WADUP Rust Build ==="
echo "Building module from /build/src"

# Check for Cargo.toml
if [ ! -f "Cargo.toml" ]; then
    echo "ERROR: No Cargo.toml found in /build/src"
    exit 1
fi

# Copy source to a writable location (source is mounted read-only)
echo "Copying source to build directory..."
cp -r /build/src /tmp/build-workspace
cd /tmp/build-workspace

# Extract package name from Cargo.toml
PACKAGE_NAME=$(grep -E '^name\s*=' Cargo.toml | head -1 | sed 's/.*"\([^"]*\)".*/\1/' | tr '-' '_')
echo "Package name: $PACKAGE_NAME"

# Build for wasm32-wasip1
echo "Compiling to wasm32-wasip1..."
cargo build --release --target wasm32-wasip1

# Find the output WASM file
WASM_FILE="target/wasm32-wasip1/release/${PACKAGE_NAME}.wasm"

if [ ! -f "$WASM_FILE" ]; then
    echo "ERROR: WASM file not found at $WASM_FILE"
    echo "Available files in target/wasm32-wasip1/release:"
    ls -la target/wasm32-wasip1/release/*.wasm 2>/dev/null || echo "No .wasm files found"
    exit 1
fi

# Copy to output directory
echo "Copying $WASM_FILE to /build/output/module.wasm"
cp "$WASM_FILE" /build/output/module.wasm

# Show file size
WASM_SIZE=$(stat -c%s /build/output/module.wasm 2>/dev/null || stat -f%z /build/output/module.wasm)
echo "Output size: $WASM_SIZE bytes"

echo "=== Build completed successfully ==="
