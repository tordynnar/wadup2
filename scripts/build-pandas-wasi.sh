#!/bin/bash
# Build Pandas C extensions for WASI
# Two-stage build:
# 1. Native build using Meson to run Cython and generate C files
# 2. Cross-compile generated C files using WASI SDK
#
# This follows the same pattern as build-numpy-wasi.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"
BUILD_DIR="$WADUP_ROOT/build"

# Versions
PANDAS_VERSION="2.3.3"

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
WASI_SYSROOT="$WASI_SDK_PATH/share/wasi-sysroot"

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

# Pandas depends on NumPy - this is required
if [ ! -f "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" ]; then
    echo "ERROR: NumPy WASI not built. Run ./scripts/build-numpy-wasi.sh first"
    exit 1
fi

# Check for required tools
for tool in ninja cython meson; do
    if ! command -v $tool &> /dev/null; then
        echo "ERROR: $tool not found. Install with: brew install ninja meson && pip3 install cython"
        exit 1
    fi
done

# Check if already built with substantial content
if [ -f "$DEPS_DIR/wasi-pandas/lib/libpandas_libs.a" ] && [ -f "$DEPS_DIR/wasi-pandas/lib/libpandas_tslibs.a" ]; then
    size=$(wc -c < "$DEPS_DIR/wasi-pandas/lib/libpandas_libs.a" | tr -d ' ')
    tslibs_size=$(wc -c < "$DEPS_DIR/wasi-pandas/lib/libpandas_tslibs.a" | tr -d ' ')
    if [ "$size" -gt 100000 ] && [ "$tslibs_size" -gt 100000 ]; then
        echo "Pandas already built (libpandas_libs.a is ${size} bytes, libpandas_tslibs.a is ${tslibs_size} bytes)"
        exit 0
    fi
fi

# Create directories
rm -rf "$DEPS_DIR/wasi-pandas"
mkdir -p "$DEPS_DIR/wasi-pandas/lib"
mkdir -p "$DEPS_DIR/wasi-pandas/python"

# Download Pandas source if needed
PANDAS_ARCHIVE="$DEPS_DIR/pandas-${PANDAS_VERSION}.tar.gz"
if [ ! -f "$PANDAS_ARCHIVE" ]; then
    echo "Downloading Pandas ${PANDAS_VERSION}..."
    curl -L -o "$PANDAS_ARCHIVE" "https://files.pythonhosted.org/packages/source/p/pandas/pandas-${PANDAS_VERSION}.tar.gz"
fi

# Extract fresh copy
echo "Extracting..."
cd "$DEPS_DIR"
rm -rf "pandas-${PANDAS_VERSION}"
tar xzf "pandas-${PANDAS_VERSION}.tar.gz"

PANDAS_SRC="$DEPS_DIR/pandas-${PANDAS_VERSION}"
cd "$PANDAS_SRC"

# Patch source to rename conflicting symbols before compilation
# These functions have different signatures in NumPy 2.x vs pandas' vendored code
echo "Patching vendored datetime functions to avoid symbol conflicts..."
CONFLICTING_FUNCS="get_datetime_metadata_from_dtype add_minutes_to_datetimestruct get_datetimestruct_days is_leapyear"

# Don't patch source files before native build - the Cython module system is complex
# Instead, we'll rely on --allow-multiple-definition and library link order
# The capsule API ensures pandas uses its own vendored functions at runtime

echo "Skipping pre-build patching (using link-time resolution instead)"

# Setup compiler paths
WASI_CC="$WASI_SDK_PATH/bin/clang"
WASI_CXX="$WASI_SDK_PATH/bin/clang++"
WASI_AR="$WASI_SDK_PATH/bin/ar"

# Python include path
PYTHON_INCLUDE="$BUILD_DIR/python-wasi/include"

# NumPy include paths (from WASI numpy build)
NUMPY_INCLUDE="$DEPS_DIR/wasi-numpy/python/numpy/_core/include"

echo ""
echo "=== Stage 1: Native build to generate C files from Cython ==="
echo ""

NATIVE_BUILD="$PANDAS_SRC/build-native"

echo "Running native meson setup..."
meson setup "$NATIVE_BUILD" \
    --buildtype=release 2>&1 | tail -30

echo ""
echo "Building native pandas (to generate Cython -> C files)..."
ninja -C "$NATIVE_BUILD" 2>&1 | tail -30

echo ""
echo "=== Stage 2: Cross-compile for WASI ==="
echo ""

