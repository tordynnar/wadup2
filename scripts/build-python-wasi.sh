#!/bin/bash
# Build CPython for WASI
# Dependencies are managed by download-deps.sh and stored in deps/

set -e

PYTHON_VERSION="3.13.7"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PYTHON_DIR="$WADUP_ROOT/build/python-wasi"
DEPS_DIR="$WADUP_ROOT/deps"

# Check if Python is already built
if [ -f "${PYTHON_DIR}/lib/libpython3.13.a" ]; then
    echo "CPython already built at ${PYTHON_DIR}"
    exit 0
fi

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

WASI_SDK_VERSION="29.0"
WASI_SDK_NAME="wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"
WASI_SDK_PATH="$DEPS_DIR/$WASI_SDK_NAME"

# Ensure dependencies are downloaded
echo "Checking dependencies..."
"$SCRIPT_DIR/download-deps.sh"

# Verify dependencies exist
if [ ! -d "$WASI_SDK_PATH" ]; then
    echo "ERROR: WASI SDK not found at $WASI_SDK_PATH"
    exit 1
fi

# Python build uses /tmp for temporary files, but final libs come from deps/
BUILD_TMP="/tmp/wadup-python-build-$$"
mkdir -p "$BUILD_TMP"

# Download and build CPython
echo ""
echo "=== Building CPython ${PYTHON_VERSION} for WASI ==="

# Download CPython
cd "$BUILD_TMP"
PYTHON_TARBALL="Python-${PYTHON_VERSION}.tar.xz"
if [ ! -f "$PYTHON_TARBALL" ]; then
    echo "Downloading CPython ${PYTHON_VERSION}..."
    if command -v curl &> /dev/null; then
        curl -L -O "https://www.python.org/ftp/python/${PYTHON_VERSION}/${PYTHON_TARBALL}"
    elif command -v wget &> /dev/null; then
        wget "https://www.python.org/ftp/python/${PYTHON_VERSION}/${PYTHON_TARBALL}"
    else
        echo "ERROR: Neither curl nor wget found."
        exit 1
    fi
fi

tar xf "$PYTHON_TARBALL"
cd "Python-${PYTHON_VERSION}"

# Enable frozen stdlib modules (required for WASI without filesystem)
echo "Enabling frozen stdlib modules..."

# Create a Python script to modify freeze_modules.py
python3 << 'PYEOF'
import re

with open('Tools/build/freeze_modules.py', 'r') as f:
    content = f.read()

# Uncomment encodings
content = content.replace("#'<encodings.*>',", "        '<encodings.*>',")

# Add comprehensive stdlib modules (only if not already added)
if "'stdlib - comprehensive'" not in content:
    stdlib_section = """    ('stdlib - comprehensive', [
        '__future__',
        '_colorize',
        '_compression',
        'base64',
        'bisect',
        'bz2',
        'calendar',
        '<collections.*>',
        'contextlib',
        'contextvars',
        'copy',
        'copyreg',
        'csv',
        'dataclasses',
        'datetime',
        'decimal',
        'difflib',
        '<email.*>',
        'enum',
        'fnmatch',
        'functools',
        'gettext',
        'dis',
        'inspect',
        'opcode',
        '_opcode_metadata',
        'token',
        'tokenize',
        'gzip',
        'hashlib',
        'heapq',
        'hmac',
        '<html.*>',
        '<importlib.*>',
        'ipaddress',
        '<json.*>',
        'keyword',
        'linecache',
        'locale',
        '<logging.*>',
        'lzma',
        'mimetypes',
        'numbers',
        'operator',
        '<pathlib.*>',
        'pickle',
        '_compat_pickle',
        'pkgutil',
        'fractions',
        '_threading_local',
        'pprint',
        'ast',
        'platform',
        'socket',
        'queue',
        'random',
        '<re.*>',
        'reprlib',
        'shlex',
        'shutil',
        'sre_compile',
        'sre_parse',
        'sre_constants',
        '_strptime',
        '<sqlite3.*>',
        'statistics',
        'string',
        'stringprep',
        'struct',
        '<sysconfig.*>',
        'tarfile',
        'textwrap',
        'threading',
        '<tomllib.*>',
        'traceback',
        'types',
        'typing',
        'urllib.parse',
        'uuid',
        'warnings',
        'weakref',
        '_weakrefset',
        '<xml.*>',
        '<zipfile.*>',
        '<zoneinfo.*>',
    ]),
"""

    content = content.replace(
        "    (TESTS_SECTION, [",
        stdlib_section + "    (TESTS_SECTION, ["
    )

    with open('Tools/build/freeze_modules.py', 'w') as f:
        f.write(content)

    print("Modified freeze_modules.py successfully")
else:
    print("freeze_modules.py already contains comprehensive stdlib modules")
