#!/bin/bash
# Convert script.py to C header file with string literal

set -e

# Read Python script
SCRIPT_CONTENT=$(cat src/script.py)

# Escape for C string literal
# Replace backslashes first, then quotes, then newlines
ESCAPED=$(echo "$SCRIPT_CONTENT" | \
    sed 's/\\/\\\\/g' | \
    sed 's/"/\\"/g' | \
    sed ':a;N;$!ba;s/\n/\\n/g')

# Write to header file
echo "\"$ESCAPED\"" > src/script.py.h

echo "✓ Embedded script.py into script.py.h"
