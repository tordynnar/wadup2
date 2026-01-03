#!/bin/bash
# Download and build external dependencies for WADUP
# All dependencies are stored in the 'deps' folder
#
# This script only downloads dependencies - no building is done here.
# Run ./scripts/build-all.sh to build after downloading.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"

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

# Versions
WASI_SDK_VERSION="24.0"
WASI_SDK_MAJOR="${WASI_SDK_VERSION%%.*}"
ZLIB_VERSION="1.3.1"
BZIP2_VERSION="1.0.8"
XZ_VERSION="5.8.2"
SQLITE_VERSION="3510100"
SQLITE_YEAR="2025"
LIBXML2_VERSION="2.13.5"
LIBXSLT_VERSION="1.1.42"
LXML_VERSION="6.0.2"
PYTHON_VERSION="3.13.7"
PYDANTIC_CORE_VERSION="2.41.5"
PYDANTIC_VERSION="2.12.5"
TYPING_EXTENSIONS_VERSION="4.15.0"
ANNOTATED_TYPES_VERSION="0.7.0"
TYPING_INSPECTION_VERSION="0.4.2"

# Create deps directory if it doesn't exist
mkdir -p "$DEPS_DIR"

# Helper function to download files
download() {
    local url="$1"
    local output="$2"

    if [ -f "$output" ]; then
        echo "  Already downloaded: $(basename "$output")"
        return 0
    fi

    echo "  Downloading: $(basename "$output")"
    if command -v curl &> /dev/null; then
        curl -L -o "$output" "$url"
    elif command -v wget &> /dev/null; then
        wget -O "$output" "$url"
    else
        echo "ERROR: Neither curl nor wget found."
        exit 1
    fi
}

echo "=== WADUP Dependency Downloader ==="
echo "Dependencies will be stored in: $DEPS_DIR"
echo ""

# =============================================================================
# Section 1: WASI SDK
# =============================================================================
WASI_SDK_NAME="wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"
WASI_SDK_PATH="$DEPS_DIR/$WASI_SDK_NAME"

echo "1. WASI SDK ${WASI_SDK_VERSION}"
if [ -d "$WASI_SDK_PATH" ]; then
    echo "  Already installed: $WASI_SDK_NAME"
else
    download "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_SDK_MAJOR}/${WASI_SDK_NAME}.tar.gz" "$DEPS_DIR/${WASI_SDK_NAME}.tar.gz"
    echo "  Extracting..."
    tar xzf "$DEPS_DIR/${WASI_SDK_NAME}.tar.gz" -C "$DEPS_DIR"
    echo "  Installed: $WASI_SDK_NAME"
fi

# =============================================================================
# Section 2: C Libraries (downloaded and built)
# =============================================================================
echo ""
echo "2. zlib ${ZLIB_VERSION}"
if [ -f "$DEPS_DIR/wasi-zlib/lib/libz.a" ]; then
    echo "  Already built"
else
    download "https://zlib.net/zlib-${ZLIB_VERSION}.tar.gz" "$DEPS_DIR/zlib-${ZLIB_VERSION}.tar.gz"
    echo "  Building..."

    cd "$DEPS_DIR"
    rm -rf "zlib-${ZLIB_VERSION}"
    tar xzf "zlib-${ZLIB_VERSION}.tar.gz"
    cd "zlib-${ZLIB_VERSION}"

    CC="$WASI_SDK_PATH/bin/clang"
    AR="$WASI_SDK_PATH/bin/ar"

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

    cd "$DEPS_DIR"
    rm -rf "zlib-${ZLIB_VERSION}"
    echo "  Built successfully"
fi

echo ""
echo "3. bzip2 ${BZIP2_VERSION}"
if [ -f "$DEPS_DIR/wasi-bzip2/lib/libbz2.a" ]; then
    echo "  Already built"
else
    download "https://sourceware.org/pub/bzip2/bzip2-${BZIP2_VERSION}.tar.gz" "$DEPS_DIR/bzip2-${BZIP2_VERSION}.tar.gz"
    echo "  Building..."

    cd "$DEPS_DIR"
    rm -rf "bzip2-${BZIP2_VERSION}"
    tar xzf "bzip2-${BZIP2_VERSION}.tar.gz"
    cd "bzip2-${BZIP2_VERSION}"

    CC="$WASI_SDK_PATH/bin/clang"
    AR="$WASI_SDK_PATH/bin/ar"

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

    cd "$DEPS_DIR"
    rm -rf "bzip2-${BZIP2_VERSION}"
    echo "  Built successfully"
