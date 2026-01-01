#!/bin/bash
# Build script for PyOnceLock WASI demonstration
#
# Prerequisites:
# - Rust with wasm32-wasip1 target: rustup target add wasm32-wasip1
# - WASI SDK (automatically downloaded)
# - Python WASI build (automatically built)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}ℹ${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1"; }

echo "============================================"
echo "PyOnceLock WASI Issue Demonstration Builder"
echo "============================================"
echo ""

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" = "darwin" ]; then
    WASI_SDK_OS="macos"
elif [ "$OS" = "linux" ]; then
    WASI_SDK_OS="linux"
else
    error "Unsupported OS: $OS"
    exit 1
fi

WASI_SDK_VERSION="29.0"
WASI_SDK_NAME="wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"
DEPS_DIR="$SCRIPT_DIR/deps"
WASI_SDK_PATH="$DEPS_DIR/$WASI_SDK_NAME"
PYTHON_DIR="$DEPS_DIR/python-wasi"
BUILD_DIR="$SCRIPT_DIR/build"

mkdir -p "$DEPS_DIR"
mkdir -p "$BUILD_DIR"

# Step 1: Download WASI SDK if needed
if [ ! -d "$WASI_SDK_PATH" ]; then
    info "Downloading WASI SDK ${WASI_SDK_VERSION}..."
    WASI_SDK_URL="https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_SDK_VERSION%%.*}/${WASI_SDK_NAME}.tar.gz"
    curl -L -o "$DEPS_DIR/${WASI_SDK_NAME}.tar.gz" "$WASI_SDK_URL"
    tar -xzf "$DEPS_DIR/${WASI_SDK_NAME}.tar.gz" -C "$DEPS_DIR"
    rm "$DEPS_DIR/${WASI_SDK_NAME}.tar.gz"
    success "WASI SDK downloaded"
else
    success "WASI SDK already present"
fi

# Step 2: Build/copy Python for WASI
if [ ! -f "$PYTHON_DIR/lib/libpython3.13.a" ]; then
    # Check if parent project has Python built
    PARENT_PYTHON="../build/python-wasi"
    if [ -f "$PARENT_PYTHON/lib/libpython3.13.a" ]; then
        info "Copying Python WASI build from parent project..."
        mkdir -p "$PYTHON_DIR"
        cp -r "$PARENT_PYTHON/lib" "$PYTHON_DIR/"
        cp -r "$PARENT_PYTHON/include" "$PYTHON_DIR/"
        success "Python WASI copied"
    else
        error "Python WASI not found. Please run from parent project first:"
        error "  cd .. && ./scripts/build-python-wasi.sh"
        exit 1
    fi
else
    success "Python WASI already present"
fi

# Step 3: Copy supporting libraries from parent if needed
for lib in wasi-zlib wasi-bzip2 wasi-xz wasi-sqlite; do
    if [ ! -d "$DEPS_DIR/$lib" ]; then
        if [ -d "../deps/$lib" ]; then
            info "Copying $lib from parent project..."
            cp -r "../deps/$lib" "$DEPS_DIR/"
        else
            warn "$lib not found - some Python features may not work"
        fi
    fi
done

# Step 4: Build Rust extension for WASI
info "Building Rust extension for wasm32-wasip1..."

# Create PyO3 cross-compilation config
PYO3_CONFIG="$BUILD_DIR/pyo3-config.txt"
cat > "$PYO3_CONFIG" << EOF
implementation=CPython
version=3.13
shared=false
abi3=false
lib_name=python3.13
lib_dir=$PYTHON_DIR/lib
pointer_width=32
build_flags=
suppress_build_script_link_lines=true
EOF

export PYO3_CONFIG_FILE="$PYO3_CONFIG"
export CARGO_TARGET_WASM32_WASIP1_LINKER="${WASI_SDK_PATH}/bin/wasm-ld"

cargo build --target wasm32-wasip1 --release

RUST_LIB="$SCRIPT_DIR/target/wasm32-wasip1/release/libpyoncelock_demo.a"
if [ ! -f "$RUST_LIB" ]; then
    error "Rust build failed - library not found"
    exit 1
fi
success "Rust extension built: $(du -h "$RUST_LIB" | cut -f1)"

# Step 5: Compile C main program
info "Compiling C main program..."

CC="${WASI_SDK_PATH}/bin/clang"
WASI_SYSROOT="${WASI_SDK_PATH}/share/wasi-sysroot"
WASI_EMU_LIBS="${WASI_SYSROOT}/lib/wasm32-wasip1"

CFLAGS=(
    "-O2"
    "-D_WASI_EMULATED_SIGNAL"
    "-D_WASI_EMULATED_GETPID"
    "-D_WASI_EMULATED_PROCESS_CLOCKS"
    "-I${PYTHON_DIR}/include"
    "-fvisibility=default"
)

"$CC" "${CFLAGS[@]}" -c main.c -o "$BUILD_DIR/main.o"
success "C main compiled"

# Step 6: Link everything together
info "Linking WASM module..."

# Find Hacl libraries if they exist
HACL_LIBS=()
for lib in "$PYTHON_DIR/lib"/libHacl_*.a; do
    [ -f "$lib" ] && HACL_LIBS+=("$lib")
done

LDFLAGS=(
    "-Wl,--allow-undefined"
    "-Wl,--initial-memory=134217728"
    "-Wl,--max-memory=268435456"
)

LINK_CMD=(
    "$CC"
    "${CFLAGS[@]}"
    "$BUILD_DIR/main.o"
    "$RUST_LIB"
    "-o" "$BUILD_DIR/pyoncelock_demo.wasm"
    "-L${PYTHON_DIR}/lib"
    "-lpython3.13"
)

# Add Python support libraries
for lib in libmpdec.a libexpat.a libsqlite3.a; do
    [ -f "$PYTHON_DIR/lib/$lib" ] && LINK_CMD+=("$PYTHON_DIR/lib/$lib")
done

# Add Hacl libraries
LINK_CMD+=("${HACL_LIBS[@]}")

# Add compression libraries if available
for lib in wasi-zlib/lib/libz.a wasi-bzip2/lib/libbz2.a wasi-xz/lib/liblzma.a; do
    [ -f "$DEPS_DIR/$lib" ] && LINK_CMD+=("$DEPS_DIR/$lib")
done

# Add WASI emulation libraries
LINK_CMD+=(
    "${WASI_EMU_LIBS}/libwasi-emulated-signal.a"
    "${WASI_EMU_LIBS}/libwasi-emulated-getpid.a"
    "${WASI_EMU_LIBS}/libwasi-emulated-process-clocks.a"
    "-lm"
    "${LDFLAGS[@]}"
)

"${LINK_CMD[@]}"

WASM_FILE="$BUILD_DIR/pyoncelock_demo.wasm"
if [ ! -f "$WASM_FILE" ]; then
    error "Linking failed"
    exit 1
fi
success "WASM module built: $(du -h "$WASM_FILE" | cut -f1)"

echo ""
echo "============================================"
success "Build complete!"
echo "============================================"
echo ""
echo "To run the demonstration:"
echo "  ./run.sh"
echo ""
echo "Expected behavior:"
echo "  - TEST 1 (simple_add): Should PASS"
echo "  - TEST 2 (get_cached_type): Will CRASH on WASI due to PyOnceLock"
echo ""
