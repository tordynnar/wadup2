"""Python counter WADUP module.

Demonstrates state persistence across multiple process() calls.
Each time WADUP invokes process(), the call_count increments.
"""
import wadup

# Global counter - persists because Python interpreter is reused
_call_count = 0


def main():
    """Entry point called by WADUP for each file processed."""
    global _call_count
    _call_count += 1

    # Define output table
    wadup.define_table("call_counter", [
        ("call_number", "Int64")
    ])

    # Insert the current call count
    wadup.insert_row("call_counter", [_call_count])

    # Flush metadata to file for WADUP to process
    wadup.flush()
