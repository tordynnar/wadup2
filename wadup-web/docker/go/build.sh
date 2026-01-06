#!/bin/bash
# WADUP Go Module Build Script
# Uses standard Go with wasip1 target
set -e

echo "=== WADUP Go Build ==="
echo "Building module in /build/src"

# Check for go.mod
if [ ! -f "go.mod" ]; then
    echo "ERROR: No go.mod found in /build/src"
    exit 1
fi

# Download dependencies
echo "Downloading dependencies..."
go mod download || true

# Build for wasip1 using standard Go
echo "Compiling to wasip1 with Go..."
GOOS=wasip1 GOARCH=wasm go build -o /build/output/module.wasm .

# Show file size
if [ -f "/build/output/module.wasm" ]; then
    WASM_SIZE=$(stat -c%s /build/output/module.wasm 2>/dev/null || stat -f%z /build/output/module.wasm)
    echo "Output size: $WASM_SIZE bytes"
    echo "=== Build completed successfully ==="
else
    echo "ERROR: WASM file not created"
    exit 1
fi
