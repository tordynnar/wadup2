"""C extension registry for WADUP Python builds.

Each extension defines:
- modules: List of (module_name, init_function) tuples to register with PyImport_AppendInittab
- libraries: List of static library paths (relative to deps/)
- python_dirs: List of pure Python directories to bundle (relative to deps/)
- dependencies: List of other extension names this depends on
- validation: List of files that must exist (relative to deps/) to consider this extension built
"""

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
        ],
        "dependencies": [],
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

    "pandas": {
        "modules": [
            # pandas._libs modules
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
            ("pandas._libs.pandas_parser", "PyInit_pandas_parser"),
            ("pandas._libs.pandas_datetime", "PyInit_pandas_datetime"),
            ("pandas._libs.properties", "PyInit_properties"),
            ("pandas._libs.reshape", "PyInit_reshape"),
            ("pandas._libs.sparse", "PyInit_sparse"),
            ("pandas._libs.testing", "PyInit_testing"),
            ("pandas._libs.tslib", "PyInit_tslib"),
            ("pandas._libs.writers", "PyInit_writers"),
            ("pandas._libs.byteswap", "PyInit_byteswap"),
            ("pandas._libs.sas", "PyInit_sas"),
            ("pandas._libs.json", "PyInit_json"),
            # pandas._libs.window modules
            ("pandas._libs.window.aggregations", "PyInit_aggregations"),
            ("pandas._libs.window.indexers", "PyInit_indexers"),
            # pandas._libs.tslibs modules
            ("pandas._libs.tslibs.base", "PyInit_base"),
            ("pandas._libs.tslibs.ccalendar", "PyInit_ccalendar"),
            ("pandas._libs.tslibs.conversion", "PyInit_conversion"),
            ("pandas._libs.tslibs.dtypes", "PyInit_dtypes"),
            ("pandas._libs.tslibs.fields", "PyInit_fields"),
            ("pandas._libs.tslibs.nattype", "PyInit_nattype"),
            ("pandas._libs.tslibs.np_datetime", "PyInit_np_datetime"),
            ("pandas._libs.tslibs.offsets", "PyInit_offsets"),
            ("pandas._libs.tslibs.parsing", "PyInit_parsing"),
            ("pandas._libs.tslibs.period", "PyInit_period"),
            ("pandas._libs.tslibs.strptime", "PyInit_strptime"),
            ("pandas._libs.tslibs.timedeltas", "PyInit_timedeltas"),
            ("pandas._libs.tslibs.timestamps", "PyInit_timestamps"),
            ("pandas._libs.tslibs.timezones", "PyInit_timezones"),
            ("pandas._libs.tslibs.tzconversion", "PyInit_tzconversion"),
            ("pandas._libs.tslibs.vectorized", "PyInit_vectorized"),
        ],
        "libraries": [
            "wasi-pandas/lib/libpandas_libs.a",
            "wasi-pandas/lib/libpandas_tslibs.a",
        ],
        # C++ runtime needed for aggregations module (uses exceptions)
        "requires_cxx_runtime": True,
        "python_dirs": [
            "wasi-pandas/python/pandas",
        ],
        "dependencies": ["numpy"],
        "validation": [
            "wasi-pandas/lib/libpandas_libs.a",
            "wasi-pandas/lib/libpandas_tslibs.a",
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