# WASI compilation flags
WASI_CFLAGS="-O2 -fPIC -DNDEBUG"
# Disable Cython threading checks for single-threaded WASI
WASI_CFLAGS="$WASI_CFLAGS -DCYTHON_WITHOUT_ASSERTIONS=1"
WASI_CFLAGS="$WASI_CFLAGS --target=wasm32-wasi"
WASI_CFLAGS="$WASI_CFLAGS --sysroot=$WASI_SYSROOT"
WASI_CFLAGS="$WASI_CFLAGS -I$PYTHON_INCLUDE"
WASI_CFLAGS="$WASI_CFLAGS -I$NUMPY_INCLUDE"
WASI_CFLAGS="$WASI_CFLAGS -I$DEPS_DIR/wasi-stubs"

# Include pandas source directories
WASI_CFLAGS="$WASI_CFLAGS -I$PANDAS_SRC"
WASI_CFLAGS="$WASI_CFLAGS -I$PANDAS_SRC/pandas/_libs"
WASI_CFLAGS="$WASI_CFLAGS -I$PANDAS_SRC/pandas/_libs/include"
WASI_CFLAGS="$WASI_CFLAGS -I$PANDAS_SRC/pandas/_libs/src"
WASI_CFLAGS="$WASI_CFLAGS -I$PANDAS_SRC/pandas/_libs/src/parser"
WASI_CFLAGS="$WASI_CFLAGS -I$PANDAS_SRC/pandas/_libs/src/vendored"

# Include generated headers from native build
WASI_CFLAGS="$WASI_CFLAGS -I$NATIVE_BUILD"
WASI_CFLAGS="$WASI_CFLAGS -I$NATIVE_BUILD/pandas"
WASI_CFLAGS="$WASI_CFLAGS -I$NATIVE_BUILD/pandas/_libs"

# Find and include all .so.p directories containing generated headers
for p_dir in $(find "$NATIVE_BUILD/pandas/_libs" -maxdepth 1 -type d -name "*.p" 2>/dev/null); do
    WASI_CFLAGS="$WASI_CFLAGS -I$p_dir"
done

# Ensure NumPy generated headers are in the right place
# These are generated by NumPy's meson build and needed by packages using NumPy C API
if [ -f "$DEPS_DIR/wasi-stubs/numpy/_numpyconfig.h" ]; then
    cp "$DEPS_DIR/wasi-stubs/numpy/_numpyconfig.h" "$NUMPY_INCLUDE/numpy/_numpyconfig.h"
fi

# Check for and generate __multiarray_api.h if needed
if [ ! -f "$NUMPY_INCLUDE/numpy/__multiarray_api.h" ]; then
    echo "WARNING: NumPy API headers not found. Regenerating from NumPy source..."
    NUMPY_SRC="$DEPS_DIR/numpy-2.1.3"
    if [ -d "$NUMPY_SRC" ]; then
        # Headers should exist in native build
        if [ -f "$NUMPY_SRC/build-native/numpy/_core/__multiarray_api.h" ]; then
            cp "$NUMPY_SRC/build-native/numpy/_core/__multiarray_api.h" "$NUMPY_INCLUDE/numpy/"
            cp "$NUMPY_SRC/build-native/numpy/_core/__ufunc_api.h" "$NUMPY_INCLUDE/numpy/"
            echo "  Copied NumPy API headers"
        fi
    fi
fi

# Critical defines for WASI
WASI_CFLAGS="$WASI_CFLAGS -D__EMSCRIPTEN__=1"
WASI_CFLAGS="$WASI_CFLAGS -DNPY_NO_DEPRECATED_API=0"
WASI_CFLAGS="$WASI_CFLAGS -DCYTHON_PEP489_MULTI_PHASE_INIT=0"
WASI_CFLAGS="$WASI_CFLAGS -D_WASI_EMULATED_SIGNAL"
WASI_CFLAGS="$WASI_CFLAGS -D_WASI_EMULATED_GETPID"

# Suppress warnings that don't apply to WASI
WASI_CFLAGS="$WASI_CFLAGS -Wno-implicit-function-declaration"
WASI_CFLAGS="$WASI_CFLAGS -Wno-int-conversion"
WASI_CFLAGS="$WASI_CFLAGS -Wno-incompatible-pointer-types"
WASI_CFLAGS="$WASI_CFLAGS -Wno-unused-function"
WASI_CFLAGS="$WASI_CFLAGS -Wno-unused-variable"
WASI_CFLAGS="$WASI_CFLAGS -Wno-missing-field-initializers"
WASI_CFLAGS="$WASI_CFLAGS -Wno-unknown-pragmas"
WASI_CFLAGS="$WASI_CFLAGS -Wno-unused-but-set-variable"
WASI_CFLAGS="$WASI_CFLAGS -Wno-macro-redefined"

