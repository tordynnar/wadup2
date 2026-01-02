"""Test full pydantic library in WADUP - THIS SHOULD CRASH."""

import sys
print("DEBUG: Module loading", file=sys.stderr, flush=True)

import wadup
print("DEBUG: wadup imported", file=sys.stderr, flush=True)


def main():
    """Try to import BaseModel - this triggers the crash."""
    print("DEBUG: main() called", file=sys.stderr, flush=True)

    # This import triggers the crash
    print("DEBUG: About to import BaseModel...", file=sys.stderr, flush=True)
    from pydantic import BaseModel
    print("DEBUG: BaseModel imported successfully!", file=sys.stderr, flush=True)

    wadup.define_table("test_result", [("status", "String")])
    wadup.insert_row("test_result", ["BaseModel imported successfully!"])
    wadup.flush()
