#!/bin/bash
# Build all WADUP components
#
# This script builds everything in the correct order:
#   1. Downloads dependencies (if not already downloaded)
#   2. Builds Python WASI
#   3. Builds pydantic extension
#   4. Builds lxml extension
#   5. Builds wadup CLI
#   6. Builds all examples
#
# Usage:
#   ./scripts/build-all.sh          # Build everything
#   ./scripts/build-all.sh --force  # Force rebuild of examples

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
FORCE=""
if [[ "$1" == "--force" ]]; then
    FORCE="--force"
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

# Track timing
TOTAL_START=$(python3 -c "import time; print(time.time())")

print_header "WADUP Full Build"

# Step 1: Download dependencies
print_header "Step 1: Downloading Dependencies"
"$SCRIPT_DIR/download-deps.sh"

# Step 2: Build Python WASI
print_header "Step 2: Building Python WASI"
"$SCRIPT_DIR/build-python-wasi.sh"

# Step 3: Build pydantic extension
print_header "Step 3: Building pydantic Extension"
"$SCRIPT_DIR/build-pydantic-wasi.sh"

# Step 4: Build lxml extension
print_header "Step 4: Building lxml Extension"
"$SCRIPT_DIR/build-lxml-wasi.sh"

# Step 5: Build wadup CLI and examples
print_header "Step 5: Building Examples"
"$SCRIPT_DIR/build-examples.sh" $FORCE

# Summary
TOTAL_END=$(python3 -c "import time; print(time.time())")
TOTAL_DURATION=$(python3 -c "print(f'{$TOTAL_END - $TOTAL_START:.1f}')")

print_header "Build Complete!"

echo "Total build time: ${TOTAL_DURATION}s"
echo ""
echo "Built components:"
echo "  - WASI SDK:     deps/wasi-sdk-*"
echo "  - Python WASI:  deps/wasi-python/"
echo "  - pydantic:     deps/wasi-pydantic/"
echo "  - lxml:         deps/wasi-lxml/"
echo "  - wadup CLI:    target/release/wadup"
echo "  - Examples:     examples/*/target/"
echo ""
echo "To run integration tests:"
echo "  ./scripts/run-integration-tests.sh"
