# Pandas WASI Support

**Status: WORKING** ✅

Pandas 2.3.3 successfully runs in WASI with full DataFrame functionality.

```
Python version: 3.13.7
Importing pandas...
Pandas version: 2.3.3
SUCCESS: Pandas imported!
Created DataFrame with shape: (3, 2)
   a  b
0  1  4
1  2  5
2  3  6
```

## Summary

Getting pandas to work in WASI required solving several interconnected issues:

1. **Thread state management** - CPython's GIL operations fail in single-threaded WASI
2. **Missing stdlib modules** - Several frozen modules needed for pandas import chain
3. **Unavailable system modules** - ctypes and mmap require OS features not in WASI
4. **C++ runtime** - Pandas window module uses C++ exceptions
5. **Nested module compilation** - Window submodule wasn't being compiled

## The Solution: Pyodide-Style GIL Patches

The core fix follows Pyodide's approach: make GIL-related operations no-ops in single-threaded WASI.

### CPython Patches

Two patches are applied during Python build (`scripts/patches/`):

#### 1. `cpython-wasi-threading.patch`

Stubs `PyEval_SaveThread` and `PyEval_RestoreThread` in `Python/ceval_gil.c`:

```c
#ifdef __wasi__
/* WASI: Single-threaded environment - GIL operations are no-ops.
   _PyThreadState_Detach/_Attach fail because WASI lacks threading primitives.
   Instead, we keep the thread state attached and skip GIL manipulation entirely.
   This matches Pyodide/Emscripten's approach where threading stubs make these no-ops. */

PyThreadState *
PyEval_SaveThread(void)
{
    return _PyThreadState_GET();
}

void
PyEval_RestoreThread(PyThreadState *tstate)
{
    (void)tstate;  /* Thread state remains attached in single-threaded WASI */
}
#else  /* !__wasi__ */
// ... normal implementation ...
#endif
```

#### 2. `cpython-wasi-gilstate.patch`

Stubs `PyGILState_Ensure` and `PyGILState_Release` in `Python/pystate.c`:

```c
#ifdef __wasi__
/* WASI: Single-threaded environment - GIL state operations are no-ops.
   We always "hold" the GIL since there's only one thread. */

PyGILState_STATE
PyGILState_Ensure(void)
{
    return PyGILState_LOCKED;  /* Always locked in single-threaded WASI */
}

void
PyGILState_Release(PyGILState_STATE oldstate)
{
    (void)oldstate;  /* No-op in single-threaded WASI */
}
#else  /* !__wasi__ */
// ... normal implementation ...
#endif
```

### Why This Works

Pandas (and NumPy) Cython modules use `with nogil:` blocks extensively, which expand to:

```c
Py_BEGIN_ALLOW_THREADS  // Calls PyEval_SaveThread() -> _PyThreadState_Detach()
// ... nogil code ...
Py_END_ALLOW_THREADS    // Calls PyEval_RestoreThread() -> _PyThreadState_Attach()
```

In normal multi-threaded Python, these operations:
1. Release the GIL so other threads can run
2. Track thread state for proper GIL reacquisition

In WASI's single-threaded environment:
- There's only one thread, so GIL management is meaningless
- `_PyThreadState_Attach` fails because it expects NULL thread state
- The patches make these operations no-ops, matching Pyodide's approach

## WASI Module Stubs

Two modules required stubs because they depend on OS features unavailable in WASI:

### ctypes Stub (`deps/wasi-stubs/python/ctypes/__init__.py`)

ctypes requires `dlopen()` for dynamic library loading, which WASI doesn't support:

```python
class WASINotSupportedError(Exception):
    """Raised when attempting to use ctypes functionality in WASI."""
    pass

# Stub type definitions
class c_void_p: pass
class c_int: pass
class Structure:
    _fields_ = []
# ... etc

class CDLL:
    def __init__(self, name, ...):
        raise WASINotSupportedError(f"Cannot load library '{name}': ctypes not supported in WASI")

# Stub loaders
cdll = LibraryLoader(CDLL)
pydll = LibraryLoader(CDLL)
```

### mmap Stub (`deps/wasi-stubs/python/mmap.py`)

mmap requires memory mapping syscalls not fully available in WASI:

```python
class mmap:
    def __init__(self, fileno, length, ...):
        raise WASINotSupportedError(
            "mmap is not supported in WASI builds - memory mapping requires "
            "syscalls that are not available in the WASI sandbox"
        )
```

## Frozen Stdlib Modules

The following modules were added to Python's frozen stdlib in `scripts/build-python-wasi.sh`:

| Module | Required By |
|--------|-------------|
| `glob` | `pathlib` (used by pandas) |
| `tempfile` | `pandas._testing.contexts` |
| `<urllib.*>` | `pandas.io.common` |
| `<zipfile._path.*>` | `zipfile` submodule in Python 3.13 |

These are in addition to previously added modules:
- `shutil`, `_strptime`, `inspect`, `dis`, `opcode`, `token`, `tokenize`
- `<sysconfig.*>`, `<pathlib.*>`, `<collections.*>`, etc.

## Pandas Build Fixes

### Window Module Compilation

The pandas `_libs/window/` subdirectory contains:
- `indexers.pyx` (C)
- `aggregations.pyx` (C++)

The build script needed fixes to:

1. **Find nested modules** - Changed `find -maxdepth 1` to `find` (no depth limit)
2. **Compile C++ files** - Added `.cpp` file handling alongside `.c`
3. **Enable exceptions** - Removed `-fno-exceptions` (Cython-generated C++ uses try/catch)
4. **Add include paths** - Added `-I$NATIVE_BUILD/pandas/_libs/window`

### C++ Runtime Linking

The aggregations module uses C++ exceptions, requiring the C++ runtime:

```python
# extensions/__init__.py
"pandas": {
    # ...
    "requires_cxx_runtime": True,  # Links libc++.a and libc++abi.a
}
```

```python
# build-python-project.py
if needs_cxx_runtime:
    wasi_lib_dir = wasi_sysroot / "lib" / "wasm32-wasi"
    link_cmd.extend([
        str(wasi_lib_dir / "libc++.a"),
        str(wasi_lib_dir / "libc++abi.a"),
    ])
```

### Extension Registration

Window modules added to `extensions/__init__.py`:

```python
("pandas._libs.window.aggregations", "PyInit_aggregations"),
("pandas._libs.window.indexers", "PyInit_indexers"),
```

## Complete File Changes

### New Files Created

| File | Purpose |
|------|---------|
| `scripts/patches/cpython-wasi-threading.patch` | Stub PyEval_SaveThread/RestoreThread |
| `scripts/patches/cpython-wasi-gilstate.patch` | Stub PyGILState_Ensure/Release |
| `deps/wasi-stubs/python/ctypes/__init__.py` | ctypes stub for WASI |
| `deps/wasi-stubs/python/mmap.py` | mmap stub for WASI |
| `deps/wasi-stubs/cxx_exception_stubs.c` | C++ exception symbol stubs (unused, libc++ used instead) |
| `tests/test-pandas/` | Test project for pandas |

### Modified Files

| File | Changes |
|------|---------|
| `scripts/build-python-wasi.sh` | Apply patches, add frozen modules |
| `scripts/build-pandas-wasi.sh` | Fix nested module compilation, C++ support |
| `scripts/build-python-project.py` | Add C++ runtime linking |
| `extensions/__init__.py` | Add window modules, `requires_cxx_runtime` flag |

## Build Output

```
deps/wasi-pandas/
├── lib/
│   ├── libpandas_libs.a      # ~21MB (53 object files including window modules)
│   └── libpandas_tslibs.a    # ~6MB (16 object files)
└── python/
    └── pandas/               # Pure Python source files
```

## Testing

### Build and Run

```bash
# Build Python with patches
./scripts/build-python-wasi.sh

# Build pandas
./scripts/build-pandas-wasi.sh

# Build test project
./scripts/build-python-project.py tests/test-pandas

# Run
wasmtime run --dir /tmp/wasi-root::/ tests/test-pandas/target/test_pandas.wasm
```

### Test Output

```
Could not find platform independent libraries <prefix>
Could not find platform dependent libraries <exec_prefix>
Starting test...
Python version: 3.13.7 (main, Jan  1 2026, 12:56:17) [Clang 21.1.4-wasi-sdk ...
Importing pandas...
Pandas version: 2.3.3
SUCCESS: Pandas imported!
Created DataFrame with shape: (3, 2)
   a  b
0  1  4
1  2  5
2  3  6
Test complete.
```

## Technical Background

### The Original Problem

```
Fatal Python error: _PyThreadState_Attach: non-NULL old thread state
```

This occurred because:
1. Pandas Cython modules use `with nogil:` extensively
2. These expand to `Py_BEGIN_ALLOW_THREADS` / `Py_END_ALLOW_THREADS`
3. Which call `PyEval_SaveThread()` / `PyEval_RestoreThread()`
4. Which call `_PyThreadState_Detach()` / `_PyThreadState_Attach()`
5. In WASI, the thread state tracking fails during nested module imports

### Pyodide's Approach

Pyodide (Emscripten-based) successfully runs pandas because:
1. Emscripten provides pthread stub implementations
2. These stubs make mutex/lock operations no-ops
3. Thread state operations succeed (even though they do nothing)

WASI lacks these stubs, so we patched CPython directly to achieve the same effect.

### Why NumPy Worked Without Patches

NumPy worked because:
1. `-DNDEBUG` disables `assert(PyGILState_Check())` in NumPy's allocator
2. NumPy's nogil operations don't nest deeply during import
3. Pandas has more complex module initialization that triggers nested GIL operations

## Limitations

While pandas imports and basic operations work, some features may not:

1. **ctypes-dependent features** - Anything requiring foreign function interface
2. **mmap-dependent features** - Memory-mapped file operations
3. **Process spawning** - subprocess is stubbed
4. **NumPy random** - Random number generation is stubbed

## References

- [Pyodide Thread State Management](https://github.com/pyodide/pyodide/blob/main/src/core/stack_switching/pystate.c)
- [CPython WASI Support](https://github.com/python/cpython/tree/main/Tools/wasm)
- [CPython Threading API](https://docs.python.org/3/c-api/init.html#thread-state-and-the-global-interpreter-lock)
- [Cython nogil Documentation](https://cython.readthedocs.io/en/latest/src/userguide/parallelism.html)
- [WASI SDK](https://github.com/WebAssembly/wasi-sdk)
