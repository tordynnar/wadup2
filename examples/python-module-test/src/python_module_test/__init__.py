"""Python module test WADUP module.

Tests that C extension modules from the frozen stdlib can be imported.
These modules are built as C extensions in the Python WASI build.
"""
import wadup

# List of C extension modules to test
# These modules are built as C extensions in the Python WASI build
C_EXTENSION_MODULES = [
    'array',
    'binascii',
    'bz2',
    'cmath',
    'hashlib',
    'io',
    'itertools',
    'lzma',
    'math',
    'struct',
    'time',
    'unicodedata',
    'zlib',
]


def main():
    """Entry point called by WADUP for each file processed."""
    # Create table for import test results
    wadup.define_table("c_extension_imports", [
        ("module_name", "String"),
        ("import_successful", "Int64"),
        ("error_message", "String"),
    ])

    # Test each module
    for module_name in C_EXTENSION_MODULES:
        try:
            __import__(module_name)
            wadup.insert_row("c_extension_imports", [module_name, 1, ""])
        except ImportError as e:
            wadup.insert_row("c_extension_imports", [module_name, 0, str(e)])
        except Exception as e:
            wadup.insert_row("c_extension_imports", [module_name, 0, f"Unexpected error: {str(e)}"])

    # Flush metadata to file for WADUP to process
    wadup.flush()
