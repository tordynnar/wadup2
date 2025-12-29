#!/bin/bash
# Python WASM project builder for WADUP
# Builds Python projects with pyproject.toml into WASM modules
#
# Usage: ./scripts/build-python-project.sh <project-dir>
#
# The project directory must contain:
#   - pyproject.toml with [project] and [tool.wadup] sections
#   - src/<module_name>/__init__.py entry point
#
# Dependencies listed in pyproject.toml are bundled if they are pure Python.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() { echo -e "${BLUE}ℹ${NC} $1"; }
print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }

# Parse arguments
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <project-directory>"
    echo ""
    echo "Builds a Python WADUP module from a project with pyproject.toml"
    echo ""
    echo "Example:"
    echo "  $0 examples/python-counter"
    exit 1
fi

PROJECT_DIR="$(cd "$1" && pwd)"

# Detect workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"

# Validate project directory
if [ ! -f "$PROJECT_DIR/pyproject.toml" ]; then
    print_error "pyproject.toml not found in $PROJECT_DIR"
    exit 1
fi

print_info "Building Python WADUP module from: $PROJECT_DIR"
echo ""

# Parse pyproject.toml using Python
print_info "Parsing pyproject.toml..."

PROJECT_INFO=$(python3 << 'PYEOF'
import sys
import os

# Try tomllib (Python 3.11+), fall back to tomli
try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print("ERROR: tomllib not available. Please use Python 3.11+ or install tomli.", file=sys.stderr)
        sys.exit(1)

project_dir = os.environ.get('PROJECT_DIR', '.')
with open(os.path.join(project_dir, 'pyproject.toml'), 'rb') as f:
    data = tomllib.load(f)

project = data.get('project', {})
wadup = data.get('tool', {}).get('wadup', {})

name = project.get('name', '')
entry_point = wadup.get('entry-point', '')
dependencies = project.get('dependencies', [])

if not name:
    print("ERROR: [project].name not found in pyproject.toml", file=sys.stderr)
    sys.exit(1)

if not entry_point:
    print("ERROR: [tool.wadup].entry-point not found in pyproject.toml", file=sys.stderr)
    sys.exit(1)

# Output as shell-parseable format
print(f"PROJECT_NAME={name}")
print(f"ENTRY_MODULE={entry_point}")
print(f"DEPENDENCIES={' '.join(dependencies)}")
PYEOF
)

if [ $? -ne 0 ]; then
    print_error "Failed to parse pyproject.toml"
    exit 1
fi

# Source the parsed values
eval "$PROJECT_INFO"

print_success "Project: $PROJECT_NAME"
print_success "Entry point: $ENTRY_MODULE"
if [ -n "$DEPENDENCIES" ]; then
    print_success "Dependencies: $DEPENDENCIES"
fi
echo ""

# Convert project name to WASM filename (hyphens to underscores)
WASM_NAME=$(echo "$PROJECT_NAME" | tr '-' '_')

# Set paths
PYTHON_VERSION="3.13"
PYTHON_DIR="$WADUP_ROOT/build/python-wasi"
OUTPUT_DIR="$PROJECT_DIR/target"

# Detect platform for WASI SDK
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" = "darwin" ]; then
    WASI_SDK_OS="macos"
elif [ "$OS" = "linux" ]; then
    WASI_SDK_OS="linux"
else
    print_error "Unsupported OS: $OS"
    exit 1
fi

WASI_SDK_VERSION="29.0"
WASI_SDK_PATH="$DEPS_DIR/wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"
WASI_SYSROOT="$WASI_SDK_PATH/share/wasi-sysroot"

# Validate dependencies
if [ ! -f "$PYTHON_DIR/lib/libpython${PYTHON_VERSION}.a" ]; then
    print_error "CPython not built. Run ./scripts/build-python-wasi.sh first"
    exit 1
fi

if [ ! -d "$WASI_SDK_PATH" ]; then
    print_error "WASI SDK not found. Run ./scripts/download-deps.sh first"
    exit 1
fi

if [ ! -f "$DEPS_DIR/wasi-zlib/lib/libz.a" ]; then
    print_error "zlib not found. Run ./scripts/download-deps.sh first"
    exit 1
fi

