"""C extension registry for WADUP Python builds.

Each extension defines:
- modules: List of (module_name, init_function) tuples to register with PyImport_AppendInittab
- libraries: List of static library paths (relative to deps/)
- python_dirs: List of pure Python directories to bundle (relative to deps/)
- dependencies: List of other extension names this depends on
- validation: List of files that must exist (relative to deps/) to consider this extension built
"""

EXTENSIONS = {
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
        "dependencies": [],
        "validation": [
            "wasi-lxml/lib/liblxml_etree.a",
            "wasi-libxml2/lib/libxml2.a",
            "wasi-libxslt/lib/libxslt.a",
        ],
    },

    "numpy": {
        "modules": [
            # Core array module - the main multiarray_umath extension
            ("numpy._core._multiarray_umath", "PyInit__multiarray_umath"),
            # Linear algebra ufuncs (solve, inv, det, eig, svd, etc.)
            ("numpy.linalg._umath_linalg", "PyInit__umath_linalg"),
        ],
        "libraries": [
            "wasi-numpy/lib/libnumpy_core.a",
            "wasi-numpy/lib/libnpymath.a",
            "wasi-numpy/lib/libnumpy_linalg.a",
        ],
        "python_dirs": [
            "wasi-numpy/python/numpy",
        ],
        "dependencies": [],
        "validation": [
            "wasi-numpy/lib/libnumpy_core.a",
            "wasi-numpy/lib/libnumpy_linalg.a",
        ],
    },

}

# PANDAS WASI STATUS: NOT WORKING
# ================================
# Pandas WASI support is blocked by fundamental incompatibilities:
#
# 1. Symbol Conflicts: Pandas vendors old NumPy datetime code (pre-2.x API) with
#    different function signatures. When statically linked with NumPy 2.x, the
#    wrong function is called at runtime, causing crashes.
#    Example: get_datetime_metadata_from_dtype(i32)->i32 vs (i32,i32)->void
#
# 2. Tight Coupling: The pandas Python code expects its C extensions to exist.
#    Even bundling just the Python files fails because imports like
#    pandas._libs.lib are mandatory and can't be stubbed easily.
#
# 3. The build script (scripts/build-pandas-wasi.sh) successfully produces
#    libpandas_libs.a and libpandas_tslibs.a, but these cannot be safely used
#    alongside NumPy due to the datetime ABI mismatch.
#
# POSSIBLE SOLUTIONS (not implemented):
# - Wait for pandas to update vendored datetime code for NumPy 2.x compatibility
# - Use objcopy or similar to rename conflicting symbols at link time
# - Build against an older NumPy version that matches pandas' vendored code
# - Create Python stubs for all pandas._libs modules (complex, partial functionality)


def get_all_extensions(requested: list[str]) -> list[str]:
    """Resolve all extensions including dependencies (topological order)."""
    result = []
    visited = set()

    def visit(ext: str):
        if ext in visited:
            return
        visited.add(ext)
        if ext not in EXTENSIONS:
            raise ValueError(f"Unknown extension: {ext}")
        # Visit dependencies first
        for dep in EXTENSIONS[ext].get("dependencies", []):
            visit(dep)
        result.append(ext)

    for ext in requested:
        visit(ext)

    return result


def get_all_modules(extensions: list[str]) -> list[tuple[str, str]]:
    """Get all modules to register for the given extensions."""
    modules = []
    for ext in get_all_extensions(extensions):
        modules.extend(EXTENSIONS[ext].get("modules", []))
    return modules


def get_all_libraries(extensions: list[str]) -> list[str]:
    """Get all library paths for the given extensions.

    Returns libraries in dependency order (dependencies first, main extension last).
    This ensures numpy's datetime functions are used (correct signature) and pandas
    accesses its vendored versions through the PandasDateTimeAPI capsule at runtime.
    """
    libraries = []
    for ext in get_all_extensions(extensions):
        libraries.extend(EXTENSIONS[ext].get("libraries", []))
    return libraries


def get_all_python_dirs(extensions: list[str]) -> list[str]:
    """Get all Python directories to bundle for the given extensions."""
    dirs = []
    for ext in get_all_extensions(extensions):
        dirs.extend(EXTENSIONS[ext].get("python_dirs", []))
    return dirs


def get_validation_files(extensions: list[str]) -> dict[str, list[str]]:
    """Get validation files for each extension."""
    result = {}
    for ext in get_all_extensions(extensions):
        result[ext] = EXTENSIONS[ext].get("validation", [])
    return result
