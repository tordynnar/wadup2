#!/bin/bash
set -e

PYTHON_VERSION="3.13.7"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_DIR="${SCRIPT_DIR}/python-wasi"

# Detect platform for WASI SDK
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" = "darwin" ]; then
    WASI_SDK_OS="macos"
elif [ "$OS" = "linux" ]; then
    WASI_SDK_OS="linux"
else
    echo "Unsupported OS: $OS"
    exit 1
fi

WASI_SDK_VERSION="24.0"
WASI_SDK_NAME="wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"
WASI_SDK_PATH="/tmp/${WASI_SDK_NAME}"

# Download WASI SDK if needed
if [ ! -d "$WASI_SDK_PATH" ]; then
    echo "Downloading WASI SDK ${WASI_SDK_VERSION}..."
    cd /tmp

    # Check if wget or curl is available
    if command -v wget &> /dev/null; then
        wget "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_SDK_VERSION}/${WASI_SDK_NAME}.tar.gz"
    elif command -v curl &> /dev/null; then
        curl -L -O "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_SDK_VERSION}/${WASI_SDK_NAME}.tar.gz"
    else
        echo "ERROR: Neither wget nor curl found. Please install one of them."
        exit 1
    fi

    tar xzf "${WASI_SDK_NAME}.tar.gz"
    echo "✓ WASI SDK downloaded and extracted to ${WASI_SDK_PATH}"
    cd "$SCRIPT_DIR"
fi

# Check if Python is already built
if [ -f "${PYTHON_DIR}/lib/libpython3.13.a" ]; then
    echo "✓ CPython already built at ${PYTHON_DIR}"
    exit 0
fi

# Build SQLite for WASI first
echo "Building SQLite for WASI..."
cd /tmp

SQLITE_VERSION="3450100"  # SQLite 3.45.1
SQLITE_YEAR="2024"
SQLITE_TARBALL="sqlite-amalgamation-${SQLITE_VERSION}.zip"

if [ ! -f "sqlite-wasi/libsqlite3.a" ]; then
    if [ ! -f "$SQLITE_TARBALL" ]; then
        echo "Downloading SQLite ${SQLITE_VERSION}..."
        if command -v wget &> /dev/null; then
            wget "https://www.sqlite.org/${SQLITE_YEAR}/${SQLITE_TARBALL}"
        elif command -v curl &> /dev/null; then
            curl -L -O "https://www.sqlite.org/${SQLITE_YEAR}/${SQLITE_TARBALL}"
        else
            echo "ERROR: Neither wget nor curl found."
            exit 1
        fi
    fi

    unzip -o -q "$SQLITE_TARBALL"
    cd "sqlite-amalgamation-${SQLITE_VERSION}"

    # Compile SQLite for WASI
    ${WASI_SDK_PATH}/bin/clang \
        -c sqlite3.c \
        -o sqlite3.o \
        -O2 \
        -DSQLITE_OMIT_LOAD_EXTENSION=1 \
        -DSQLITE_THREADSAFE=0 \
        -DSQLITE_ENABLE_FTS5=1 \
        -DSQLITE_ENABLE_JSON1=1

    # Create static library
    ${WASI_SDK_PATH}/bin/ar rcs libsqlite3.a sqlite3.o

    # Install to /tmp/sqlite-wasi
    mkdir -p /tmp/sqlite-wasi/include /tmp/sqlite-wasi/lib
    cp libsqlite3.a /tmp/sqlite-wasi/lib/
    cp sqlite3.h sqlite3ext.h /tmp/sqlite-wasi/include/

    echo "✓ SQLite built for WASI"
    cd /tmp
fi

# Download and build CPython
echo "Building CPython ${PYTHON_VERSION} for WASI..."

# Download CPython
cd /tmp
PYTHON_TARBALL="Python-${PYTHON_VERSION}.tar.xz"
if [ ! -f "$PYTHON_TARBALL" ]; then
    echo "Downloading CPython ${PYTHON_VERSION}..."
    if command -v wget &> /dev/null; then
        wget "https://www.python.org/ftp/python/${PYTHON_VERSION}/${PYTHON_TARBALL}"
    elif command -v curl &> /dev/null; then
        curl -L -O "https://www.python.org/ftp/python/${PYTHON_VERSION}/${PYTHON_TARBALL}"
    else
        echo "ERROR: Neither wget nor curl found."
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

