#!/bin/bash
# Download and build all WASI dependencies for Python modules
# This script runs during Docker image build
#
# Based on historical scripts from commit dc4c1ad:
# - scripts/download-deps.sh
# - scripts/build-python-wasi.sh
# - scripts/build-lxml-wasi.sh
# - scripts/build-pydantic-wasi.sh

set -e

DEPS_DIR="/wadup/deps"
WASI_SDK_PATH="/opt/wasi-sdk"
PATCHES_DIR="/tmp/patches"
CONFIG_SITE="/tmp/config.site-wasm32-wasi"
BUILD_TMP="/tmp/wasi-build"

# Versions
ZLIB_VERSION="1.3.1"
BZIP2_VERSION="1.0.8"
XZ_VERSION="5.8.2"
SQLITE_VERSION="3510100"
SQLITE_YEAR="2025"
LIBXML2_VERSION="2.13.5"
LIBXSLT_VERSION="1.1.42"
PYTHON_VERSION="3.13.1"
LXML_VERSION="6.0.2"
PYDANTIC_CORE_VERSION="2.41.5"
PYDANTIC_VERSION="2.12.5"
TYPING_EXTENSIONS_VERSION="4.15.0"
ANNOTATED_TYPES_VERSION="0.7.0"
TYPING_INSPECTION_VERSION="0.4.2"

# Create directories
mkdir -p "$DEPS_DIR" "$BUILD_TMP"

# Helper function to download files
download() {
    local url="$1"
    local output="$2"

    if [ -f "$output" ]; then
        echo "  Already downloaded: $(basename "$output")"
        return 0
    fi

    echo "  Downloading: $(basename "$output")"
    curl -L -o "$output" "$url"
}

echo "=== WADUP WASI Dependency Builder ==="
echo "Output directory: $DEPS_DIR"
echo ""

# =============================================================================
# Section 1: C Libraries
# =============================================================================
echo "=== Building C Libraries ==="

CC="$WASI_SDK_PATH/bin/clang"
AR="$WASI_SDK_PATH/bin/ar"
RANLIB="$WASI_SDK_PATH/bin/ranlib"

# --- zlib ---
echo ""
echo "1. Building zlib ${ZLIB_VERSION}..."
download "https://zlib.net/zlib-${ZLIB_VERSION}.tar.gz" "$BUILD_TMP/zlib-${ZLIB_VERSION}.tar.gz"

cd "$BUILD_TMP"
rm -rf "zlib-${ZLIB_VERSION}"
tar xzf "zlib-${ZLIB_VERSION}.tar.gz"
cd "zlib-${ZLIB_VERSION}"

$CC -O2 -D_LARGEFILE64_SOURCE=1 -c adler32.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c crc32.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c deflate.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c infback.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c inffast.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c inflate.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c inftrees.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c trees.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c zutil.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c compress.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c uncompr.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c gzclose.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c gzlib.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c gzread.c
$CC -O2 -D_LARGEFILE64_SOURCE=1 -c gzwrite.c

$AR rcs libz.a adler32.o crc32.o deflate.o infback.o \
    inffast.o inflate.o inftrees.o trees.o zutil.o compress.o uncompr.o \
    gzclose.o gzlib.o gzread.o gzwrite.o

mkdir -p "$DEPS_DIR/wasi-zlib/include" "$DEPS_DIR/wasi-zlib/lib"
cp libz.a "$DEPS_DIR/wasi-zlib/lib/"
cp zlib.h zconf.h "$DEPS_DIR/wasi-zlib/include/"
echo "  zlib built successfully"

# --- bzip2 ---
echo ""
echo "2. Building bzip2 ${BZIP2_VERSION}..."
download "https://sourceware.org/pub/bzip2/bzip2-${BZIP2_VERSION}.tar.gz" "$BUILD_TMP/bzip2-${BZIP2_VERSION}.tar.gz"

