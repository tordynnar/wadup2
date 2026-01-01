#!/bin/bash
# Build pydantic_core Rust extension for WASI
# This script compiles the pydantic_core Rust library for wasm32-wasip1

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"
BUILD_DIR="$WADUP_ROOT/build"

# Version
PYDANTIC_CORE_VERSION="2.41.5"

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

echo "=== Building pydantic_core ${PYDANTIC_CORE_VERSION} for WASI ==="

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
if [ -f "$DEPS_DIR/wasi-pydantic/lib/lib_pydantic_core.a" ]; then
    echo "pydantic_core already built"
    exit 0
fi

# Create directories
mkdir -p "$DEPS_DIR/wasi-pydantic/lib"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/pydantic_core"

# Download pydantic_core source if needed
PYDANTIC_ARCHIVE="$DEPS_DIR/pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
if [ ! -f "$PYDANTIC_ARCHIVE" ]; then
    echo "Downloading pydantic_core ${PYDANTIC_CORE_VERSION}..."
    curl -L -o "$PYDANTIC_ARCHIVE" \
        "https://files.pythonhosted.org/packages/source/p/pydantic_core/pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
fi

# Extract
echo "Extracting..."
cd "$DEPS_DIR"
rm -rf "pydantic-core-${PYDANTIC_CORE_VERSION}"
tar xzf "pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
cd "pydantic-core-${PYDANTIC_CORE_VERSION}"

# Patch Cargo.toml for WASI static linking
echo "Patching Cargo.toml for WASI..."
cp Cargo.toml Cargo.toml.orig

# Remove generate-import-lib feature (not needed for static linking)
sed -i.bak 's/"generate-import-lib", //' Cargo.toml

# Change crate-type from cdylib to staticlib
sed -i.bak 's/crate-type = \["cdylib", "rlib"\]/crate-type = ["staticlib", "rlib"]/' Cargo.toml

# Add workspace isolation
echo '' >> Cargo.toml
echo '# Keep out of parent workspace' >> Cargo.toml
echo '[workspace]' >> Cargo.toml

# Create PyO3 config file for cross-compilation
cat > pyo3-wasi-config.txt << EOF
implementation=CPython
version=3.13
shared=false
abi3=false
lib_name=python3.13
lib_dir=$BUILD_DIR/python-wasi/lib
pointer_width=32
build_flags=
suppress_build_script_link_lines=true
EOF

echo "Building Rust library for wasm32-wasip1..."
export PYO3_CONFIG_FILE="$(pwd)/pyo3-wasi-config.txt"
export CARGO_TARGET_WASM32_WASIP1_LINKER="${WASI_SDK_PATH}/bin/wasm-ld"

# Build with cargo
cargo build --target wasm32-wasip1 --release 2>&1

# Check build succeeded
if [ ! -f "target/wasm32-wasip1/release/lib_pydantic_core.a" ]; then
    echo "ERROR: Rust build failed"
    exit 1
fi

echo "Copying library..."
cp "target/wasm32-wasip1/release/lib_pydantic_core.a" "$DEPS_DIR/wasi-pydantic/lib/"

# Copy Python stub files
echo "Copying Python files..."
cp python/pydantic_core/__init__.py "$DEPS_DIR/wasi-pydantic/python/pydantic_core/"
cp python/pydantic_core/core_schema.py "$DEPS_DIR/wasi-pydantic/python/pydantic_core/"

# Create __init__.py that imports from the C extension
cat > "$DEPS_DIR/wasi-pydantic/python/pydantic_core/__init__.py" << 'PYEOF'
"""pydantic_core - Core validation library for Pydantic V2."""
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
    list_all_errors,
)

__all__ = [
    "__version__",
    "ArgsKwargs",
    "MultiHostUrl",
    "PydanticCustomError",
    "PydanticKnownError",
    "PydanticOmit",
    "PydanticSerializationError",
    "PydanticSerializationUnexpectedValue",
    "PydanticUndefined",
    "PydanticUndefinedType",
    "PydanticUseDefault",
    "SchemaError",
    "SchemaSerializer",
    "SchemaValidator",
    "Some",
    "TzInfo",
    "Url",
    "ValidationError",
    "from_json",
    "to_json",
    "to_jsonable_python",
    "list_all_errors",
]
PYEOF

# Copy core_schema.py (pure Python)
if [ -f "python/pydantic_core/core_schema.py" ]; then
    cp "python/pydantic_core/core_schema.py" "$DEPS_DIR/wasi-pydantic/python/pydantic_core/"
fi

# Clean up build artifacts (keep source for debugging)
rm -rf target

echo ""
echo "=== pydantic_core build complete ==="
echo "Library: $DEPS_DIR/wasi-pydantic/lib/lib_pydantic_core.a ($(du -h "$DEPS_DIR/wasi-pydantic/lib/lib_pydantic_core.a" | cut -f1))"
echo "Python:  $DEPS_DIR/wasi-pydantic/python/pydantic_core/"
