"""Test module that imports crashing code at load time."""

import sys
print("DEBUG: Module loading...", file=sys.stderr, flush=True)

# Import the large module at load time
print("DEBUG: About to import large_module...", file=sys.stderr, flush=True)
from python_large_file_test import large_module
print("DEBUG: large_module imported successfully!", file=sys.stderr, flush=True)

print("DEBUG: All imports done!", file=sys.stderr, flush=True)


def main():
    """Main entry point - just print success."""
    print("DEBUG: main() called", file=sys.stderr, flush=True)
    print("SUCCESS!", file=sys.stderr, flush=True)