cd "$BUILD_TMP"
rm -rf "bzip2-${BZIP2_VERSION}"
tar xzf "bzip2-${BZIP2_VERSION}.tar.gz"
cd "bzip2-${BZIP2_VERSION}"

$CC -O2 -D_FILE_OFFSET_BITS=64 -c blocksort.c
$CC -O2 -D_FILE_OFFSET_BITS=64 -c huffman.c
$CC -O2 -D_FILE_OFFSET_BITS=64 -c crctable.c
$CC -O2 -D_FILE_OFFSET_BITS=64 -c randtable.c
$CC -O2 -D_FILE_OFFSET_BITS=64 -c compress.c
$CC -O2 -D_FILE_OFFSET_BITS=64 -c decompress.c
$CC -O2 -D_FILE_OFFSET_BITS=64 -c bzlib.c

$AR rcs libbz2.a blocksort.o huffman.o crctable.o \
    randtable.o compress.o decompress.o bzlib.o

mkdir -p "$DEPS_DIR/wasi-bzip2/include" "$DEPS_DIR/wasi-bzip2/lib"
cp bzlib.h "$DEPS_DIR/wasi-bzip2/include/"
cp libbz2.a "$DEPS_DIR/wasi-bzip2/lib/"
echo "  bzip2 built successfully"

# --- liblzma (xz-utils) ---
echo ""
echo "3. Building liblzma ${XZ_VERSION}..."
download "https://github.com/tukaani-project/xz/releases/download/v${XZ_VERSION}/xz-${XZ_VERSION}.tar.gz" "$BUILD_TMP/xz-${XZ_VERSION}.tar.gz"

cd "$BUILD_TMP"
rm -rf "xz-${XZ_VERSION}"
tar xzf "xz-${XZ_VERSION}.tar.gz"
cd "xz-${XZ_VERSION}"

CC="$WASI_SDK_PATH/bin/clang" \
AR="$WASI_SDK_PATH/bin/ar" \
RANLIB="$WASI_SDK_PATH/bin/ranlib" \
./configure \
    --host=wasm32-wasi \
    --prefix="$DEPS_DIR/wasi-xz" \
    --disable-shared \
    --enable-static \
    --disable-threads \
    --disable-xz \
    --disable-xzdec \
    --disable-lzmadec \
    --disable-lzmainfo \
    --disable-scripts \
    --disable-doc \
    > /dev/null 2>&1

make -j $(nproc) > /dev/null 2>&1
make install > /dev/null 2>&1
echo "  liblzma built successfully"

# --- SQLite ---
echo ""
echo "4. Building SQLite ${SQLITE_VERSION}..."
download "https://www.sqlite.org/${SQLITE_YEAR}/sqlite-amalgamation-${SQLITE_VERSION}.zip" "$BUILD_TMP/sqlite-amalgamation-${SQLITE_VERSION}.zip"

cd "$BUILD_TMP"
rm -rf "sqlite-amalgamation-${SQLITE_VERSION}"
unzip -o -q "sqlite-amalgamation-${SQLITE_VERSION}.zip"
cd "sqlite-amalgamation-${SQLITE_VERSION}"

$CC -c sqlite3.c -o sqlite3.o \
    -O2 \
    -DSQLITE_OMIT_LOAD_EXTENSION=1 \
    -DSQLITE_THREADSAFE=0 \
    -DSQLITE_ENABLE_FTS5=1 \
    -DSQLITE_ENABLE_JSON1=1

$AR rcs libsqlite3.a sqlite3.o

mkdir -p "$DEPS_DIR/wasi-sqlite/include" "$DEPS_DIR/wasi-sqlite/lib"
cp libsqlite3.a "$DEPS_DIR/wasi-sqlite/lib/"
cp sqlite3.h sqlite3ext.h "$DEPS_DIR/wasi-sqlite/include/"
echo "  SQLite built successfully"

# --- libxml2 ---
echo ""
echo "5. Building libxml2 ${LIBXML2_VERSION}..."
download "https://download.gnome.org/sources/libxml2/2.13/libxml2-${LIBXML2_VERSION}.tar.xz" "$BUILD_TMP/libxml2-${LIBXML2_VERSION}.tar.xz"

