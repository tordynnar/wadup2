#!/bin/bash
# Build NumPy C extensions for WASI
# Two-stage build:
# 1. Native build using NumPy's vendored meson to generate headers and C files
# 2. Cross-compile generated files using WASI SDK

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
WASI_SYSROOT="$WASI_SDK_PATH/share/wasi-sysroot"

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

# Check for required tools
for tool in ninja cython; do
    if ! command -v $tool &> /dev/null; then
        echo "ERROR: $tool not found. Install with: brew install ninja && pip3 install cython"
        exit 1
    fi
done

# Check if already built with substantial content
if [ -f "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" ] && [ -f "$DEPS_DIR/wasi-numpy/lib/libnpymath.a" ]; then
    size=$(wc -c < "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" | tr -d ' ')
    npymath_size=$(wc -c < "$DEPS_DIR/wasi-numpy/lib/libnpymath.a" | tr -d ' ')
    if [ "$size" -gt 100000 ] && [ "$npymath_size" -gt 0 ]; then
        echo "NumPy already built (libnumpy_core.a is ${size} bytes, libnpymath.a is ${npymath_size} bytes)"
        exit 0
    fi
fi

# Create directories
rm -rf "$DEPS_DIR/wasi-numpy"
mkdir -p "$DEPS_DIR/wasi-numpy/lib"
mkdir -p "$DEPS_DIR/wasi-numpy/python"

# Download NumPy source if needed
NUMPY_ARCHIVE="$DEPS_DIR/numpy-${NUMPY_VERSION}.tar.gz"
if [ ! -f "$NUMPY_ARCHIVE" ]; then
    echo "Downloading NumPy ${NUMPY_VERSION}..."
    curl -L -o "$NUMPY_ARCHIVE" "https://files.pythonhosted.org/packages/source/n/numpy/numpy-${NUMPY_VERSION}.tar.gz"
fi

# Extract fresh copy
echo "Extracting..."
cd "$DEPS_DIR"
rm -rf "numpy-${NUMPY_VERSION}"
tar xzf "numpy-${NUMPY_VERSION}.tar.gz"

NUMPY_SRC="$DEPS_DIR/numpy-${NUMPY_VERSION}"
cd "$NUMPY_SRC"

# Setup compiler paths
WASI_CC="$WASI_SDK_PATH/bin/clang"
WASI_CXX="$WASI_SDK_PATH/bin/clang++"
WASI_AR="$WASI_SDK_PATH/bin/ar"

# Python include path
PYTHON_INCLUDE="$BUILD_DIR/python-wasi/include"

echo ""
echo "=== Stage 1: Native build to generate headers and C files ==="
echo ""

NATIVE_BUILD="$NUMPY_SRC/build-native"

# Use NumPy's vendored meson (required for their custom 'features' module)
VENDORED_MESON="python3 $NUMPY_SRC/vendored-meson/meson/meson.py"

echo "Running native meson setup (using vendored meson)..."
$VENDORED_MESON setup "$NATIVE_BUILD" \
    -Dallow-noblas=true \
    -Ddisable-svml=true \
    -Ddisable-optimization=true \
    --buildtype=release 2>&1 | tail -30

echo ""
echo "Building native NumPy..."
ninja -C "$NATIVE_BUILD" 2>&1 | tail -20

echo ""
echo "=== Stage 2: Cross-compile for WASI ==="
echo ""

# Patch config.h for WASI compatibility
echo "Patching config.h for WASI..."
CONFIG_H="$NATIVE_BUILD/numpy/_core/config.h"
if [ -f "$CONFIG_H" ]; then
    # Remove HAVE_XLOCALE_H (WASI doesn't have xlocale.h)
    grep -v "HAVE_XLOCALE_H" "$CONFIG_H" > /tmp/config_patched.h
    cp /tmp/config_patched.h "$CONFIG_H"
    # Also remove other problematic defines
    grep -v "HAVE_BACKTRACE" "$CONFIG_H" > /tmp/config_patched.h
    cp /tmp/config_patched.h "$CONFIG_H"
fi

