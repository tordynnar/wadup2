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
            # Core array module (essential)
            ("numpy._core._multiarray_umath", "PyInit__multiarray_umath"),
            # Random number generation (core modules only)
            ("numpy.random._common", "PyInit__common"),
            ("numpy.random.bit_generator", "PyInit_bit_generator"),
            ("numpy.random._bounded_integers", "PyInit__bounded_integers"),
            ("numpy.random._mt19937", "PyInit__mt19937"),
            ("numpy.random._generator", "PyInit__generator"),
            ("numpy.random.mtrand", "PyInit_mtrand"),
            # Linear algebra (basic, uses internal fallback or OpenBLAS)
            ("numpy.linalg._umath_linalg", "PyInit__umath_linalg"),
            ("numpy.linalg.lapack_lite", "PyInit_lapack_lite"),
            # FFT
            ("numpy.fft._pocketfft_umath", "PyInit__pocketfft_umath"),
        ],
        "libraries": [
            "wasi-numpy/lib/libnumpy_core.a",
            "wasi-numpy/lib/libnpymath.a",
            "wasi-numpy/lib/libnpyrandom.a",
            # OpenBLAS is optional - if present, link it
            # "wasi-openblas/lib/libopenblas.a",
        ],
        "python_dirs": [
            "wasi-numpy/python/numpy",
        ],
        "dependencies": [],
        "validation": [
            "wasi-numpy/lib/libnumpy_core.a",
        ],
    },

    "pandas": {
        "modules": [
            # Core pandas C extensions
            ("pandas._libs.lib", "PyInit_lib"),
            ("pandas._libs.hashtable", "PyInit_hashtable"),
            ("pandas._libs.algos", "PyInit_algos"),
            ("pandas._libs.arrays", "PyInit_arrays"),
            ("pandas._libs.groupby", "PyInit_groupby"),
            ("pandas._libs.hashing", "PyInit_hashing"),
            ("pandas._libs.index", "PyInit_index"),
            ("pandas._libs.indexing", "PyInit_indexing"),
            ("pandas._libs.internals", "PyInit_internals"),
            ("pandas._libs.interval", "PyInit_interval"),
            ("pandas._libs.join", "PyInit_join"),
            ("pandas._libs.missing", "PyInit_missing"),
            ("pandas._libs.ops", "PyInit_ops"),
            ("pandas._libs.ops_dispatch", "PyInit_ops_dispatch"),
            ("pandas._libs.parsers", "PyInit_parsers"),
            ("pandas._libs.properties", "PyInit_properties"),
            ("pandas._libs.reshape", "PyInit_reshape"),
            ("pandas._libs.sparse", "PyInit_sparse"),
            ("pandas._libs.tslib", "PyInit_tslib"),
            ("pandas._libs.writers", "PyInit_writers"),
        ],
        "libraries": [
            "wasi-pandas/lib/libpandas_libs.a",
        ],
        "python_dirs": [
            "wasi-pandas/python/pandas",
        ],
        "dependencies": ["numpy"],
        "validation": [
            "wasi-pandas/lib/libpandas_libs.a",
        ],
    },
}


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
    """Get all library paths for the given extensions."""
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