# Validate source directory
if [ ! -d "$PROJECT_DIR/src/$ENTRY_MODULE" ]; then
    print_error "Source directory not found: $PROJECT_DIR/src/$ENTRY_MODULE"
    print_info "Expected structure:"
    print_info "  $PROJECT_DIR/"
    print_info "  ├── pyproject.toml"
    print_info "  └── src/"
    print_info "      └── $ENTRY_MODULE/"
    print_info "          └── __init__.py"
    exit 1
fi

# Create build directory
BUILD_TIMESTAMP=$(date +%s)
BUILD_DIR="/tmp/wadup-python-build-${PROJECT_NAME}-${BUILD_TIMESTAMP}"
mkdir -p "$BUILD_DIR"
mkdir -p "$OUTPUT_DIR"

print_info "Build directory: $BUILD_DIR"
echo ""

# Create bundle directory structure
BUNDLE_DIR="$BUILD_DIR/bundle"
mkdir -p "$BUNDLE_DIR"

# Copy wadup library
print_info "Bundling wadup library..."
cp -r "$WADUP_ROOT/python-wadup-guest/wadup" "$BUNDLE_DIR/"

# Copy project source
print_info "Bundling project source..."
cp -r "$PROJECT_DIR/src/$ENTRY_MODULE" "$BUNDLE_DIR/"

# Handle dependencies (if any)
if [ -n "$DEPENDENCIES" ]; then
    print_info "Downloading dependencies (including transitive)..."
    DEPS_TEMP="$BUILD_DIR/deps"
    mkdir -p "$DEPS_TEMP"

    # Download all dependencies at once, letting pip resolve the full dependency tree
    print_info "  Dependencies: $DEPENDENCIES"
    pip download --no-binary :all: -d "$DEPS_TEMP" $DEPENDENCIES 2>/dev/null || {
        print_warning "  Failed to download as pure Python, trying with binaries..."
        pip download -d "$DEPS_TEMP" $DEPENDENCIES 2>/dev/null || {
            print_error "  Failed to download dependencies"
            exit 1
        }
    }

    # Extract dependencies
    for archive in "$DEPS_TEMP"/*.tar.gz "$DEPS_TEMP"/*.zip; do
        [ -f "$archive" ] || continue
        print_info "  Extracting: $(basename "$archive")"

        if [[ "$archive" == *.tar.gz ]]; then
            tar -xzf "$archive" -C "$DEPS_TEMP"
        else
            unzip -q "$archive" -d "$DEPS_TEMP"
        fi
    done

    # Copy extracted packages to bundle
    for pkg_dir in "$DEPS_TEMP"/*/; do
        [ -d "$pkg_dir" ] || continue
        pkg_name=$(basename "$pkg_dir")

        # Skip if it's just the archive basename
        [[ "$pkg_name" == *.tar.gz ]] && continue
        [[ "$pkg_name" == *.zip ]] && continue

        # Look for the actual Python package inside
        # Try src/ layout first (e.g., attrs uses src/attr/)
        if [ -d "$pkg_dir/src" ]; then
            for subpkg in "$pkg_dir/src"/*/; do
                [ -d "$subpkg" ] || continue
                [ -f "$subpkg/__init__.py" ] || continue
                # Remove trailing slash to copy directory, not contents
                subpkg="${subpkg%/}"
                cp -r "$subpkg" "$BUNDLE_DIR/"
                print_success "  Added: $(basename "$subpkg")"
            done
        else
            # Try flat layout (e.g., chardet uses chardet-5.2.0/chardet/)
            for subpkg in "$pkg_dir"/*/; do
                [ -d "$subpkg" ] || continue
                [ -f "$subpkg/__init__.py" ] || continue
                subpkg_name=$(basename "$subpkg")
                # Skip common non-package directories
                [[ "$subpkg_name" == tests ]] && continue
                [[ "$subpkg_name" == test ]] && continue
                [[ "$subpkg_name" == docs ]] && continue
                [[ "$subpkg_name" == examples ]] && continue
                [[ "$subpkg_name" == .* ]] && continue
                # Remove trailing slash to copy directory, not contents
                subpkg="${subpkg%/}"
                cp -r "$subpkg" "$BUNDLE_DIR/"
                print_success "  Added: $(basename "$subpkg")"
            done
        fi
    done

    # Also check for wheel files
    for wheel in "$DEPS_TEMP"/*.whl; do
        [ -f "$wheel" ] || continue
        print_info "  Extracting wheel: $(basename "$wheel")"
        unzip -q "$wheel" -d "$BUILD_DIR/wheel_extract"

        # Copy Python packages from wheel
        for subdir in "$BUILD_DIR/wheel_extract"/*/; do
            [ -d "$subdir" ] || continue
            subname=$(basename "$subdir")
            # Skip metadata directories
            [[ "$subname" == *.dist-info ]] && continue
            [[ "$subname" == *.data ]] && continue

            if [ -f "$subdir/__init__.py" ]; then
                # Remove trailing slash to copy directory, not contents
                subdir="${subdir%/}"
                cp -r "$subdir" "$BUNDLE_DIR/"
                print_success "  Added: $subname"
            fi
        done
        rm -rf "$BUILD_DIR/wheel_extract"
    done
