"""C extension registry for WADUP Python builds.

lxml and pydantic are always included in all Python WASM modules.
"""

# Extensions always included in Python builds
EXTENSIONS = {
    "pydantic": {
        "modules": [
            # The core Rust extension
            ("_pydantic_core", "PyInit__pydantic_core"),
        ],
        "libraries": [
            "wasi-pydantic/lib/lib_pydantic_core.a",
        ],
        "python_dirs": [
            "wasi-pydantic/python/pydantic_core",
            "wasi-pydantic/python/pydantic",
            "wasi-pydantic/python/annotated_types",
            "wasi-pydantic/python/typing_inspection",
        ],
        "python_files": [
            # Single-file modules (not packages)
            "wasi-pydantic/python/typing_extensions.py",
        ],
        "validation": [
            "wasi-pydantic/lib/lib_pydantic_core.a",
        ],
    },

    "lxml": {
        "modules": [
            ("lxml.etree", "PyInit_etree"),
        ],
        "libraries": [
            "wasi-lxml/lib/liblxml_etree.a",
            "wasi-libxslt/lib/libexslt.a",
            "wasi-libxslt/lib/libxslt.a",
            "wasi-libxml2/lib/libxml2.a",
        ],
        "python_dirs": [
            "wasi-lxml/python/lxml",
        ],
        "validation": [
            "wasi-lxml/lib/liblxml_etree.a",
            "wasi-libxml2/lib/libxml2.a",
            "wasi-libxslt/lib/libxslt.a",
        ],
    },
}


def get_all_modules() -> list[tuple[str, str]]:
    """Get all C extension modules to register."""
    modules = []
    for ext in EXTENSIONS.values():
        modules.extend(ext.get("modules", []))
    return modules


def get_all_libraries() -> list[str]:
    """Get all library paths to link."""
    libraries = []
    for ext in EXTENSIONS.values():
        libraries.extend(ext.get("libraries", []))
    return libraries


def get_all_python_dirs() -> list[str]:
    """Get all Python directories to bundle."""
    dirs = []
    for ext in EXTENSIONS.values():
        dirs.extend(ext.get("python_dirs", []))
    return dirs


def get_all_python_files() -> list[str]:
    """Get all Python single-file modules to bundle."""
    files = []
    for ext in EXTENSIONS.values():
        files.extend(ext.get("python_files", []))
    return files


def get_validation_files() -> list[str]:
    """Get all files that must exist to confirm extensions are built."""
    files = []
    for ext in EXTENSIONS.values():
        files.extend(ext.get("validation", []))
    return files