PYEOF

# Regenerate frozen modules
echo "Regenerating frozen modules..."
python3 Tools/build/freeze_modules.py

# Create Setup.local to force modules to be built as builtin modules
# Use paths from deps/ folder
cat > Modules/Setup.local << EOF
# Force modules to be built in (statically linked)
*static*

_sqlite3 _sqlite/blob.c _sqlite/connection.c _sqlite/cursor.c _sqlite/microprotocols.c _sqlite/module.c _sqlite/prepare_protocol.c _sqlite/row.c _sqlite/statement.c _sqlite/util.c -I\$(srcdir)/Modules/_sqlite -DMODULE_NAME='"sqlite3"' -DSQLITE_OMIT_LOAD_EXTENSION=1

# Compression modules
zlib zlibmodule.c -I${DEPS_DIR}/wasi-zlib/include -L${DEPS_DIR}/wasi-zlib/lib -lz
_bz2 _bz2module.c -I${DEPS_DIR}/wasi-bzip2/include -L${DEPS_DIR}/wasi-bzip2/lib -lbz2
_lzma _lzmamodule.c -I${DEPS_DIR}/wasi-xz/include -L${DEPS_DIR}/wasi-xz/lib -llzma

# Hash module (uses existing HACL* in Python build)
_hashlib _hashopenssl.c
EOF

echo "Building native Python for cross-compilation..."
mkdir -p builddir/build
cd builddir/build
../../configure -C > /dev/null 2>&1
make -s -j $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) all
PYTHON_VERSION_SHORT=$(./python.exe -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
BUILD_PYTHON_PATH=$(pwd)/python.exe
cd ../..

echo "Building WASI Python..."
# Use our custom config.site which extends the official one with dlopen disabled
export CONFIG_SITE="$SCRIPT_DIR/config.site-wasm32-wasi"
export WASI_SDK_PATH="${WASI_SDK_PATH}"

# Point Python configure to all WASI library builds
export CPPFLAGS="-I${DEPS_DIR}/wasi-sqlite/include -I${DEPS_DIR}/wasi-zlib/include -I${DEPS_DIR}/wasi-bzip2/include -I${DEPS_DIR}/wasi-xz/include"
export LDFLAGS="-L${DEPS_DIR}/wasi-sqlite/lib -L${DEPS_DIR}/wasi-zlib/lib -L${DEPS_DIR}/wasi-bzip2/lib -L${DEPS_DIR}/wasi-xz/lib"

mkdir -p builddir/wasi
cd builddir/wasi

../../Tools/wasm/wasi-env \
    ../../configure \
        -C \
        --host=wasm32-unknown-wasi \
        --build=$(../../config.guess) \
        --with-build-python=${BUILD_PYTHON_PATH} \
        > /dev/null 2>&1

make -s -j $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) all

# Copy build artifacts to PYTHON_DIR
echo "Installing CPython to ${PYTHON_DIR}..."
mkdir -p "${PYTHON_DIR}/lib"
mkdir -p "${PYTHON_DIR}/include"

# Copy static library
cp libpython${PYTHON_VERSION_SHORT}.a "${PYTHON_DIR}/lib/"

# Copy additional module libraries (mpdecimal, etc.)
if [ -f "Modules/_decimal/libmpdec/libmpdec.a" ]; then
    cp Modules/_decimal/libmpdec/libmpdec.a "${PYTHON_DIR}/lib/"
    echo "  Copied libmpdec.a"
fi
if [ -f "Modules/expat/libexpat.a" ]; then
    cp Modules/expat/libexpat.a "${PYTHON_DIR}/lib/"
    echo "  Copied libexpat.a"
fi
if [ -d "Modules/_hacl" ]; then
    find Modules/_hacl -name "*.a" -exec cp {} "${PYTHON_DIR}/lib/" \;
    echo "  Copied HACL libraries"
fi

# Copy SQLite library from deps
if [ -f "${DEPS_DIR}/wasi-sqlite/lib/libsqlite3.a" ]; then
    cp "${DEPS_DIR}/wasi-sqlite/lib/libsqlite3.a" "${PYTHON_DIR}/lib/"
    echo "  Copied libsqlite3.a"
fi

# Copy headers
cp -r ../../Include/* "${PYTHON_DIR}/include/"
cp pyconfig.h "${PYTHON_DIR}/include/"

# Get back to original dir
cd "$WADUP_ROOT"

# Clean up build temp
rm -rf "$BUILD_TMP"

echo ""
echo "===================================================="
echo "CPython WASI build complete!"
echo "===================================================="
echo "Python library: ${PYTHON_DIR}/lib/libpython3.13.a"
echo "Python headers: ${PYTHON_DIR}/include/"
echo ""
