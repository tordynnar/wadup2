#!/bin/bash
# Universal Python WASM module builder for WADUP
# Builds Python WASM modules using shared runtime components
# Dependencies are stored in deps/ folder

set -e

# Parse arguments
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <module-name> <script-path> [output-dir]"
    echo ""
    echo "Arguments:"
    echo "  module-name  : Name of the module (e.g., 'python-counter')"
    echo "  script-path  : Absolute path to script.py"
    echo "  output-dir   : Optional. Directory for output WASM file"
    echo "                 Default: <script-dir>/../target/"
    echo ""
    echo "Example:"
    echo "  $0 python-counter /path/to/script.py /path/to/output"
    exit 1
fi

MODULE_NAME="$1"
SCRIPT_PATH="$2"
OUTPUT_DIR="${3:-}"

# Convert module name with hyphens to underscores for WASM filename
WASM_NAME=$(echo "$MODULE_NAME" | tr '-' '_')

# Detect workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"

# Set Python directory
PYTHON_VERSION="3.13"
PYTHON_DIR="$WADUP_ROOT/build/python-wasi"

# Detect platform for WASI SDK
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

# Set WASI SDK path in deps folder
WASI_SDK_VERSION="24.0"
WASI_SDK_PATH="$DEPS_DIR/wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"

# Validate inputs
if [ ! -f "$SCRIPT_PATH" ]; then
    echo "ERROR: Script not found: $SCRIPT_PATH"
    exit 1
fi

if [ ! -f "$PYTHON_DIR/lib/libpython${PYTHON_VERSION}.a" ]; then
    echo "ERROR: CPython not built. Run ./scripts/build-python-wasi.sh first"
    echo "Expected: $PYTHON_DIR/lib/libpython${PYTHON_VERSION}.a"
    exit 1
fi

if [ ! -d "$WASI_SDK_PATH" ]; then
    echo "ERROR: WASI SDK not found at: $WASI_SDK_PATH"
    echo "Run ./scripts/download-deps.sh first"
    exit 1
fi

# Check for compression libraries
if [ ! -f "$DEPS_DIR/wasi-zlib/lib/libz.a" ]; then
    echo "ERROR: zlib not found. Run ./scripts/download-deps.sh first"
    exit 1
fi

# Determine output directory
if [ -z "$OUTPUT_DIR" ]; then
    SCRIPT_DIR_PATH="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
    OUTPUT_DIR="$SCRIPT_DIR_PATH/../target"
fi

mkdir -p "$OUTPUT_DIR"

# Create temporary build directory
BUILD_TIMESTAMP=$(date +%s)
BUILD_DIR="/tmp/wadup-python-build-${MODULE_NAME}-${BUILD_TIMESTAMP}"
mkdir -p "$BUILD_DIR"

echo "Building $MODULE_NAME WASM module..."
echo "  Script: $SCRIPT_PATH"
echo "  Output: $OUTPUT_DIR/${WASM_NAME}.wasm"
echo "  Build dir: $BUILD_DIR"

# Copy shared C sources
echo "Copying shared runtime sources..."
cp "$WADUP_ROOT/python-wadup-guest/src/main.c" "$BUILD_DIR/"
cp "$WADUP_ROOT/python-wadup-guest/src/wadup_module.c" "$BUILD_DIR/"
# Note: signal_stubs.c is no longer needed - WADUP provides these as host imports

# Embed Python script
echo "Embedding Python script..."
SCRIPT_CONTENT=$(cat "$SCRIPT_PATH")

# Escape for C string literal
# Replace backslashes first, then quotes, then newlines
ESCAPED=$(echo "$SCRIPT_CONTENT" | \
    sed 's/\\/\\\\/g' | \
    sed 's/"/\\"/g' | \
    sed ':a;N;$!ba;s/\n/\\n/g')

# Write to header file
echo "\"$ESCAPED\"" > "$BUILD_DIR/script.py.h"

# Compiler and linker settings
CC="$WASI_SDK_PATH/bin/clang"
CFLAGS="-O2 -D_WASI_EMULATED_PROCESS_CLOCKS -I$PYTHON_DIR/include -fvisibility=default"
LDFLAGS="-Wl,--allow-undefined -Wl,--export=process -Wl,--initial-memory=134217728 -Wl,--max-memory=268435456 -Wl,--no-entry"

# Compile C sources
echo "Compiling C sources..."
cd "$BUILD_DIR"

"$CC" $CFLAGS -c main.c -o main.o
"$CC" $CFLAGS -c wadup_module.c -o wadup_module.o

# Link into WASM module
# Note: signal/getpid/clock/times/raise/strsignal/dl* stubs are now provided by WADUP host
echo "Linking WASM module..."
"$CC" $CFLAGS main.o wadup_module.o -o "${WASM_NAME}.wasm" \
    -L"$PYTHON_DIR/lib" \
    -lpython${PYTHON_VERSION} \
    "$PYTHON_DIR/lib/libmpdec.a" \
    "$PYTHON_DIR/lib/libexpat.a" \
    "$PYTHON_DIR/lib/libsqlite3.a" \
    $PYTHON_DIR/lib/libHacl_*.a \
    "$DEPS_DIR/wasi-zlib/lib/libz.a" \
    "$DEPS_DIR/wasi-bzip2/lib/libbz2.a" \
    "$DEPS_DIR/wasi-xz/lib/liblzma.a" \
    -lm \
    $LDFLAGS

# Copy to output directory
echo "Copying output..."
cp "${WASM_NAME}.wasm" "$OUTPUT_DIR/"

# Clean up temporary directory
cd /
rm -rf "$BUILD_DIR"

echo "Build successful: $OUTPUT_DIR/${WASM_NAME}.wasm"
ls -lh "$OUTPUT_DIR/${WASM_NAME}.wasm"
