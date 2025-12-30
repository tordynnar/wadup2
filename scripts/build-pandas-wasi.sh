#!/bin/bash
# Build Pandas C extensions for WASI
# This script compiles Pandas core modules using WASI cross-compilation
#
# IMPORTANT: This is a placeholder script. Full Pandas WASI compilation requires:
#   1. NumPy to be built first (pandas depends on numpy)
#   2. Proper Meson cross-compilation setup
#   3. Cython to generate C files from .pyx sources
#   4. Patches for WASI compatibility
#
# For reference implementations, see:
#   - Pyodide: https://github.com/pyodide/pyodide/tree/main/packages/pandas
#   - wasi-wheels: https://github.com/dicej/wasi-wheels

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"
BUILD_DIR="$WADUP_ROOT/build"

# Versions
PANDAS_VERSION="2.2.3"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" = "darwin" ]; then
    WASI_SDK_OS="macos"
elif [ "$OS" = "linux" ]; then
    WASI_SDK_OS="linux"
else
    echo "ERROR: Unsupported OS: $OS"
    exit 1
fi

WASI_SDK_VERSION="29.0"
WASI_SDK_PATH="$DEPS_DIR/wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"

echo "=== Building Pandas ${PANDAS_VERSION} for WASI ==="

# Check dependencies
if [ ! -f "$BUILD_DIR/python-wasi/lib/libpython3.13.a" ]; then
    echo "ERROR: Python WASI not built. Run ./scripts/build-python-wasi.sh first"
    exit 1
fi

if [ ! -d "$WASI_SDK_PATH" ]; then
    echo "ERROR: WASI SDK not found. Run ./scripts/download-deps.sh first"
    exit 1
fi

# Note: Pandas depends on NumPy - for full build this would be required
# if [ ! -f "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" ]; then
#     echo "ERROR: NumPy WASI not built. Run ./scripts/build-numpy-wasi.sh first"
#     exit 1
# fi

# Check if already built
if [ -f "$DEPS_DIR/wasi-pandas/lib/libpandas_libs.a" ]; then
    echo "Pandas already built"
    exit 0
fi

# Create directories
mkdir -p "$DEPS_DIR/wasi-pandas/lib"
mkdir -p "$DEPS_DIR/wasi-pandas/python/pandas"

# Download Pandas source if needed
PANDAS_ARCHIVE="$DEPS_DIR/pandas-${PANDAS_VERSION}.tar.gz"
if [ ! -f "$PANDAS_ARCHIVE" ]; then
    echo "Downloading Pandas ${PANDAS_VERSION}..."
    curl -L -o "$PANDAS_ARCHIVE" "https://files.pythonhosted.org/packages/source/p/pandas/pandas-${PANDAS_VERSION}.tar.gz"
fi

# Extract
echo "Extracting..."
cd "$DEPS_DIR"
rm -rf "pandas-${PANDAS_VERSION}"
tar xzf "pandas-${PANDAS_VERSION}.tar.gz"
cd "pandas-${PANDAS_VERSION}"

# Setup compiler
CC="$WASI_SDK_PATH/bin/clang"
AR="$WASI_SDK_PATH/bin/ar"

# Include paths
PYTHON_INCLUDE="$BUILD_DIR/python-wasi/include"

# CFLAGS for WASI cross-compilation
CFLAGS="-O2 -fPIC"
CFLAGS="$CFLAGS -I$PYTHON_INCLUDE"
CFLAGS="$CFLAGS -DCYTHON_PEP489_MULTI_PHASE_INIT=0"
CFLAGS="$CFLAGS -D__wasi__=1"

echo "Pandas build requires Meson and complex cross-compilation setup."
echo "For initial implementation, we'll copy pure Python files."
echo ""

# Copy pure Python files from the source
echo "Copying Python files..."
cp -r pandas "$DEPS_DIR/wasi-pandas/python/"

# Remove compiled files if any
find "$DEPS_DIR/wasi-pandas/python/pandas" -name "*.so" -delete 2>/dev/null || true
find "$DEPS_DIR/wasi-pandas/python/pandas" -name "*.pyc" -delete 2>/dev/null || true

# Create placeholder library
echo "Creating placeholder library..."
echo "/* Pandas WASI build placeholder */" > /tmp/pandas_placeholder.c
$CC $CFLAGS -c /tmp/pandas_placeholder.c -o /tmp/pandas_placeholder.o 2>/dev/null || true
$AR rcs "$DEPS_DIR/wasi-pandas/lib/libpandas_libs.a" /tmp/pandas_placeholder.o 2>/dev/null || touch "$DEPS_DIR/wasi-pandas/lib/libpandas_libs.a"

# Clean up
cd "$DEPS_DIR"
rm -rf "pandas-${PANDAS_VERSION}"

echo ""
echo "=== Pandas partial build complete ==="
echo "NOTE: Full Pandas WASI compilation is complex and requires:"
echo "  1. NumPy WASI to be built first"
echo "  2. Proper Meson cross-compilation setup"
echo "  3. Cython to generate C from .pyx files"
echo "  4. Static compilation of all C extensions"
echo ""
echo "Pure Python files have been copied to: $DEPS_DIR/wasi-pandas/python/pandas/"
echo "Placeholder library created in: $DEPS_DIR/wasi-pandas/lib/"
echo ""
echo "For full Pandas support, consider:"
echo "  - Using Pyodide's patches and build scripts"
echo "  - Referencing wasi-wheels project: https://github.com/dicej/wasi-wheels"