# Add comprehensive stdlib modules for sqlite3 and dependencies
# Use <package.*> for package directories, plain names for single .py files
stdlib_section = """    ('stdlib - comprehensive for sqlite3', [
        'functools',
        '<collections.*>',
        'operator',
        'keyword',
        'heapq',
        'reprlib',
        'weakref',
        'datetime',
        'warnings',
        'types',
        'enum',
        'copy',
        '<re.*>',
        'sre_compile',
        'sre_parse',
        'sre_constants',
        'contextlib',
        'traceback',
        'linecache',
        '<sqlite3.*>',
    ]),
"""

# Find TESTS_SECTION and insert before it
content = content.replace(
    "    (TESTS_SECTION, [",
    stdlib_section + "    (TESTS_SECTION, ["
)

with open('Tools/build/freeze_modules.py', 'w') as f:
    f.write(content)

print("Modified freeze_modules.py successfully")
PYEOF

# Regenerate frozen modules
echo "Regenerating frozen modules..."
python3 Tools/build/freeze_modules.py

# Create Setup.local to force sqlite3 to be built as a builtin module
# This embeds sqlite3 directly into libpython
cat > Modules/Setup.local << 'EOF'
# Force sqlite3 to be built in (statically linked)
*static*

_sqlite3 _sqlite/blob.c _sqlite/connection.c _sqlite/cursor.c _sqlite/microprotocols.c _sqlite/module.c _sqlite/prepare_protocol.c _sqlite/row.c _sqlite/statement.c _sqlite/util.c -I$(srcdir)/Modules/_sqlite -DMODULE_NAME='"sqlite3"' -DSQLITE_OMIT_LOAD_EXTENSION=1
EOF

echo "Building native Python for cross-compilation..."
mkdir -p builddir/build
cd builddir/build
../../configure -C
make -s -j $(sysctl -n hw.ncpu 2>/dev/null || echo 4) all
PYTHON_VERSION_SHORT=$(./python.exe -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
BUILD_PYTHON_PATH=$(pwd)/python.exe
cd ../..

echo "Building WASI Python..."
export CONFIG_SITE="$(pwd)/Tools/wasm/config.site-wasm32-wasi"
export WASI_SDK_PATH="${WASI_SDK_PATH}"

# Point Python configure to SQLite WASI build
export CPPFLAGS="-I/tmp/sqlite-wasi/include"
export LDFLAGS="-L/tmp/sqlite-wasi/lib"

mkdir -p builddir/wasi
cd builddir/wasi

../../Tools/wasm/wasi-env \
    ../../configure \
        -C \
        --host=wasm32-unknown-wasi \
        --build=$(../../config.guess) \
        --with-build-python=${BUILD_PYTHON_PATH}

make -s -j $(sysctl -n hw.ncpu 2>/dev/null || echo 4) all

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

# Copy SQLite library
if [ -f "/tmp/sqlite-wasi/lib/libsqlite3.a" ]; then
    cp /tmp/sqlite-wasi/lib/libsqlite3.a "${PYTHON_DIR}/lib/"
    echo "  Copied libsqlite3.a"
fi

# Copy headers
cp -r ../../Include/* "${PYTHON_DIR}/include/"
cp pyconfig.h "${PYTHON_DIR}/include/"

# Get back to source root
cd ../..

echo "✓ CPython built successfully at ${PYTHON_DIR}"

# Clean up source
cd /tmp
rm -rf "Python-${PYTHON_VERSION}"

echo ""
echo "===================================================="
echo "CPython WASI build complete!"
echo "===================================================="
echo "Python library: ${PYTHON_DIR}/lib/libpython3.13.a"
echo "Python headers: ${PYTHON_DIR}/include/python3.13/"
echo ""