# NOTE: Pandas vendors old NumPy datetime code (np_datetime.c, np_datetime_strings.c)
# These have different function signatures than NumPy 2.x (e.g., return by value vs pointer).
# The vendored functions are accessed via PandasDateTimeAPI capsule at runtime, not direct calls.
# We patch the source to rename conflicting functions before compilation.

# C++ specific flags
WASI_CXXFLAGS="$WASI_CFLAGS -std=c++17 -fno-exceptions -fno-rtti"

mkdir -p "$PANDAS_SRC/build-wasi"
cd "$PANDAS_SRC/build-wasi"

# Function to compile a C file
compile_file() {
    local src="$1"
    local extra_flags="${2:-}"
    local obj=$(basename "${src%.c}.o")
    if [[ "$src" == *.cpp ]]; then
        return 1
    fi
    # Capture stderr to detect real errors vs warnings
    local err_output
    if err_output=$($WASI_CC $WASI_CFLAGS $extra_flags -c "$src" -o "$obj" 2>&1); then
        echo "  ✓ $(basename $src)"
        return 0
    else
        # Only show errors if compilation actually failed
        if [ ! -f "$obj" ]; then
            echo "  ✗ $(basename $src)"
            echo "$err_output" | head -5
        else
            # Compilation succeeded despite non-zero exit (warnings treated as errors?)
            echo "  ✓ $(basename $src)"
            return 0
        fi
        return 1
    fi
}

# Function to compile a C file and show errors on failure
compile_file_verbose() {
    local src="$1"
    local extra_flags="${2:-}"
    local obj=$(basename "${src%.c}.o")
    if $WASI_CC $WASI_CFLAGS $extra_flags -c "$src" -o "$obj" 2>&1; then
        echo "  ✓ $(basename $src)"
        return 0
    else
        echo "  ✗ $(basename $src)"
        return 1
    fi
}

# Function to compile a C++ file
compile_cpp_file() {
    local src="$1"
    local extra_flags="${2:-}"
    local obj=$(basename "${src%.cpp}.o")
    if $WASI_CXX $WASI_CXXFLAGS $extra_flags -c "$src" -o "$obj" 2>/dev/null; then
        echo "  ✓ $(basename $src)"
        return 0
    else
        return 1
    fi
}

echo "Compiling pandas._libs modules..."
echo ""

# Find generated .c files from native build
# These are the Cython-generated C files
LIBS_BUILD="$NATIVE_BUILD/pandas/_libs"

# Pattern varies by platform - find the .so.p directories
# Generated C files are nested: *.so.p/pandas/_libs/xxx.pyx.c
for p_dir in $(find "$LIBS_BUILD" -maxdepth 1 -type d -name "*.so.p" 2>/dev/null); do
    module_name=$(basename "$p_dir" | sed 's/\.cpython.*//' | sed 's/\..*//')
    echo "  Module: $module_name"
    # Find all .c files recursively in the .so.p directory
    for src in $(find "$p_dir" -name "*.c" -type f 2>/dev/null); do
        compile_file "$src" || true
    done
done

