#!/bin/bash
# Build lxml C extension for WASI
# This script compiles lxml.etree from the pre-cythonized C code

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"
BUILD_DIR="$WADUP_ROOT/build"

# Versions
LXML_VERSION="6.0.2"

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

echo "=== Building lxml ${LXML_VERSION} for WASI ==="

# Check dependencies
if [ ! -f "$DEPS_DIR/wasi-libxml2/lib/libxml2.a" ]; then
    echo "ERROR: libxml2 not built. Run ./scripts/download-deps.sh first"
    exit 1
fi

if [ ! -f "$BUILD_DIR/python-wasi/lib/libpython3.13.a" ]; then
    echo "ERROR: Python WASI not built. Run ./scripts/build-python-wasi.sh first"
    exit 1
fi

# Check if already built
if [ -f "$DEPS_DIR/wasi-lxml/lib/liblxml_etree.a" ]; then
    echo "lxml already built"
    exit 0
fi

# Create directories
mkdir -p "$DEPS_DIR/wasi-lxml/lib"
mkdir -p "$DEPS_DIR/wasi-lxml/python/lxml"

# Download lxml source if needed
LXML_ARCHIVE="$DEPS_DIR/lxml-${LXML_VERSION}.tar.gz"
if [ ! -f "$LXML_ARCHIVE" ]; then
    echo "Downloading lxml ${LXML_VERSION}..."
    curl -L -o "$LXML_ARCHIVE" "https://files.pythonhosted.org/packages/source/l/lxml/lxml-${LXML_VERSION}.tar.gz"
fi

# Extract
echo "Extracting..."
cd "$DEPS_DIR"
rm -rf "lxml-${LXML_VERSION}"
tar xzf "lxml-${LXML_VERSION}.tar.gz"
cd "lxml-${LXML_VERSION}"

# Setup compiler
CC="$WASI_SDK_PATH/bin/clang"
AR="$WASI_SDK_PATH/bin/ar"

# Check libxslt
if [ ! -f "$DEPS_DIR/wasi-libxslt/lib/libxslt.a" ]; then
    echo "ERROR: libxslt not built. Run ./scripts/download-deps.sh first"
    exit 1
fi

# Include paths
PYTHON_INCLUDE="$BUILD_DIR/python-wasi/include"
LIBXML2_INCLUDE="$DEPS_DIR/wasi-libxml2/include/libxml2"
LIBXSLT_INCLUDE="$DEPS_DIR/wasi-libxslt/include"
LXML_INCLUDE="$DEPS_DIR/lxml-${LXML_VERSION}/src/lxml/includes"

CFLAGS="-O2 -fPIC"
CFLAGS="$CFLAGS -I$PYTHON_INCLUDE"
CFLAGS="$CFLAGS -I$LIBXML2_INCLUDE"
CFLAGS="$CFLAGS -I$LIBXSLT_INCLUDE"
CFLAGS="$CFLAGS -I$LXML_INCLUDE"
CFLAGS="$CFLAGS -I$DEPS_DIR/lxml-${LXML_VERSION}/src/lxml"

# Important defines for WASI/static linking
CFLAGS="$CFLAGS -DCYTHON_PEP489_MULTI_PHASE_INIT=0"
CFLAGS="$CFLAGS -DLIBXML_STATIC"
CFLAGS="$CFLAGS -DLIBXSLT_STATIC"

echo "Compiling lxml.etree..."
$CC $CFLAGS -c src/lxml/etree.c -o etree.o 2>&1

echo "Creating static library..."
$AR rcs liblxml_etree.a etree.o

# Copy to output
cp liblxml_etree.a "$DEPS_DIR/wasi-lxml/lib/"

# Copy pure Python files
echo "Copying Python files..."
cp src/lxml/__init__.py "$DEPS_DIR/wasi-lxml/python/lxml/"
cp src/lxml/_elementpath.py "$DEPS_DIR/wasi-lxml/python/lxml/"

# Note: The etree module is a C extension that will be registered
# via PyImport_AppendInittab as "lxml.etree" before Python initialization.
# No stub file is needed - the C extension is loaded directly.

# Clean up
cd "$DEPS_DIR"
rm -rf "lxml-${LXML_VERSION}"

echo ""
echo "=== lxml build complete ==="
echo "Library: $DEPS_DIR/wasi-lxml/lib/liblxml_etree.a"
echo "Python:  $DEPS_DIR/wasi-lxml/python/lxml/"
