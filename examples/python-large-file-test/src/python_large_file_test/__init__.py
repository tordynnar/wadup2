"""Test importing a large if/elif chain module."""

import sys
print("DEBUG: Module loading...", file=sys.stderr, flush=True)

import wadup
print("DEBUG: wadup imported", file=sys.stderr, flush=True)


def main():
    """Test importing large_module with if/elif chain."""
    print("DEBUG: main() called", file=sys.stderr, flush=True)
    print("DEBUG: About to import large_module...", file=sys.stderr, flush=True)

    from python_large_file_test import large_module

    print("DEBUG: large_module imported successfully!", file=sys.stderr, flush=True)

    wadup.define_table("test_result", [("status", "String")])
    wadup.insert_row("test_result", ["large_module imported successfully!"])
    wadup.flush()