cd "$BUILD_TMP"
rm -rf "libxml2-${LIBXML2_VERSION}"
tar xJf "libxml2-${LIBXML2_VERSION}.tar.xz"
cd "libxml2-${LIBXML2_VERSION}"

# Patch xmlIO.c to work around missing dup() in WASI
python3 -c '
import sys
with open("xmlIO.c", "r") as f:
    content = f.read()

old_code = """    if (!strcmp(filename, "-")) {
        fd = dup(STDOUT_FILENO);

        if (fd < 0)
            return(xmlIOErr(0, "dup()"));
    }"""

new_code = """    if (!strcmp(filename, "-")) {
#ifdef __wasi__
        return(xmlIOErr(0, "stdout not supported in WASI"));
#else
        fd = dup(STDOUT_FILENO);

        if (fd < 0)
            return(xmlIOErr(0, "dup()"));
#endif
    }"""

if old_code in content:
    content = content.replace(old_code, new_code)
    with open("xmlIO.c", "w") as f:
        f.write(content)
    print("  Patched xmlIO.c for WASI compatibility")
'

CC="$WASI_SDK_PATH/bin/clang" \
AR="$WASI_SDK_PATH/bin/ar" \
RANLIB="$WASI_SDK_PATH/bin/ranlib" \
CFLAGS="-O2 -I$DEPS_DIR/wasi-zlib/include" \
LDFLAGS="-L$DEPS_DIR/wasi-zlib/lib" \
./configure \
    --host=wasm32-wasi \
    --prefix="$DEPS_DIR/wasi-libxml2" \
    --disable-shared \
    --enable-static \
    --without-http \
    --without-ftp \
    --without-threads \
    --without-thread-alloc \
    --without-modules \
    --without-python \
    --without-iconv \
    --without-icu \
    --without-readline \
    --without-history \
    --without-debug \
    --without-legacy \
    --with-zlib="$DEPS_DIR/wasi-zlib" \
    > /dev/null 2>&1

make -j $(nproc) > /dev/null 2>&1
make install > /dev/null 2>&1
echo "  libxml2 built successfully"

# --- libxslt ---
echo ""
echo "6. Building libxslt ${LIBXSLT_VERSION}..."
download "https://download.gnome.org/sources/libxslt/1.1/libxslt-${LIBXSLT_VERSION}.tar.xz" "$BUILD_TMP/libxslt-${LIBXSLT_VERSION}.tar.xz"

cd "$BUILD_TMP"
rm -rf "libxslt-${LIBXSLT_VERSION}"
tar xJf "libxslt-${LIBXSLT_VERSION}.tar.xz"
cd "libxslt-${LIBXSLT_VERSION}"

CC="$WASI_SDK_PATH/bin/clang" \
AR="$WASI_SDK_PATH/bin/ar" \
RANLIB="$WASI_SDK_PATH/bin/ranlib" \
CFLAGS="-O2 -I$DEPS_DIR/wasi-libxml2/include/libxml2" \
LDFLAGS="-L$DEPS_DIR/wasi-libxml2/lib" \
./configure \
    --host=wasm32-wasi \
    --prefix="$DEPS_DIR/wasi-libxslt" \
    --disable-shared \
    --enable-static \
    --without-python \
    --without-crypto \
    --without-plugins \
    --without-debug \
    --with-libxml-prefix="$DEPS_DIR/wasi-libxml2" \
    > /dev/null 2>&1

make -j $(nproc) > /dev/null 2>&1
make install > /dev/null 2>&1
echo "  libxslt built successfully"

# =============================================================================
# Section 2: CPython
# =============================================================================
echo ""
echo "=== Building CPython ${PYTHON_VERSION} ==="

download "https://www.python.org/ftp/python/${PYTHON_VERSION}/Python-${PYTHON_VERSION}.tar.xz" "$BUILD_TMP/Python-${PYTHON_VERSION}.tar.xz"

