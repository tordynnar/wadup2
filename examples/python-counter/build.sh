#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "===================================================="
echo "Building Python SQLite Parser WASM Module"
echo "===================================================="
echo ""

# Step 1: Build CPython for WASI if needed
echo "[1/2] Checking CPython WASI build..."
../../scripts/build-python-wasi.sh

echo ""

# Step 2: Build the WASM module using Make
echo "[2/2] Building WASM module..."
make clean
make all

echo ""
echo "===================================================="
echo "Build Complete!"
echo "===================================================="

WASM_FILE="target/python_sqlite_parser.wasm"
if [ -f "$WASM_FILE" ]; then
    SIZE=$(ls -lh "$WASM_FILE" | awk '{print $5}')
    echo "✓ WASM module: $WASM_FILE ($SIZE)"

    # Verify process() export
    if command -v wasm-objdump &> /dev/null; then
        echo ""
        echo "Exports:"
        wasm-objdump -x "$WASM_FILE" | grep "export" | head -5
    fi
else
    echo "✗ Build failed - WASM file not found"
    exit 1
fi

echo ""
