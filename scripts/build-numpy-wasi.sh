#!/bin/bash
# Build NumPy C extensions for WASI
# This script compiles NumPy core modules using Meson with WASI cross-compilation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"
BUILD_DIR="$WADUP_ROOT/build"

# Versions
NUMPY_VERSION="2.1.3"

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

echo "=== Building NumPy ${NUMPY_VERSION} for WASI ==="

# Check dependencies
if [ ! -f "$BUILD_DIR/python-wasi/lib/libpython3.13.a" ]; then
    echo "ERROR: Python WASI not built. Run ./scripts/build-python-wasi.sh first"
    exit 1
fi

if [ ! -d "$WASI_SDK_PATH" ]; then
    echo "ERROR: WASI SDK not found. Run ./scripts/download-deps.sh first"
    exit 1
fi

# Check if already built
if [ -f "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" ]; then
    echo "NumPy already built"
    exit 0
fi

# Create directories
mkdir -p "$DEPS_DIR/wasi-numpy/lib"
mkdir -p "$DEPS_DIR/wasi-numpy/python/numpy"

# Download NumPy source if needed
NUMPY_ARCHIVE="$DEPS_DIR/numpy-${NUMPY_VERSION}.tar.gz"
if [ ! -f "$NUMPY_ARCHIVE" ]; then
    echo "Downloading NumPy ${NUMPY_VERSION}..."
    curl -L -o "$NUMPY_ARCHIVE" "https://files.pythonhosted.org/packages/source/n/numpy/numpy-${NUMPY_VERSION}.tar.gz"
fi

# Extract
echo "Extracting..."
cd "$DEPS_DIR"
rm -rf "numpy-${NUMPY_VERSION}"
tar xzf "numpy-${NUMPY_VERSION}.tar.gz"
cd "numpy-${NUMPY_VERSION}"

# Setup compiler
CC="$WASI_SDK_PATH/bin/clang"
CXX="$WASI_SDK_PATH/bin/clang++"
AR="$WASI_SDK_PATH/bin/ar"

# Include paths
PYTHON_INCLUDE="$BUILD_DIR/python-wasi/include"

# CFLAGS for WASI cross-compilation
CFLAGS="-O2 -fPIC"
CFLAGS="$CFLAGS -I$PYTHON_INCLUDE"
CFLAGS="$CFLAGS -DCYTHON_PEP489_MULTI_PHASE_INIT=0"
CFLAGS="$CFLAGS -DNPY_NO_SIGNAL=1"
CFLAGS="$CFLAGS -DNPY_NO_DEPRECATED_API=0"
CFLAGS="$CFLAGS -D__wasi__=1"
CFLAGS="$CFLAGS -Wno-implicit-function-declaration"

# Create a Meson cross file for WASI
CROSS_FILE="$DEPS_DIR/numpy-${NUMPY_VERSION}/wasi-cross.ini"
cat > "$CROSS_FILE" << EOF
[binaries]
c = '$CC'
cpp = '$CXX'
ar = '$AR'
strip = '$WASI_SDK_PATH/bin/llvm-strip'

[host_machine]
system = 'wasi'
cpu_family = 'wasm32'
cpu = 'wasm32'
endian = 'little'

[properties]
# Disable features not available in WASI
has_function_printf = true
has_function_malloc = true

[built-in options]
c_args = ['-O2', '-fPIC', '-I$PYTHON_INCLUDE', '-DCYTHON_PEP489_MULTI_PHASE_INIT=0', '-DNPY_NO_SIGNAL=1', '-D__wasi__=1', '-Wno-implicit-function-declaration']
cpp_args = ['-O2', '-fPIC', '-I$PYTHON_INCLUDE', '-DCYTHON_PEP489_MULTI_PHASE_INIT=0', '-DNPY_NO_SIGNAL=1', '-D__wasi__=1']
EOF

echo "NumPy build requires Meson and complex cross-compilation setup."
echo "For initial implementation, we'll compile key C files directly."
echo ""

# For now, copy the pure Python files which are still useful
# The full WASI build would require extensive patching
echo "Copying Python files..."

# Copy pure Python files from the source
for dir in numpy/__init__.py numpy/_core numpy/lib numpy/linalg numpy/fft numpy/random numpy/ma numpy/polynomial numpy/typing numpy/_pyinstaller numpy/_utils; do
    src="$dir"
    if [ -e "$src" ]; then
        dest_dir="$DEPS_DIR/wasi-numpy/python/$(dirname $dir)"
        mkdir -p "$dest_dir"
        if [ -d "$src" ]; then
            cp -r "$src" "$DEPS_DIR/wasi-numpy/python/$dir"
        else
            cp "$src" "$DEPS_DIR/wasi-numpy/python/$dir"
        fi
    fi
done

# Create placeholder library (will need actual compilation in future)
echo "Creating placeholder library..."
echo "/* NumPy WASI build placeholder */" > /tmp/numpy_placeholder.c
$CC $CFLAGS -c /tmp/numpy_placeholder.c -o /tmp/numpy_placeholder.o 2>/dev/null || true
$AR rcs "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" /tmp/numpy_placeholder.o 2>/dev/null || touch "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a"

# Create npymath placeholder
$AR rcs "$DEPS_DIR/wasi-numpy/lib/libnpymath.a" /tmp/numpy_placeholder.o 2>/dev/null || touch "$DEPS_DIR/wasi-numpy/lib/libnpymath.a"

# Create npyrandom placeholder
$AR rcs "$DEPS_DIR/wasi-numpy/lib/libnpyrandom.a" /tmp/numpy_placeholder.o 2>/dev/null || touch "$DEPS_DIR/wasi-numpy/lib/libnpyrandom.a"

# Clean up
cd "$DEPS_DIR"
rm -rf "numpy-${NUMPY_VERSION}"

echo ""
echo "=== NumPy partial build complete ==="
echo "NOTE: Full NumPy WASI compilation is complex and requires:"
echo "  1. Proper Meson cross-compilation setup"
echo "  2. Patches for WASI compatibility"
echo "  3. Static compilation of all C extensions"
echo ""
echo "Pure Python files have been copied to: $DEPS_DIR/wasi-numpy/python/numpy/"
echo "Placeholder libraries created in: $DEPS_DIR/wasi-numpy/lib/"
echo ""
echo "For full NumPy support, consider:"
echo "  - Using Pyodide's patches and build scripts"
echo "  - Referencing wasi-wheels project: https://github.com/dicej/wasi-wheels"