fi

echo ""

# Create zip bundle
print_info "Creating bundle.zip..."
BUNDLE_ZIP="$BUILD_DIR/bundle.zip"
(cd "$BUNDLE_DIR" && zip -rq "$BUNDLE_ZIP" .)
BUNDLE_SIZE=$(wc -c < "$BUNDLE_ZIP" | tr -d ' ')
print_success "Bundle size: $BUNDLE_SIZE bytes"

# Generate bundle.h
print_info "Generating bundle.h..."
python3 << PYEOF
import sys

with open('$BUNDLE_ZIP', 'rb') as f:
    data = f.read()

with open('$BUILD_DIR/bundle.h', 'w') as f:
    f.write('// Auto-generated bundle header\n')
    f.write('// Contains embedded Python modules as a zip file\n\n')

    f.write(f'#define ENTRY_MODULE "{sys.argv[1] if len(sys.argv) > 1 else "$ENTRY_MODULE"}"\n\n')

    f.write(f'static const size_t BUNDLE_SIZE = {len(data)};\n\n')

    f.write('static const unsigned char BUNDLE_DATA[] = {\n')

    # Write bytes in rows of 16
    for i in range(0, len(data), 16):
        chunk = data[i:i+16]
        hex_vals = ', '.join(f'0x{b:02x}' for b in chunk)
        f.write(f'    {hex_vals},\n')

    f.write('};\n')

print(f'Generated bundle.h ({len(data)} bytes)')
PYEOF

echo ""

# Compile
print_info "Compiling..."
CC="$WASI_SDK_PATH/bin/clang"
CFLAGS="-O2 -D_WASI_EMULATED_SIGNAL -D_WASI_EMULATED_GETPID -D_WASI_EMULATED_PROCESS_CLOCKS -I$PYTHON_DIR/include -I$BUILD_DIR -fvisibility=default"
LDFLAGS="-Wl,--allow-undefined -Wl,--export=process -Wl,--initial-memory=134217728 -Wl,--max-memory=268435456 -Wl,--no-entry"
WASI_EMU_LIBS="$WASI_SYSROOT/lib/wasm32-wasip1"

# Copy main_bundled.c to build directory
cp "$WADUP_ROOT/python-wadup-guest/src/main_bundled.c" "$BUILD_DIR/"

cd "$BUILD_DIR"
"$CC" $CFLAGS -c main_bundled.c -o main_bundled.o

print_info "Linking..."
"$CC" $CFLAGS main_bundled.o -o "${WASM_NAME}.wasm" \
    -L"$PYTHON_DIR/lib" \
    -lpython${PYTHON_VERSION} \
    "$PYTHON_DIR/lib/libmpdec.a" \
    "$PYTHON_DIR/lib/libexpat.a" \
    "$PYTHON_DIR/lib/libsqlite3.a" \
    $PYTHON_DIR/lib/libHacl_*.a \
    "$DEPS_DIR/wasi-zlib/lib/libz.a" \
    "$DEPS_DIR/wasi-bzip2/lib/libbz2.a" \
    "$DEPS_DIR/wasi-xz/lib/liblzma.a" \
    "$WASI_EMU_LIBS/libwasi-emulated-signal.a" \
    "$WASI_EMU_LIBS/libwasi-emulated-getpid.a" \
    "$WASI_EMU_LIBS/libwasi-emulated-process-clocks.a" \
    -lm \
    $LDFLAGS

# Copy to output directory
cp "${WASM_NAME}.wasm" "$OUTPUT_DIR/"

# Clean up
cd /
rm -rf "$BUILD_DIR"

echo ""
print_success "Build successful!"
print_success "Output: $OUTPUT_DIR/${WASM_NAME}.wasm"
ls -lh "$OUTPUT_DIR/${WASM_NAME}.wasm"
