#!/bin/bash
# WADUP Python Module Build Script
set -e

echo "=== WADUP Python Build ==="
echo "Building module in /build/src"

# Check for pyproject.toml
if [ ! -f "pyproject.toml" ]; then
    echo "ERROR: No pyproject.toml found in /build/src"
    exit 1
fi

# Extract module name from pyproject.toml
MODULE_NAME=$(grep -E '^name\s*=' pyproject.toml | head -1 | sed 's/.*"\([^"]*\)".*/\1/' | tr '-' '_')
echo "Module name: $MODULE_NAME"

# Run the Python build script
python3 /usr/local/bin/build_module.py /build/src

# Show file size
if [ -f "/build/output/module.wasm" ]; then
    WASM_SIZE=$(stat -c%s /build/output/module.wasm 2>/dev/null || stat -f%z /build/output/module.wasm)
    echo "Output size: $WASM_SIZE bytes"
    echo "=== Build completed successfully ==="
else
    echo "ERROR: WASM file not created"
    exit 1
fi
