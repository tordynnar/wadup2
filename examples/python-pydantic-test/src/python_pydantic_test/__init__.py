"""Test full pydantic library in WADUP."""

import sys

def debug(msg):
    print(msg, file=sys.stderr, flush=True)

debug("DEBUG: Module loading")

import wadup
debug("DEBUG: wadup imported")


def main():
    """Run test."""
    debug("DEBUG: main() started")

    # Import pydantic_core first (before pydantic)
    debug("DEBUG: Importing pydantic_core first...")
    import pydantic_core
    debug(f"DEBUG: pydantic_core {pydantic_core.__version__} imported!")

    # Now import pydantic
    debug("DEBUG: Importing pydantic...")
    import pydantic
    debug(f"DEBUG: pydantic {pydantic.__version__} imported!")

    # Now try to get BaseModel
    debug("DEBUG: Getting BaseModel from pydantic...")
    from pydantic import BaseModel
    debug("DEBUG: BaseModel imported!")

    debug("DEBUG: SUCCESS!")
