#!/usr/bin/env bash

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Detect platform
detect_platform() {
    local os=""
    local arch=""

    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="linux";;
        Darwin*)    os="macos";;
        MINGW*|MSYS*|CYGWIN*) os="windows";;
        *)
            print_error "Unsupported OS: $(uname -s)"
            exit 1
            ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64";;
        arm64|aarch64)  arch="arm64";;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    echo "${arch}-${os}"
}

# Get the project root directory (two levels up from this script)
get_project_root() {
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    echo "$(cd "$script_dir/../.." && pwd)"
}

# Find WASI SDK in project deps directory
find_wasi_sdk() {
    local platform=$1
    local project_root=$(get_project_root)

    # Check for any WASI SDK in project deps
    local wasi_sdk=$(find "${project_root}/deps" -maxdepth 1 -name "wasi-sdk-*" -type d 2>/dev/null | head -n 1)
    if [ -n "$wasi_sdk" ]; then
        print_success "WASI SDK found at $wasi_sdk" >&2
        echo "$wasi_sdk"
        return 0
    fi

    print_error "WASI SDK not found in ${project_root}/deps/" >&2
    print_info "Please run: scripts/download-deps.sh" >&2
    exit 1
}

# Check and add Rust target
ensure_rust_target() {
    print_info "Checking for wasm32-wasip1 target..."

    if rustup target list | grep -q "wasm32-wasip1 (installed)"; then
        print_success "wasm32-wasip1 target already installed"
    else
        print_info "Adding wasm32-wasip1 target..."
        rustup target add wasm32-wasip1 || {
            print_error "Failed to add wasm32-wasip1 target"
            exit 1
        }
        print_success "wasm32-wasip1 target added"
    fi
}

# Main build function
build_module() {
    local wasi_sdk_path=$1

    print_info "Building sqlite-parser module..."

    # Set environment variables
    export WASI_SDK_PATH="$wasi_sdk_path"
    export LIBSQLITE3_FLAGS="-DSQLITE_THREADSAFE=0"

    # Build
    cargo build --target wasm32-wasip1 --release || {
        print_error "Build failed"
        exit 1
    }

    print_success "Build completed successfully!"

    # Show output location
    local output="target/wasm32-wasip1/release/sqlite_parser.wasm"
    if [ -f "$output" ]; then
        local size=$(du -h "$output" | cut -f1)
        print_success "Module built: $output (${size})"
    fi
}

# Main script
main() {
    echo ""
    print_info "SQLite Parser WASM Module Builder"
    echo ""

    # Detect platform
    print_info "Detecting platform..."
    PLATFORM=$(detect_platform)
    print_success "Platform: $PLATFORM"
    echo ""

    # Find WASI SDK in deps directory
    WASI_SDK_PATH=$(find_wasi_sdk "$PLATFORM")
    echo ""

    # Ensure Rust target
    ensure_rust_target
    echo ""

    # Build
    build_module "$WASI_SDK_PATH"
    echo ""

    print_success "All done! 🎉"
    echo ""
}

# Run main
main "$@"
