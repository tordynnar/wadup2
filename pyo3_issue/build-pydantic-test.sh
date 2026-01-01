#!/bin/bash
# Build pydantic_core test for WASI

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Paths
WASI_SDK_PATH="$SCRIPT_DIR/deps/wasi-sdk-29.0-arm64-macos"
PYTHON_DIR="$SCRIPT_DIR/deps/python-wasi"
PYDANTIC_LIB="/Users/jared/Projects/wadup2/deps/pydantic-core-2.41.5/target/wasm32-wasip1/release/lib_pydantic_core.a"
BUILD_DIR="$SCRIPT_DIR/build"
WASI_SYSROOT="${WASI_SDK_PATH}/share/wasi-sysroot"
WASI_EMU_LIBS="${WASI_SYSROOT}/lib/wasm32-wasip1"

CC="${WASI_SDK_PATH}/bin/clang"

echo "============================================"
echo "Building pydantic_core WASI test"
echo "============================================"
echo ""

# Check dependencies
if [ ! -f "$PYDANTIC_LIB" ]; then
    echo "ERROR: pydantic_core library not found at $PYDANTIC_LIB"
    echo "Build it first with: cd ../deps/pydantic-core-2.41.5 && cargo build --target wasm32-wasip1 --release"
    exit 1
fi
echo "Using pydantic_core: $(du -h "$PYDANTIC_LIB" | cut -f1)"

# Compile C program
echo "Compiling test_pydantic_core.c..."
CFLAGS=(
    "-O2"
    "-D_WASI_EMULATED_SIGNAL"
    "-D_WASI_EMULATED_GETPID"
    "-D_WASI_EMULATED_PROCESS_CLOCKS"
    "-I${PYTHON_DIR}/include"
    "-fvisibility=default"
)

"$CC" "${CFLAGS[@]}" -c test_pydantic_core.c -o "$BUILD_DIR/test_pydantic_core.o"
echo "Compiled."

# Find Hacl libraries
HACL_LIBS=()
for lib in "$PYTHON_DIR/lib"/libHacl_*.a; do
    [ -f "$lib" ] && HACL_LIBS+=("$lib")
done

# Link
echo "Linking..."
LDFLAGS=(
    "-Wl,--allow-undefined"
    "-Wl,--initial-memory=268435456"    # 256MB
    "-Wl,--max-memory=536870912"         # 512MB (pydantic is big)
)

LINK_CMD=(
    "$CC"
    "${CFLAGS[@]}"
    "$BUILD_DIR/test_pydantic_core.o"
    "$PYDANTIC_LIB"
    "-o" "$BUILD_DIR/test_pydantic_core.wasm"
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
    [ -f "$SCRIPT_DIR/deps/$lib" ] && LINK_CMD+=("$SCRIPT_DIR/deps/$lib")
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

if [ -f "$BUILD_DIR/test_pydantic_core.wasm" ]; then
    echo ""
    echo "============================================"
    echo "Build complete: $(du -h "$BUILD_DIR/test_pydantic_core.wasm" | cut -f1)"
    echo "============================================"
    echo ""
    echo "Run with:"
    echo "  wasmtime run --dir=/ --env HOME=/ --env PYTHONDONTWRITEBYTECODE=1 $BUILD_DIR/test_pydantic_core.wasm"
else
    echo "ERROR: Build failed"
    exit 1
fi