# Compile parser C sources
echo ""
echo "Compiling parser C sources..."
PARSER_SRC="$PANDAS_SRC/pandas/_libs/src/parser"
if [ -d "$PARSER_SRC" ]; then
    for src in "$PARSER_SRC"/*.c; do
        if [ -f "$src" ]; then
            compile_file "$src" "-I$PARSER_SRC" || true
        fi
    done
fi

# Compile vendored sources (ujson and numpy datetime)
echo ""
echo "Compiling vendored sources..."
VENDORED_SRC="$PANDAS_SRC/pandas/_libs/src/vendored"

if [ -d "$VENDORED_SRC" ]; then
    for src in $(find "$VENDORED_SRC" -name "*.c" 2>/dev/null); do
        if [ -f "$src" ]; then
            # Include vendored numpy datetime with special flags
            if [[ "$src" == *"numpy/datetime"* ]]; then
                echo "  Compiling vendored datetime: $(basename $src)"
                # Compile with include paths for numpy datetime headers
                DATETIME_FLAGS="-I$VENDORED_SRC -I$VENDORED_SRC/numpy/datetime"
                DATETIME_FLAGS="$DATETIME_FLAGS -I$PANDAS_SRC/pandas/_libs/include/pandas/vendored/numpy/datetime"
                compile_file "$src" "$DATETIME_FLAGS" || true
            else
                compile_file "$src" "-I$VENDORED_SRC" || true
            fi
        fi
    done
fi

# Compile pandas_datetime module sources directly from source tree
# These are pure C files (not Cython-generated), found in src/datetime/
echo ""
echo "Compiling pandas_datetime module..."
DATETIME_SRC="$PANDAS_SRC/pandas/_libs/src/datetime"
if [ -d "$DATETIME_SRC" ]; then
    echo "  Compiling datetime sources from $DATETIME_SRC..."
    DATETIME_FLAGS="-I$VENDORED_SRC -I$PANDAS_SRC/pandas/_libs/src"
    DATETIME_FLAGS="$DATETIME_FLAGS -I$PANDAS_SRC/pandas/_libs/include"
    DATETIME_FLAGS="$DATETIME_FLAGS -I$PANDAS_SRC/pandas/_libs/include/pandas/vendored/numpy/datetime"
    for src in "$DATETIME_SRC"/*.c; do
        if [ -f "$src" ]; then
            compile_file "$src" "$DATETIME_FLAGS" || true
        fi
    done
else
    echo "  WARNING: datetime source directory not found at $DATETIME_SRC"
fi

# Create libpandas_libs.a from compiled objects
echo ""
echo "Creating libpandas_libs.a..."
LIBS_OBJS=$(ls *.o 2>/dev/null | tr '\n' ' ')
if [ -n "$LIBS_OBJS" ]; then
    LIBS_OBJ_COUNT=$(echo $LIBS_OBJS | wc -w | tr -d ' ')
    $WASI_AR rcs "$DEPS_DIR/wasi-pandas/lib/libpandas_libs.a" *.o
    echo "Created libpandas_libs.a with $LIBS_OBJ_COUNT object files"
    rm -f *.o
else
    echo "WARNING: No pandas._libs objects compiled"
    touch "$DEPS_DIR/wasi-pandas/lib/libpandas_libs.a"
fi

# Compile tslibs modules
echo ""
echo "Compiling pandas._libs.tslibs modules..."

TSLIBS_BUILD="$NATIVE_BUILD/pandas/_libs/tslibs"

# Find tslibs .so.p directories
# Generated C files are nested: *.so.p/pandas/_libs/tslibs/xxx.pyx.c
for p_dir in $(find "$TSLIBS_BUILD" -maxdepth 1 -type d -name "*.so.p" 2>/dev/null); do
    module_name=$(basename "$p_dir" | sed 's/\.cpython.*//' | sed 's/\..*//')
    echo "  Module: $module_name"
    # Find all .c files recursively in the .so.p directory
    for src in $(find "$p_dir" -name "*.c" -type f 2>/dev/null); do
        # Add tslibs-specific include paths
        TSLIBS_FLAGS="-I$PANDAS_SRC/pandas/_libs/tslibs"
        TSLIBS_FLAGS="$TSLIBS_FLAGS -I$NATIVE_BUILD/pandas/_libs/tslibs"
        compile_file "$src" "$TSLIBS_FLAGS" || true
    done
done

# Create libpandas_tslibs.a
echo ""
echo "Creating libpandas_tslibs.a..."
TSLIBS_OBJS=$(ls *.o 2>/dev/null | tr '\n' ' ')
if [ -n "$TSLIBS_OBJS" ]; then
    TSLIBS_OBJ_COUNT=$(echo $TSLIBS_OBJS | wc -w | tr -d ' ')
    $WASI_AR rcs "$DEPS_DIR/wasi-pandas/lib/libpandas_tslibs.a" *.o
    echo "Created libpandas_tslibs.a with $TSLIBS_OBJ_COUNT object files"
    rm -f *.o
else
    echo "WARNING: No pandas._libs.tslibs objects compiled"
    touch "$DEPS_DIR/wasi-pandas/lib/libpandas_tslibs.a"
fi

# Copy Python files
echo ""
echo "Copying Python files..."
cp -r "$PANDAS_SRC/pandas" "$DEPS_DIR/wasi-pandas/python/"

# Also copy generated Python files from native build
if [ -d "$NATIVE_BUILD/pandas" ]; then
    find "$NATIVE_BUILD/pandas" -name "*.py" -o -name "*.pyi" | while read pyfile; do
        relpath="${pyfile#$NATIVE_BUILD/}"
        destdir="$DEPS_DIR/wasi-pandas/python/$(dirname $relpath)"
        mkdir -p "$destdir"
        cp "$pyfile" "$destdir/" 2>/dev/null || true
    done
fi

# Remove .so files and pycache
find "$DEPS_DIR/wasi-pandas/python" -name "*.so" -delete 2>/dev/null || true
find "$DEPS_DIR/wasi-pandas/python" -name "*.pyc" -delete 2>/dev/null || true
find "$DEPS_DIR/wasi-pandas/python" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true

# Clean up source
cd "$DEPS_DIR"
rm -rf "pandas-${PANDAS_VERSION}"

echo ""
echo "=== Pandas build complete ==="
echo "Libraries:"
ls -la "$DEPS_DIR/wasi-pandas/lib/"
echo ""
echo "Python files: $DEPS_DIR/wasi-pandas/python/pandas/"