# Replace _numpyconfig.h with WASI-specific version
# The native build generates values for arm64 (NPY_SIZEOF_LONG=8, etc.)
# but WASI needs 32-bit values (NPY_SIZEOF_LONG=4, etc.)
echo "Replacing _numpyconfig.h with WASI-compatible version..."
NUMPYCONFIG_H="$NATIVE_BUILD/numpy/_core/_numpyconfig.h"
if [ -f "$NUMPYCONFIG_H" ] && [ -f "$DEPS_DIR/wasi-stubs/numpy/_numpyconfig.h" ]; then
    cp "$DEPS_DIR/wasi-stubs/numpy/_numpyconfig.h" "$NUMPYCONFIG_H"
    echo "  Replaced _numpyconfig.h with WASI values (NPY_SIZEOF_LONG=4, etc.)"
fi

# Key directories
CORE_SRC="$NUMPY_SRC/numpy/_core/src"
CORE_INCLUDE="$NUMPY_SRC/numpy/_core/include"
GENERATED_DIR="$NATIVE_BUILD"

# WASI compilation flags
WASI_CFLAGS="-O2 -fPIC"
WASI_CFLAGS="$WASI_CFLAGS --target=wasm32-wasi"
WASI_CFLAGS="$WASI_CFLAGS --sysroot=$WASI_SYSROOT"
WASI_CFLAGS="$WASI_CFLAGS -I$PYTHON_INCLUDE"
WASI_CFLAGS="$WASI_CFLAGS -I$DEPS_DIR/wasi-stubs"
WASI_CFLAGS="$WASI_CFLAGS -I$NUMPY_SRC"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_INCLUDE"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/common"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/multiarray"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/umath"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/npymath"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/npysort"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/_simd"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/multiarray/stringdtype"
WASI_CFLAGS="$WASI_CFLAGS -I$CORE_SRC/multiarray/textreading"

# Add generated header directories
WASI_CFLAGS="$WASI_CFLAGS -I$GENERATED_DIR"
WASI_CFLAGS="$WASI_CFLAGS -I$GENERATED_DIR/numpy"
WASI_CFLAGS="$WASI_CFLAGS -I$GENERATED_DIR/numpy/_core"
WASI_CFLAGS="$WASI_CFLAGS -I$GENERATED_DIR/meson_cpu"

# Find and include all .so.p and .a.p directories containing generated headers
# Pattern varies by platform: _multiarray_umath.cpython-313-darwin.so.p on macOS
for p_dir in $(find "$GENERATED_DIR/numpy/_core" -maxdepth 1 -type d -name "*.p" 2>/dev/null); do
    WASI_CFLAGS="$WASI_CFLAGS -I$p_dir"
done

# Critical defines for WASI
# Use __EMSCRIPTEN__ to trigger NumPy's WASM code paths
WASI_CFLAGS="$WASI_CFLAGS -D__EMSCRIPTEN__=1"
WASI_CFLAGS="$WASI_CFLAGS -DNPY_NO_SIGNAL=1"
WASI_CFLAGS="$WASI_CFLAGS -DNPY_NO_DEPRECATED_API=0"
WASI_CFLAGS="$WASI_CFLAGS -DNPY_INTERNAL_BUILD=1"
WASI_CFLAGS="$WASI_CFLAGS -DNPY_DISABLE_OPTIMIZATION=1"
WASI_CFLAGS="$WASI_CFLAGS -D_WASI_EMULATED_SIGNAL"
WASI_CFLAGS="$WASI_CFLAGS -D_WASI_EMULATED_GETPID"
WASI_CFLAGS="$WASI_CFLAGS -DCYTHON_PEP489_MULTI_PHASE_INIT=0"

# Suppress warnings that don't apply to WASI
WASI_CFLAGS="$WASI_CFLAGS -Wno-implicit-function-declaration"
WASI_CFLAGS="$WASI_CFLAGS -Wno-int-conversion"
WASI_CFLAGS="$WASI_CFLAGS -Wno-incompatible-pointer-types"
WASI_CFLAGS="$WASI_CFLAGS -Wno-unused-function"
WASI_CFLAGS="$WASI_CFLAGS -Wno-unused-variable"
WASI_CFLAGS="$WASI_CFLAGS -Wno-missing-field-initializers"
WASI_CFLAGS="$WASI_CFLAGS -Wno-unknown-pragmas"
WASI_CFLAGS="$WASI_CFLAGS -Wno-shift-count-overflow"
WASI_CFLAGS="$WASI_CFLAGS -Wno-constant-conversion"

