#!/usr/bin/env bash
# Build all example WASM modules for WADUP integration tests
#
# Usage: ./scripts/build-examples.sh [--force]
#
# Options:
#   --force    Rebuild all modules even if they already exist

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Timing (stored in temp file for bash 3.x compatibility)
TIMING_FILE=$(mktemp)
TOTAL_START=$(python3 -c "import time; print(time.time())")

cleanup_timing() {
    rm -f "$TIMING_FILE" 2>/dev/null || true
}
trap cleanup_timing EXIT

# Parse arguments
FORCE=false
if [[ "$1" == "--force" ]]; then
    FORCE=true
fi

print_header() {
    echo ""
    echo -e "${BLUE}============================================================${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}============================================================${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

record_timing() {
    local name="$1"
    local duration="$2"
    echo "$name|$duration" >> "$TIMING_FILE"
}

build_rust_module() {
    local name="$1"
    local target="wasm32-wasip1"
    local wasm_file="$WADUP_ROOT/examples/$name/target/$target/release/${name//-/_}.wasm"

    if [[ "$FORCE" == "false" && -f "$wasm_file" ]]; then
        print_info "Skipping $name (already built)"
        record_timing "$name" "0 (cached)"
        return 0
    fi

    local start=$(python3 -c "import time; print(time.time())")

    cargo build \
        --manifest-path "$WADUP_ROOT/examples/$name/Cargo.toml" \
        --target "$target" \
        --release \
        2>&1

    local end=$(python3 -c "import time; print(time.time())")
    local duration=$(python3 -c "print(f'{$end - $start:.2f}')")

    if [[ -f "$wasm_file" ]]; then
        local size=$(du -h "$wasm_file" | cut -f1)
        print_success "$name built in ${duration}s ($size)"
        record_timing "$name" "$duration"
    else
        print_error "$name build failed"
        record_timing "$name" "$duration (FAILED)"
        return 1
    fi
}

build_python_module() {
    local name="$1"
    local wasm_file="$WADUP_ROOT/examples/$name/target/${name//-/_}.wasm"

    if [[ "$FORCE" == "false" && -f "$wasm_file" ]]; then
        print_info "Skipping $name (already built)"
        record_timing "$name" "0 (cached)"
        return 0
    fi

    local start=$(python3 -c "import time; print(time.time())")

    "$WADUP_ROOT/scripts/build-python-project.py" "$WADUP_ROOT/examples/$name" 2>&1

    local end=$(python3 -c "import time; print(time.time())")
    local duration=$(python3 -c "print(f'{$end - $start:.2f}')")

    if [[ -f "$wasm_file" ]]; then
        local size=$(du -h "$wasm_file" | cut -f1)
        print_success "$name built in ${duration}s ($size)"
        record_timing "$name" "$duration"
    else
        print_error "$name build failed"
        record_timing "$name" "$duration (FAILED)"
        return 1
    fi
}

build_go_module() {
    local name="$1"
    local wasm_file="$WADUP_ROOT/examples/$name/target/${name//-/_}.wasm"

    if [[ "$FORCE" == "false" && -f "$wasm_file" ]]; then
        print_info "Skipping $name (already built)"
        record_timing "$name" "0 (cached)"
        return 0
    fi

    local start=$(python3 -c "import time; print(time.time())")

    (
        cd "$WADUP_ROOT/examples/$name"
        mkdir -p target

        GOOS=wasip1 GOARCH=wasm go build -o "target/${name//-/_}.wasm" .
    ) 2>&1

    local end=$(python3 -c "import time; print(time.time())")
    local duration=$(python3 -c "print(f'{$end - $start:.2f}')")

    if [[ -f "$wasm_file" ]]; then
        local size=$(du -h "$wasm_file" | cut -f1)
        print_success "$name built in ${duration}s ($size)"
        record_timing "$name" "$duration"
    else
        print_error "$name build failed"
        record_timing "$name" "$duration (FAILED)"
        return 1
    fi
}

build_csharp_module() {
    local name="$1"
    local wasm_file="$WADUP_ROOT/examples/$name/target/${name//-/_}.wasm"

    if [[ "$FORCE" == "false" && -f "$wasm_file" ]]; then
        print_info "Skipping $name (already built)"
        record_timing "$name" "0 (cached)"
        return 0
    fi

    local start=$(python3 -c "import time; print(time.time())")

    (
        cd "$WADUP_ROOT/examples/$name"
        mkdir -p target

        dotnet publish -c Release -o publish 2>&1
        cp "publish/${name}.wasm" "target/${name//-/_}.wasm"
    ) 2>&1

    local end=$(python3 -c "import time; print(time.time())")
    local duration=$(python3 -c "print(f'{$end - $start:.2f}')")

    if [[ -f "$wasm_file" ]]; then
        local size=$(du -h "$wasm_file" | cut -f1)
        print_success "$name built in ${duration}s ($size)"
        record_timing "$name" "$duration"
    else
        print_error "$name build failed"
        record_timing "$name" "$duration (FAILED)"
        return 1
    fi
}

print_timing_table() {
    echo ""
    echo -e "${BLUE}============================================================${NC}"
    echo -e "${BLUE}  BUILD TIMING SUMMARY${NC}"
    echo -e "${BLUE}============================================================${NC}"
    echo ""

    printf "  %-35s %12s\n" "Module" "Duration"
    printf "  %-35s %12s\n" "-----------------------------------" "------------"

    # Sort by duration (numeric, descending), put cached at end
    (grep -v "cached" "$TIMING_FILE" | sort -t'|' -k2 -rn; grep "cached" "$TIMING_FILE") | while IFS='|' read -r name duration; do
        if [[ "$duration" == *"cached"* ]]; then
            printf "  %-35s %12s\n" "$name" "$duration"
        elif [[ "$duration" == *"FAILED"* ]]; then
            printf "  %-35s ${RED}%12s${NC}\n" "$name" "$duration"
        else
            printf "  %-35s %11ss\n" "$name" "$duration"
        fi
    done

    local total_end=$(python3 -c "import time; print(time.time())")
    local total_duration=$(python3 -c "print(f'{$total_end - $TOTAL_START:.2f}')")

    printf "  %-35s %12s\n" "-----------------------------------" "------------"
    printf "  %-35s %11ss\n" "TOTAL" "$total_duration"
    echo ""
}

# Main build sequence
print_header "Building WADUP Examples"

echo "Building wadup CLI..."
CLI_START=$(python3 -c "import time; print(time.time())")
cargo build --release --manifest-path "$WADUP_ROOT/Cargo.toml" 2>&1
CLI_END=$(python3 -c "import time; print(time.time())")
CLI_DURATION=$(python3 -c "print(f'{$CLI_END - $CLI_START:.2f}')")
print_success "wadup-cli built in ${CLI_DURATION}s"
record_timing "wadup-cli" "$CLI_DURATION"

print_header "Building Rust Modules"
build_rust_module "sqlite-parser"
build_rust_module "zip-extractor"
build_rust_module "byte-counter"
build_rust_module "simple-test"

print_header "Building Go Modules"
build_go_module "go-sqlite-parser"

print_header "Building C# Modules"
build_csharp_module "csharp-json-analyzer"

print_header "Building Python Modules"
build_python_module "python-sqlite-parser"
build_python_module "python-counter"
build_python_module "python-module-test"
build_python_module "python-multi-file"
build_python_module "python-lxml-test"
build_python_module "python-numpy-test"
build_python_module "python-pandas-test"
build_python_module "python-pydantic-test"

print_header "Precompiling WASM Modules"

precompile_modules() {
    local dir="$1"
    if [[ -d "$dir" ]]; then
        # Check if there are any .wasm files in the directory
        if ls "$dir"/*.wasm 1>/dev/null 2>&1; then
            "$WADUP_ROOT/target/release/wadup" compile --modules "$dir" 2>&1 | grep -v "^$" || true
        fi
    fi
}

PRECOMPILE_START=$(python3 -c "import time; print(time.time())")

# Rust modules (in wasm32-wasip1/release)
precompile_modules "$WADUP_ROOT/examples/sqlite-parser/target/wasm32-wasip1/release"
precompile_modules "$WADUP_ROOT/examples/zip-extractor/target/wasm32-wasip1/release"
precompile_modules "$WADUP_ROOT/examples/byte-counter/target/wasm32-wasip1/release"
precompile_modules "$WADUP_ROOT/examples/simple-test/target/wasm32-wasip1/release"

# Go modules
precompile_modules "$WADUP_ROOT/examples/go-sqlite-parser/target"

# C# modules
precompile_modules "$WADUP_ROOT/examples/csharp-json-analyzer/target"

# Python modules
precompile_modules "$WADUP_ROOT/examples/python-sqlite-parser/target"
precompile_modules "$WADUP_ROOT/examples/python-counter/target"
precompile_modules "$WADUP_ROOT/examples/python-module-test/target"
precompile_modules "$WADUP_ROOT/examples/python-multi-file/target"
precompile_modules "$WADUP_ROOT/examples/python-lxml-test/target"
precompile_modules "$WADUP_ROOT/examples/python-numpy-test/target"
precompile_modules "$WADUP_ROOT/examples/python-pandas-test/target"
precompile_modules "$WADUP_ROOT/examples/python-pydantic-test/target"

PRECOMPILE_END=$(python3 -c "import time; print(time.time())")
PRECOMPILE_DURATION=$(python3 -c "print(f'{$PRECOMPILE_END - $PRECOMPILE_START:.2f}')")
print_success "Precompilation completed in ${PRECOMPILE_DURATION}s"
record_timing "precompile-all" "$PRECOMPILE_DURATION"

print_timing_table

print_success "All examples built successfully!"
