#!/bin/bash
# Clean all build artifacts and dependencies
#
# This script removes:
#   - deps/          All downloaded dependencies (including Python WASI build)
#   - target/        Rust build artifacts
#   - examples/*/target/  Example build artifacts

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== WADUP Clean ==="
echo ""

# Function to remove directory with size info
remove_dir() {
    local dir="$1"
    local name="$2"

    if [ -d "$dir" ]; then
        local size=$(du -sh "$dir" 2>/dev/null | cut -f1)
        echo -e "Removing $name ($size)..."
        rm -rf "$dir"
        echo -e "  ${GREEN}✓${NC} Removed $name"
    else
        echo -e "  ${YELLOW}○${NC} $name not found (skipping)"
    fi
}

# Remove main directories
remove_dir "$WADUP_ROOT/deps" "deps/"
remove_dir "$WADUP_ROOT/target" "target/"

# Remove example build artifacts
echo ""
echo "Removing example build artifacts..."
for example_dir in "$WADUP_ROOT/examples"/*/; do
    if [ -d "${example_dir}target" ]; then
        example_name=$(basename "$example_dir")
        size=$(du -sh "${example_dir}target" 2>/dev/null | cut -f1)
        rm -rf "${example_dir}target"
        echo -e "  ${GREEN}✓${NC} examples/$example_name/target ($size)"
    fi
done

# Remove any backup files
echo ""
echo "Removing backup files..."
backup_count=0
while IFS= read -r -d '' file; do
    rm -f "$file"
    ((backup_count++))
done < <(find "$WADUP_ROOT" -name "*.bak" -o -name "*.bak2" -o -name "*.orig" 2>/dev/null | tr '\n' '\0')
if [ $backup_count -gt 0 ]; then
    echo -e "  ${GREEN}✓${NC} Removed $backup_count backup files"
else
    echo -e "  ${YELLOW}○${NC} No backup files found"
fi

echo ""
echo -e "${GREEN}Clean complete!${NC}"
echo ""
echo "To rebuild:"
echo "  1. ./scripts/download-deps.sh   # Download dependencies"
echo "  2. ./scripts/build-all.sh       # Build everything"