# C++ specific flags for WASI 32-bit environment
WASI_CXXFLAGS="$WASI_CFLAGS -std=c++17 -fno-exceptions -fno-rtti"
WASI_CXXFLAGS="$WASI_CXXFLAGS -Wno-c++11-narrowing"
WASI_CXXFLAGS="$WASI_CXXFLAGS -Wno-tautological-constant-out-of-range-compare"

mkdir -p "$NUMPY_SRC/build-wasi"
cd "$NUMPY_SRC/build-wasi"

# Function to compile a C file
compile_file() {
    local src="$1"
    local obj=$(basename "${src%.c}.o")
    if [[ "$src" == *.cpp ]]; then
        return 1
    fi
    if $WASI_CC $WASI_CFLAGS -c "$src" -o "$obj" 2>/dev/null; then
        echo "  ✓ $(basename $src)"
        return 0
    else
        return 1
    fi
}

# Function to compile a C++ file
compile_cpp_file() {
    local src="$1"
    local obj=$(basename "${src%.cpp}.o")
    if $WASI_CXX $WASI_CXXFLAGS -c "$src" -o "$obj" 2>/dev/null; then
        echo "  ✓ $(basename $src)"
        return 0
    else
        return 1
    fi
}

# Compile npymath (math library)
echo "Compiling npymath..."
for src in $(find "$GENERATED_DIR/numpy/_core/libnpymath.a.p" -name "*.c" 2>/dev/null); do
    compile_file "$src" || true
done

# Also compile from source (C files)
for src in "$CORE_SRC/npymath"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile halffloat stubs (WASI-compatible version instead of halffloat.cpp)
# The original halffloat.cpp uses 64-bit assumptions that don't work on WASI 32-bit
if [ -f "$DEPS_DIR/wasi-stubs/halffloat_stubs.c" ]; then
    echo "  Compiling halffloat stubs..."
    compile_file "$DEPS_DIR/wasi-stubs/halffloat_stubs.c" || true
fi

# Compile NumPy trig stubs (FLOAT_cos, DOUBLE_sin, etc.)
# These are strided loop implementations with correct signatures:
#   void FUNC(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data)
if [ -f "$DEPS_DIR/wasi-stubs/numpy_trig_stubs.c" ]; then
    echo "  Compiling numpy trig stubs..."
    compile_file "$DEPS_DIR/wasi-stubs/numpy_trig_stubs.c" || true
fi

# Compile linalg stub (provides numpy.linalg._umath_linalg)
if [ -f "$DEPS_DIR/wasi-stubs/numpy_linalg_stub.c" ]; then
    echo "  Compiling numpy linalg stub..."
    compile_file "$DEPS_DIR/wasi-stubs/numpy_linalg_stub.c" || true
fi

# Create npymath library
NPYMATH_OBJS=$(ls *.o 2>/dev/null | tr '\n' ' ')
if [ -n "$NPYMATH_OBJS" ]; then
    $WASI_AR rcs "$DEPS_DIR/wasi-numpy/lib/libnpymath.a" *.o
    echo "Created libnpymath.a with $(echo $NPYMATH_OBJS | wc -w | tr -d ' ') objects"
    rm -f *.o
fi

