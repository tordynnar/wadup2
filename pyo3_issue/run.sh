#!/bin/bash
# Run the PyOnceLock WASI demonstration
#
# This script runs the WASM module using wasmtime and shows the crash.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

WASM_FILE="$SCRIPT_DIR/build/pyoncelock_demo.wasm"

if [ ! -f "$WASM_FILE" ]; then
    echo "ERROR: WASM file not found. Please run ./build.sh first."
    exit 1
fi

# Check for wasmtime
if ! command -v wasmtime &> /dev/null; then
    echo "ERROR: wasmtime not found. Please install it:"
    echo "  curl https://wasmtime.dev/install.sh -sSf | bash"
    exit 1
fi

echo "============================================"
echo "Running PyOnceLock WASI Demonstration"
echo "============================================"
echo ""
echo "WASM module: $WASM_FILE"
echo "Runtime: $(wasmtime --version)"
echo ""
echo "Tests:"
echo "  - TEST 1: simple_add() - no OnceLock"
echo "  - TEST 2: std::sync::OnceLock"
echo "  - TEST 3: pyo3::sync::PyOnceLock"
echo ""
echo "============================================"
echo ""

# Run with wasmtime
# --dir=/ gives access to filesystem (needed for Python)
# The crash will show a WASM backtrace
wasmtime run \
    --dir=/ \
    --env HOME=/ \
    --env PYTHONDONTWRITEBYTECODE=1 \
    "$WASM_FILE"

echo ""
echo "All tests passed. See README.md for investigation notes."