fi

echo ""
echo "4. liblzma (xz-utils) ${XZ_VERSION}"
if [ -f "$DEPS_DIR/wasi-xz/lib/liblzma.a" ]; then
    echo "  Already built"
else
    download "https://tukaani.org/xz/xz-${XZ_VERSION}.tar.gz" "$DEPS_DIR/xz-${XZ_VERSION}.tar.gz"
    echo "  Building (this may take a minute)..."

    cd "$DEPS_DIR"
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

    make -j $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) > /dev/null 2>&1
    make install > /dev/null 2>&1

    cd "$DEPS_DIR"
    rm -rf "xz-${XZ_VERSION}"
    echo "  Built successfully"
fi

echo ""
echo "5. SQLite ${SQLITE_VERSION}"
if [ -f "$DEPS_DIR/wasi-sqlite/lib/libsqlite3.a" ]; then
    echo "  Already built"
else
    download "https://www.sqlite.org/${SQLITE_YEAR}/sqlite-amalgamation-${SQLITE_VERSION}.zip" "$DEPS_DIR/sqlite-amalgamation-${SQLITE_VERSION}.zip"
    echo "  Building..."

    cd "$DEPS_DIR"
    rm -rf "sqlite-amalgamation-${SQLITE_VERSION}"
    unzip -o -q "sqlite-amalgamation-${SQLITE_VERSION}.zip"
    cd "sqlite-amalgamation-${SQLITE_VERSION}"

    CC="$WASI_SDK_PATH/bin/clang"
    AR="$WASI_SDK_PATH/bin/ar"

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

    cd "$DEPS_DIR"
    rm -rf "sqlite-amalgamation-${SQLITE_VERSION}"
    echo "  Built successfully"
fi

echo ""
echo "6. libxml2 ${LIBXML2_VERSION}"
if [ -f "$DEPS_DIR/wasi-libxml2/lib/libxml2.a" ]; then
    echo "  Already built"
else
    download "https://download.gnome.org/sources/libxml2/2.13/libxml2-${LIBXML2_VERSION}.tar.xz" "$DEPS_DIR/libxml2-${LIBXML2_VERSION}.tar.xz"
    echo "  Building (this may take a few minutes)..."

    cd "$DEPS_DIR"
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

    make -j $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) > /dev/null 2>&1
    make install > /dev/null 2>&1

    cd "$DEPS_DIR"
    rm -rf "libxml2-${LIBXML2_VERSION}"
    echo "  Built successfully"
fi

echo ""
echo "7. libxslt ${LIBXSLT_VERSION}"
if [ -f "$DEPS_DIR/wasi-libxslt/lib/libxslt.a" ]; then
    echo "  Already built"
else
    download "https://download.gnome.org/sources/libxslt/1.1/libxslt-${LIBXSLT_VERSION}.tar.xz" "$DEPS_DIR/libxslt-${LIBXSLT_VERSION}.tar.xz"
    echo "  Building (this may take a few minutes)..."

    cd "$DEPS_DIR"
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

    make -j $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) > /dev/null 2>&1
    make install > /dev/null 2>&1

    cd "$DEPS_DIR"
    rm -rf "libxslt-${LIBXSLT_VERSION}"
    echo "  Built successfully"
fi

# =============================================================================
# Section 3: Python Source Downloads (download only, built by build-python-wasi.sh)
# =============================================================================
echo ""
echo "8. CPython ${PYTHON_VERSION} source"
PYTHON_TARBALL="$DEPS_DIR/Python-${PYTHON_VERSION}.tar.xz"
if [ -f "$PYTHON_TARBALL" ]; then
    echo "  Already downloaded"
else
    download "https://www.python.org/ftp/python/${PYTHON_VERSION}/Python-${PYTHON_VERSION}.tar.xz" "$PYTHON_TARBALL"
fi