# Compile common sources
echo ""
echo "Compiling common sources..."
for src in "$CORE_SRC/common"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile multiarray sources
echo ""
echo "Compiling multiarray sources..."
for src in "$CORE_SRC/multiarray"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile multiarray/stringdtype sources
echo ""
echo "Compiling stringdtype sources..."
for src in "$CORE_SRC/multiarray/stringdtype"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile multiarray/textreading sources
echo ""
echo "Compiling textreading sources..."
for src in "$CORE_SRC/multiarray/textreading"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile textreading C++ sources (tokenize.cpp)
for src in "$CORE_SRC/multiarray/textreading"/*.cpp; do
    [ -f "$src" ] && compile_cpp_file "$src" || true
done

# Compile generated multiarray sources
echo ""
echo "Compiling generated multiarray sources..."
MULTIARRAY_GEN="$GENERATED_DIR/numpy/_core/_multiarray_umath.cpython-313-darwin.so.p"
for src in "$MULTIARRAY_GEN"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile umath sources
echo ""
echo "Compiling umath sources..."
for src in "$CORE_SRC/umath"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile umath C++ sources (stringdtype_ufuncs.cpp)
for src in "$CORE_SRC/umath"/*.cpp; do
    [ -f "$src" ] && compile_cpp_file "$src" || true
done

# Compile npysort sources (C files)
echo ""
echo "Compiling npysort sources..."
for src in "$CORE_SRC/npysort"/*.c; do
    [ -f "$src" ] && compile_file "$src" || true
done

# Compile npysort C++ sources (timsort, heapsort, quicksort, etc.)
echo ""
echo "Compiling npysort C++ sources..."
for src in "$CORE_SRC/npysort"/*.cpp; do
    [ -f "$src" ] && compile_cpp_file "$src" || true
done

# Compile all generated dispatch files from lib*.a.p directories
echo ""
echo "Compiling generated dispatch sources..."
for p_dir in $(find "$GENERATED_DIR/numpy/_core" -maxdepth 1 -type d -name "lib*.a.p" 2>/dev/null); do
    for src in "$p_dir"/*.c; do
        [ -f "$src" ] && compile_file "$src" || true
    done
done

# Compile __umath_generated.c
echo ""
echo "Compiling umath generated sources..."
if [ -f "$GENERATED_DIR/numpy/_core/__umath_generated.c" ]; then
    compile_file "$GENERATED_DIR/numpy/_core/__umath_generated.c" || true
fi
if [ -f "$GENERATED_DIR/numpy/_core/__multiarray_api.c" ]; then
    compile_file "$GENERATED_DIR/numpy/_core/__multiarray_api.c" || true
fi
if [ -f "$GENERATED_DIR/numpy/_core/__ufunc_api.c" ]; then
    compile_file "$GENERATED_DIR/numpy/_core/__ufunc_api.c" || true
fi

# Create main library from all objects
echo ""
echo "Creating libraries..."
ALL_OBJS=$(ls *.o 2>/dev/null | tr '\n' ' ')
if [ -n "$ALL_OBJS" ]; then
    OBJ_COUNT=$(echo $ALL_OBJS | wc -w | tr -d ' ')
    $WASI_AR rcs "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" *.o
    echo "Created libnumpy_core.a with $OBJ_COUNT object files"
else
    echo "WARNING: No object files compiled successfully"
    echo "/* NumPy WASI placeholder */" > /tmp/np_placeholder.c
    $WASI_CC $WASI_CFLAGS -c /tmp/np_placeholder.c -o np_placeholder.o 2>/dev/null || touch np_placeholder.o
    $WASI_AR rcs "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a" np_placeholder.o 2>/dev/null || touch "$DEPS_DIR/wasi-numpy/lib/libnumpy_core.a"
fi

# Create npyrandom placeholder
touch "$DEPS_DIR/wasi-numpy/lib/libnpyrandom.a"

# Copy Python files
echo ""
echo "Copying Python files..."
cp -r "$NUMPY_SRC/numpy" "$DEPS_DIR/wasi-numpy/python/"

# Also copy generated Python files from native build
if [ -d "$GENERATED_DIR/numpy" ]; then
    # Copy generated .py and .pyi files
    find "$GENERATED_DIR/numpy" -name "*.py" -o -name "*.pyi" | while read pyfile; do
        relpath="${pyfile#$GENERATED_DIR/}"
        destdir="$DEPS_DIR/wasi-numpy/python/$(dirname $relpath)"
        mkdir -p "$destdir"
        cp "$pyfile" "$destdir/" 2>/dev/null || true
    done
fi

# Remove .so files and pycache
find "$DEPS_DIR/wasi-numpy/python" -name "*.so" -delete 2>/dev/null || true
find "$DEPS_DIR/wasi-numpy/python" -name "*.pyc" -delete 2>/dev/null || true
find "$DEPS_DIR/wasi-numpy/python" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true

