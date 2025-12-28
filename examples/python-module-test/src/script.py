import wadup

# List of C extension modules to test
# Note: Only testing modules that are confirmed to be built as C extensions
# in the Python WASI build. Some modules (bz2, hashlib, lzma, struct, zlib)
# may not be available depending on the build configuration.
c_extension_modules = [
    'array',
    'binascii',
    'cmath',
    'io',
    'itertools',
    'math',
    'time',
    'unicodedata',
]

def main():
    # Create table for import test results
    wadup.define_table("c_extension_imports", [
        ("module_name", "String"),
        ("import_successful", "Int64"),
        ("error_message", "String"),
    ])

    # Test each module
    for module_name in c_extension_modules:
        try:
            __import__(module_name)
            wadup.insert_row("c_extension_imports", [module_name, 1, ""])
        except ImportError as e:
            wadup.insert_row("c_extension_imports", [module_name, 0, str(e)])
        except Exception as e:
            wadup.insert_row("c_extension_imports", [module_name, 0, f"Unexpected error: {str(e)}"])

if __name__ == "__main__":
    main()
