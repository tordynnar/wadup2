#!/bin/bash
# WADUP Python Module Build Script
set -e

echo "=== WADUP Python Build ==="
echo "Building module in /build/src"

# Check for pyproject.toml
if [ ! -f "pyproject.toml" ]; then
    echo "ERROR: No pyproject.toml found in /build/src"
    exit 1
fi

# Extract module name from pyproject.toml
MODULE_NAME=$(grep -E '^name\s*=' pyproject.toml | head -1 | sed 's/.*"\([^"]*\)".*/\1/' | tr '-' '_')
echo "Module name: $MODULE_NAME"

# Find the main Python file
if [ -f "${MODULE_NAME}/__init__.py" ]; then
    MAIN_FILE="${MODULE_NAME}/__init__.py"
elif [ -f "${MODULE_NAME}.py" ]; then
    MAIN_FILE="${MODULE_NAME}.py"
elif [ -f "__init__.py" ]; then
    MAIN_FILE="__init__.py"
else
    echo "ERROR: Could not find main Python file"
    echo "Expected: ${MODULE_NAME}/__init__.py, ${MODULE_NAME}.py, or __init__.py"
    exit 1
fi

echo "Main file: $MAIN_FILE"

# Create a WIT file for the component interface
cat > /tmp/wadup-module.wit << 'EOF'
package wadup:module;

world module {
    export process: func() -> s32;
}
EOF

# Build using componentize-py
echo "Componentizing Python module..."
componentize-py -d /tmp/wadup-module.wit -w module componentize "$MODULE_NAME" -o /build/output/module.wasm

# Show file size
if [ -f "/build/output/module.wasm" ]; then
    WASM_SIZE=$(stat -c%s /build/output/module.wasm 2>/dev/null || stat -f%z /build/output/module.wasm)
    echo "Output size: $WASM_SIZE bytes"
    echo "=== Build completed successfully ==="
else
    echo "ERROR: WASM file not created"
    exit 1
fi
