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

# Download and extract WASI SDK
download_wasi_sdk() {
    local platform=$1
    local wasi_version="24.0"
    local wasi_dir="/tmp/wasi-sdk-${wasi_version}-${platform}"

    # Check if already downloaded
    if [ -d "$wasi_dir" ]; then
        print_success "WASI SDK already present at $wasi_dir" >&2
        echo "$wasi_dir"
        return 0
    fi

    # Check for any WASI SDK in /tmp
    local existing_wasi=$(find /tmp -maxdepth 1 -name "wasi-sdk-*" -type d 2>/dev/null | head -n 1)
    if [ -n "$existing_wasi" ]; then
        print_success "Found existing WASI SDK at $existing_wasi" >&2
        echo "$existing_wasi"
        return 0
    fi

    print_info "Downloading WASI SDK ${wasi_version} for ${platform}..." >&2

    local filename="wasi-sdk-${wasi_version}-${platform}.tar.gz"
    local url="https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/${filename}"
    local temp_file="/tmp/${filename}"

    # Download with progress
    if command -v curl &> /dev/null; then
        curl -L -o "$temp_file" "$url" --progress-bar || {
            print_error "Failed to download WASI SDK" >&2
            exit 1
        }
    elif command -v wget &> /dev/null; then
        wget -O "$temp_file" "$url" || {
            print_error "Failed to download WASI SDK" >&2
            exit 1
        }
    else
        print_error "Neither curl nor wget found. Please install one of them." >&2
        exit 1
    fi

    print_info "Extracting WASI SDK..." >&2
    tar -xzf "$temp_file" -C /tmp || {
        print_error "Failed to extract WASI SDK" >&2
        rm -f "$temp_file"
        exit 1
    }

    rm -f "$temp_file"
    print_success "WASI SDK downloaded and extracted to $wasi_dir" >&2

    echo "$wasi_dir"
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

    # Download WASI SDK if needed
    WASI_SDK_PATH=$(download_wasi_sdk "$PLATFORM")
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