cd "$BUILD_TMP"
rm -rf "Python-${PYTHON_VERSION}"
tar xf "Python-${PYTHON_VERSION}.tar.xz"
cd "Python-${PYTHON_VERSION}"

# Apply WASI-specific patches
echo "  Applying WASI patches..."
patch -p1 < "$PATCHES_DIR/cpython-wasi-threading.patch"
patch -p1 < "$PATCHES_DIR/cpython-wasi-gilstate.patch"

# Enable frozen stdlib modules
echo "  Enabling frozen stdlib modules..."
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
        'glob',
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
        'tempfile',
        'textwrap',
        'threading',
        '<tomllib.*>',
        'traceback',
        'types',
        'typing',
        '<urllib.*>',
        'uuid',
        'warnings',
        'weakref',
        '_weakrefset',
        '<xml.*>',
        '<zipfile.*>',
        '<zipfile._path.*>',
        '<zoneinfo.*>',
    ]),
"""

    content = content.replace(
        "    (TESTS_SECTION, [",
        stdlib_section + "    (TESTS_SECTION, ["
    )

    with open('Tools/build/freeze_modules.py', 'w') as f:
        f.write(content)

    print("  Modified freeze_modules.py successfully")
else:
    print("  freeze_modules.py already contains comprehensive stdlib modules")
PYEOF

# Regenerate frozen modules
echo "  Regenerating frozen modules..."
python3 Tools/build/freeze_modules.py

# Create Setup.local for builtin modules
cat > Modules/Setup.local << EOF
# Force modules to be built in (statically linked)
*static*

_sqlite3 _sqlite/blob.c _sqlite/connection.c _sqlite/cursor.c _sqlite/microprotocols.c _sqlite/module.c _sqlite/prepare_protocol.c _sqlite/row.c _sqlite/statement.c _sqlite/util.c -I\$(srcdir)/Modules/_sqlite -DMODULE_NAME='"sqlite3"' -DSQLITE_OMIT_LOAD_EXTENSION=1

# Compression modules
zlib zlibmodule.c -I${DEPS_DIR}/wasi-zlib/include -L${DEPS_DIR}/wasi-zlib/lib -lz
_bz2 _bz2module.c -I${DEPS_DIR}/wasi-bzip2/include -L${DEPS_DIR}/wasi-bzip2/lib -lbz2
_lzma _lzmamodule.c -I${DEPS_DIR}/wasi-xz/include -L${DEPS_DIR}/wasi-xz/lib -llzma
EOF

# Build native Python first (for cross-compilation)
echo "  Building native Python for cross-compilation..."
mkdir -p builddir/build
cd builddir/build
../../configure -C > /dev/null 2>&1
make -s -j $(nproc) all
PYTHON_VERSION_SHORT=$(./python -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
BUILD_PYTHON_PATH=$(pwd)/python
cd ../..

# Build WASI Python
echo "  Building WASI Python..."
export CONFIG_SITE="$CONFIG_SITE"
export WASI_SDK_PATH="${WASI_SDK_PATH}"
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

make -s -j $(nproc) all

# Install CPython
echo "  Installing CPython..."
mkdir -p "${DEPS_DIR}/wasi-python/lib"
mkdir -p "${DEPS_DIR}/wasi-python/include"

cp libpython${PYTHON_VERSION_SHORT}.a "${DEPS_DIR}/wasi-python/lib/"

# Copy additional module libraries
if [ -f "Modules/_decimal/libmpdec/libmpdec.a" ]; then
    cp Modules/_decimal/libmpdec/libmpdec.a "${DEPS_DIR}/wasi-python/lib/"
fi
if [ -f "Modules/expat/libexpat.a" ]; then
    cp Modules/expat/libexpat.a "${DEPS_DIR}/wasi-python/lib/"
fi
if [ -d "Modules/_hacl" ]; then
    find Modules/_hacl -name "*.a" -exec cp {} "${DEPS_DIR}/wasi-python/lib/" \;
fi

# Copy SQLite library
cp "${DEPS_DIR}/wasi-sqlite/lib/libsqlite3.a" "${DEPS_DIR}/wasi-python/lib/"

# Copy headers
cp -r ../../Include/* "${DEPS_DIR}/wasi-python/include/"
cp pyconfig.h "${DEPS_DIR}/wasi-python/include/"

echo "  CPython built successfully"

# =============================================================================
# Section 3: lxml
# =============================================================================
echo ""
echo "=== Building lxml ${LXML_VERSION} ==="

download "https://files.pythonhosted.org/packages/source/l/lxml/lxml-${LXML_VERSION}.tar.gz" "$BUILD_TMP/lxml-${LXML_VERSION}.tar.gz"

cd "$BUILD_TMP"
rm -rf "lxml-${LXML_VERSION}"
tar xzf "lxml-${LXML_VERSION}.tar.gz"
cd "lxml-${LXML_VERSION}"

PYTHON_INCLUDE="$DEPS_DIR/wasi-python/include"
LIBXML2_INCLUDE="$DEPS_DIR/wasi-libxml2/include/libxml2"
LIBXSLT_INCLUDE="$DEPS_DIR/wasi-libxslt/include"
LXML_INCLUDE="$BUILD_TMP/lxml-${LXML_VERSION}/src/lxml/includes"

CFLAGS="-O2 -fPIC"
CFLAGS="$CFLAGS -I$PYTHON_INCLUDE"
CFLAGS="$CFLAGS -I$LIBXML2_INCLUDE"
CFLAGS="$CFLAGS -I$LIBXSLT_INCLUDE"
CFLAGS="$CFLAGS -I$LXML_INCLUDE"
CFLAGS="$CFLAGS -I$BUILD_TMP/lxml-${LXML_VERSION}/src/lxml"
CFLAGS="$CFLAGS -DCYTHON_PEP489_MULTI_PHASE_INIT=0"
CFLAGS="$CFLAGS -DLIBXML_STATIC"
CFLAGS="$CFLAGS -DLIBXSLT_STATIC"

echo "  Compiling lxml.etree..."
$CC $CFLAGS -c src/lxml/etree.c -o etree.o 2>&1

echo "  Creating static library..."
$AR rcs liblxml_etree.a etree.o

mkdir -p "$DEPS_DIR/wasi-lxml/lib"
mkdir -p "$DEPS_DIR/wasi-lxml/python/lxml"
cp liblxml_etree.a "$DEPS_DIR/wasi-lxml/lib/"
cp src/lxml/__init__.py "$DEPS_DIR/wasi-lxml/python/lxml/"
cp src/lxml/_elementpath.py "$DEPS_DIR/wasi-lxml/python/lxml/"
echo "  lxml built successfully"

# =============================================================================
# Section 4: pydantic
# =============================================================================
echo ""
echo "=== Building pydantic_core ${PYDANTIC_CORE_VERSION} ==="

# Install typing_extensions for pydantic_core build.rs
pip3 install typing_extensions==${TYPING_EXTENSIONS_VERSION} --quiet

# Download all pydantic-related packages
download "https://files.pythonhosted.org/packages/source/p/pydantic_core/pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz" "$BUILD_TMP/pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
download "https://files.pythonhosted.org/packages/source/p/pydantic/pydantic-${PYDANTIC_VERSION}.tar.gz" "$BUILD_TMP/pydantic-${PYDANTIC_VERSION}.tar.gz"
download "https://files.pythonhosted.org/packages/source/t/typing_extensions/typing_extensions-${TYPING_EXTENSIONS_VERSION}.tar.gz" "$BUILD_TMP/typing_extensions-${TYPING_EXTENSIONS_VERSION}.tar.gz"
download "https://files.pythonhosted.org/packages/source/a/annotated_types/annotated_types-${ANNOTATED_TYPES_VERSION}.tar.gz" "$BUILD_TMP/annotated_types-${ANNOTATED_TYPES_VERSION}.tar.gz"
download "https://files.pythonhosted.org/packages/source/t/typing_inspection/typing_inspection-${TYPING_INSPECTION_VERSION}.tar.gz" "$BUILD_TMP/typing_inspection-${TYPING_INSPECTION_VERSION}.tar.gz"

cd "$BUILD_TMP"
rm -rf "pydantic_core-${PYDANTIC_CORE_VERSION}" "pydantic-${PYDANTIC_VERSION}" "typing_extensions-${TYPING_EXTENSIONS_VERSION}" "annotated_types-${ANNOTATED_TYPES_VERSION}" "typing_inspection-${TYPING_INSPECTION_VERSION}"

tar xzf "pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
tar xzf "pydantic-${PYDANTIC_VERSION}.tar.gz"
tar xzf "typing_extensions-${TYPING_EXTENSIONS_VERSION}.tar.gz"
tar xzf "annotated_types-${ANNOTATED_TYPES_VERSION}.tar.gz"
tar xzf "typing_inspection-${TYPING_INSPECTION_VERSION}.tar.gz"

# Find pydantic_core directory (naming varies)
PYDANTIC_CORE_DIR=""
if [ -d "pydantic_core-${PYDANTIC_CORE_VERSION}" ]; then
    PYDANTIC_CORE_DIR="pydantic_core-${PYDANTIC_CORE_VERSION}"
elif [ -d "pydantic-core-${PYDANTIC_CORE_VERSION}" ]; then
    PYDANTIC_CORE_DIR="pydantic-core-${PYDANTIC_CORE_VERSION}"
else
    echo "ERROR: Could not find pydantic_core source directory"
    exit 1
fi

cd "$PYDANTIC_CORE_DIR"

# Patch Cargo.toml for WASI static linking
echo "  Patching Cargo.toml for WASI..."
cp Cargo.toml Cargo.toml.orig
sed -i 's/"generate-import-lib", //' Cargo.toml
sed -i 's/crate-type = \["cdylib", "rlib"\]/crate-type = ["staticlib", "rlib"]/' Cargo.toml
echo '' >> Cargo.toml
echo '[workspace]' >> Cargo.toml

# Create PyO3 config
cat > pyo3-wasi-config.txt << EOF
implementation=CPython
version=3.13
shared=false
abi3=false
lib_name=python3.13
lib_dir=$DEPS_DIR/wasi-python/lib
pointer_width=32
build_flags=
suppress_build_script_link_lines=true
EOF

echo "  Building Rust library for wasm32-wasip1..."
export PYO3_CONFIG_FILE="$(pwd)/pyo3-wasi-config.txt"
export CARGO_TARGET_WASM32_WASIP1_LINKER="${WASI_SDK_PATH}/bin/wasm-ld"

cargo build --target wasm32-wasip1 --release 2>&1 | tail -20

if [ ! -f "target/wasm32-wasip1/release/lib_pydantic_core.a" ]; then
    echo "ERROR: Rust build failed"
    exit 1
fi

# Create pydantic output directories
mkdir -p "$DEPS_DIR/wasi-pydantic/lib"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/pydantic_core"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/pydantic"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/annotated_types"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/typing_inspection"

cp "target/wasm32-wasip1/release/lib_pydantic_core.a" "$DEPS_DIR/wasi-pydantic/lib/"

# Copy Python files
echo "  Copying Python packages..."
cp python/pydantic_core/__init__.py "$DEPS_DIR/wasi-pydantic/python/pydantic_core/"
cp python/pydantic_core/core_schema.py "$DEPS_DIR/wasi-pydantic/python/pydantic_core/"

# Create pydantic_core __init__.py with imports
cat > "$DEPS_DIR/wasi-pydantic/python/pydantic_core/__init__.py" << 'PYEOF'
"""pydantic_core - Core validation library for Pydantic V2."""
from __future__ import annotations

import sys as _sys
from typing import Any as _Any

from typing_extensions import Sentinel

from _pydantic_core import (
    ArgsKwargs,
    MultiHostUrl,
    PydanticCustomError,
    PydanticKnownError,
    PydanticOmit,
    PydanticSerializationError,
    PydanticSerializationUnexpectedValue,
    PydanticUndefined,
    PydanticUndefinedType,
    PydanticUseDefault,
    SchemaError,
    SchemaSerializer,
    SchemaValidator,
    Some,
    TzInfo,
    Url,
    ValidationError,
    __version__,
    from_json,
    to_json,
    to_jsonable_python,
)

# Import core_schema for full pydantic compatibility
from . import core_schema
from .core_schema import CoreConfig, CoreSchema, CoreSchemaType, ErrorType

if _sys.version_info < (3, 11):
    from typing_extensions import NotRequired as _NotRequired
else:
    from typing import NotRequired as _NotRequired

if _sys.version_info < (3, 12):
    from typing_extensions import TypedDict as _TypedDict
else:
    from typing import TypedDict as _TypedDict

# Sentinel values
UNSET: Sentinel = Sentinel('UNSET')
MISSING: Sentinel = Sentinel('MISSING')

__all__ = [
    '__version__',
    'MISSING',
    'UNSET',
    'CoreConfig',
    'CoreSchema',
    'CoreSchemaType',
    'SchemaValidator',
    'SchemaSerializer',
    'Some',
    'Url',
    'MultiHostUrl',
    'ArgsKwargs',
    'PydanticUndefined',
    'PydanticUndefinedType',
    'SchemaError',
    'ErrorDetails',
    'InitErrorDetails',
    'ValidationError',
    'PydanticCustomError',
    'PydanticKnownError',
    'PydanticOmit',
    'PydanticUseDefault',
    'PydanticSerializationError',
    'PydanticSerializationUnexpectedValue',
    'TzInfo',
    'to_json',
    'from_json',
    'to_jsonable_python',
    'core_schema',
]


class ErrorDetails(_TypedDict):
    type: str
    loc: tuple[int | str, ...]
    msg: str
    input: _Any
    ctx: _NotRequired[dict[str, _Any]]
    url: _NotRequired[str]


class InitErrorDetails(_TypedDict):
    type: str | PydanticCustomError
    loc: _NotRequired[tuple[int | str, ...]]
    input: _Any
    ctx: _NotRequired[dict[str, _Any]]
PYEOF

# Copy pydantic package
cd "$BUILD_TMP/pydantic-${PYDANTIC_VERSION}"
cp -r pydantic/* "$DEPS_DIR/wasi-pydantic/python/pydantic/"

# Copy typing_extensions
cd "$BUILD_TMP/typing_extensions-${TYPING_EXTENSIONS_VERSION}"
cp src/typing_extensions.py "$DEPS_DIR/wasi-pydantic/python/"

# Copy annotated_types (note: has annotated_types/ directly, not src/)
cd "$BUILD_TMP/annotated_types-${ANNOTATED_TYPES_VERSION}"
cp annotated_types/__init__.py "$DEPS_DIR/wasi-pydantic/python/annotated_types/"
cp annotated_types/py.typed "$DEPS_DIR/wasi-pydantic/python/annotated_types/" 2>/dev/null || true
cp annotated_types/test_cases.py "$DEPS_DIR/wasi-pydantic/python/annotated_types/" 2>/dev/null || true

# Copy typing_inspection
cd "$BUILD_TMP/typing_inspection-${TYPING_INSPECTION_VERSION}"
cp -r src/typing_inspection/* "$DEPS_DIR/wasi-pydantic/python/typing_inspection/"

echo "  pydantic built successfully"

# =============================================================================
# Cleanup
# =============================================================================
echo ""
echo "=== Cleaning up ==="
rm -rf "$BUILD_TMP"
echo "  Cleanup complete"

echo ""
echo "=== All dependencies built successfully ==="
echo "Output directory: $DEPS_DIR"
ls -la "$DEPS_DIR"