echo ""
echo "9. lxml ${LXML_VERSION} source"
LXML_ARCHIVE="$DEPS_DIR/lxml-${LXML_VERSION}.tar.gz"
if [ -f "$LXML_ARCHIVE" ]; then
    echo "  Already downloaded"
else
    download "https://files.pythonhosted.org/packages/source/l/lxml/lxml-${LXML_VERSION}.tar.gz" "$LXML_ARCHIVE"
fi

# =============================================================================
# Section 4: Pydantic and Python Package Downloads
# =============================================================================
echo ""
echo "10. pydantic_core ${PYDANTIC_CORE_VERSION} source"
PYDANTIC_CORE_ARCHIVE="$DEPS_DIR/pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
if [ -f "$PYDANTIC_CORE_ARCHIVE" ]; then
    echo "  Already downloaded"
else
    download "https://files.pythonhosted.org/packages/source/p/pydantic_core/pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz" "$PYDANTIC_CORE_ARCHIVE"
fi

echo ""
echo "11. pydantic ${PYDANTIC_VERSION} source"
PYDANTIC_ARCHIVE="$DEPS_DIR/pydantic-${PYDANTIC_VERSION}.tar.gz"
if [ -f "$PYDANTIC_ARCHIVE" ]; then
    echo "  Already downloaded"
else
    download "https://files.pythonhosted.org/packages/source/p/pydantic/pydantic-${PYDANTIC_VERSION}.tar.gz" "$PYDANTIC_ARCHIVE"
fi

echo ""
echo "12. typing_extensions ${TYPING_EXTENSIONS_VERSION}"
TYPING_EXT_ARCHIVE="$DEPS_DIR/typing_extensions-${TYPING_EXTENSIONS_VERSION}.tar.gz"
if [ -f "$TYPING_EXT_ARCHIVE" ]; then
    echo "  Already downloaded"
else
    download "https://files.pythonhosted.org/packages/source/t/typing_extensions/typing_extensions-${TYPING_EXTENSIONS_VERSION}.tar.gz" "$TYPING_EXT_ARCHIVE"
fi

echo ""
echo "13. annotated_types ${ANNOTATED_TYPES_VERSION}"
ANNOTATED_TYPES_ARCHIVE="$DEPS_DIR/annotated_types-${ANNOTATED_TYPES_VERSION}.tar.gz"
if [ -f "$ANNOTATED_TYPES_ARCHIVE" ]; then
    echo "  Already downloaded"
else
    download "https://files.pythonhosted.org/packages/source/a/annotated_types/annotated_types-${ANNOTATED_TYPES_VERSION}.tar.gz" "$ANNOTATED_TYPES_ARCHIVE"
fi

echo ""
echo "14. typing_inspection ${TYPING_INSPECTION_VERSION}"
TYPING_INSPECTION_ARCHIVE="$DEPS_DIR/typing_inspection-${TYPING_INSPECTION_VERSION}.tar.gz"
if [ -f "$TYPING_INSPECTION_ARCHIVE" ]; then
    echo "  Already downloaded"
else
    download "https://files.pythonhosted.org/packages/source/t/typing_inspection/typing_inspection-${TYPING_INSPECTION_VERSION}.tar.gz" "$TYPING_INSPECTION_ARCHIVE"
fi

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "=== All dependencies ready ==="
echo ""
echo "C Libraries (built):"
echo "  WASI SDK: $WASI_SDK_PATH"
echo "  zlib:     $DEPS_DIR/wasi-zlib"
echo "  bzip2:    $DEPS_DIR/wasi-bzip2"
echo "  liblzma:  $DEPS_DIR/wasi-xz"
echo "  SQLite:   $DEPS_DIR/wasi-sqlite"
echo "  libxml2:  $DEPS_DIR/wasi-libxml2"
echo "  libxslt:  $DEPS_DIR/wasi-libxslt"
echo ""
echo "Source Downloads (to be built by separate scripts):"
echo "  CPython:        $PYTHON_TARBALL"
echo "  lxml:           $LXML_ARCHIVE"
echo "  pydantic_core:  $PYDANTIC_CORE_ARCHIVE"
echo "  pydantic:       $PYDANTIC_ARCHIVE"
echo ""
echo "Next steps:"
echo "  Run ./scripts/build-all.sh to build everything"