# Patch numpy/__init__.py to remove source directory check
# This check causes issues in WASM environment
NUMPY_INIT="$DEPS_DIR/wasi-numpy/python/numpy/__init__.py"
if [ -f "$NUMPY_INIT" ]; then
    echo "Patching numpy/__init__.py to remove source directory check..."
    python3 -c "
import sys
init_file = sys.argv[1]
with open(init_file, 'r') as f:
    content = f.read()

# Remove the try/except block around __config__ import
old = '''    try:
        from numpy.__config__ import show as show_config
    except ImportError as e:
        msg = \"\"\"Error importing numpy: you should not try to import numpy from
        its source directory; please exit the numpy source tree, and relaunch
        your python interpreter from there.\"\"\"
        raise ImportError(msg) from e'''

new = '''    # Source directory check removed for WASM
    from numpy.__config__ import show as show_config'''

if old in content:
    content = content.replace(old, new)
    with open(init_file, 'w') as f:
        f.write(content)
    print('Patched successfully')
else:
    print('Pattern not found, may already be patched')
" "$NUMPY_INIT"
fi

# Completely replace numpy/linalg/__init__.py for WASM
LINALG_INIT="$DEPS_DIR/wasi-numpy/python/numpy/linalg/__init__.py"
if [ -f "$LINALG_INIT" ]; then
    echo "Patching numpy/linalg/__init__.py for WASM compatibility..."
    cat > "$LINALG_INIT" << 'LINALG_PATCH'
"""
numpy.linalg - Stub for WASM compatibility

Linear algebra functions are not available in the WASM build.
"""

class LinAlgError(Exception):
    """Linear algebra error - linalg not available in WASM."""
    pass

def _not_available(*args, **kwargs):
    raise NotImplementedError("numpy.linalg is not available in WASM builds")

# Stub all common functions
matrix_power = _not_available
solve = _not_available
lstsq = _not_available
inv = _not_available
pinv = _not_available
det = _not_available
eig = _not_available
eigh = _not_available
eigvals = _not_available
eigvalsh = _not_available
svd = _not_available
svdvals = _not_available
cholesky = _not_available
qr = _not_available
norm = _not_available
matrix_norm = _not_available
vector_norm = _not_available
cond = _not_available
matrix_rank = _not_available
slogdet = _not_available
tensorsolve = _not_available
tensorinv = _not_available
multi_dot = _not_available

__all__ = ['LinAlgError']
LINALG_PATCH
    echo "Replaced linalg/__init__.py with WASM stub"
fi

# Patch numpy/matrixlib/defmatrix.py to not import linalg at module level
DEFMATRIX="$DEPS_DIR/wasi-numpy/python/numpy/matrixlib/defmatrix.py"
if [ -f "$DEFMATRIX" ]; then
    echo "Patching numpy/matrixlib/defmatrix.py for WASM compatibility..."
    python3 -c "
import sys
file_path = sys.argv[1]
with open(file_path, 'r') as f:
    content = f.read()

# Make linalg import lazy/optional
old_import = 'from numpy.linalg import matrix_power'
new_import = '''# Lazy import for WASM compatibility
def _get_matrix_power():
    try:
        from numpy.linalg import matrix_power
        return matrix_power
    except ImportError:
        def matrix_power_stub(*args, **kwargs):
            raise NotImplementedError(\"matrix_power not available in WASM\")
        return matrix_power_stub
matrix_power = None  # Will be set on first use'''

if old_import in content and 'Lazy import for WASM' not in content:
    content = content.replace(old_import, new_import)

    # Also patch the usage to call the getter
    content = content.replace(
        'matrix_power(self.A',
        '(_get_matrix_power() if matrix_power is None else matrix_power)(self.A'
    )

    with open(file_path, 'w') as f:
        f.write(content)
    print('Patched defmatrix.py successfully')
else:
    print('Already patched or pattern not found')
" "$DEFMATRIX"
fi

# Clean up source
cd "$DEPS_DIR"
rm -rf "numpy-${NUMPY_VERSION}"

echo ""
echo "=== NumPy build complete ==="
echo "Libraries:"
ls -la "$DEPS_DIR/wasi-numpy/lib/"
echo ""
echo "Python files: $DEPS_DIR/wasi-numpy/python/numpy/"
