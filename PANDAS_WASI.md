# Pandas WASI Support

**Status: BLOCKED** - Build succeeds, runtime crashes during import

This document describes the current state of pandas support in WADUP's Python WASI builds.

## Current Status

| Component | Status |
|-----------|--------|
| NumPy 2.4.0 | **Working** |
| Pandas 2.3.3 Build | **Working** |
| Pandas 2.3.3 Runtime | **Blocked** - Thread state crash |

## What Works

### NumPy 2.4.0

NumPy is fully functional in WASI builds:

```
Starting numpy test...
Python version: 3.13.7
Importing numpy...
NumPy version: 2.4.0
Created array: [1 2 3 4 5]
Sum: 15
```

Features working:
- Core array operations (`numpy._core._multiarray_umath`)
- Linear algebra (`numpy.linalg._umath_linalg`) with LAPACK lite
- Basic mathematical operations

### Pandas Build

The pandas 2.3.3 build completes successfully with all C extensions:

**Libraries built:**
- `libpandas_libs.a` (~15MB) - Core pandas C extensions
- `libpandas_tslibs.a` (~6MB) - Time series C extensions

**Modules compiled (47 total):**

`pandas._libs.*`:
- lib, hashtable, algos, arrays, groupby, hashing, index, indexing
- internals, interval, join, missing, ops, ops_dispatch, parsers
- pandas_parser, pandas_datetime, properties, reshape, sparse
- testing, tslib, writers, byteswap, sas, json

`pandas._libs.tslibs.*`:
- base, ccalendar, conversion, dtypes, fields, nattype, np_datetime
- offsets, parsing, period, strptime, timedeltas, timestamps
- timezones, tzconversion, vectorized

## The Blocking Issue

### Error Message

```
Fatal Python error: _PyThreadState_Attach: non-NULL old thread state
Python runtime state: initialized

Extension modules: numpy._core._multiarray_umath, numpy.linalg._umath_linalg,
pandas._libs.interval, pandas._libs.hashtable, pandas._libs.missing,
pandas._libs.tslibs.ccalendar, pandas._libs.tslibs.np_datetime,
pandas._libs.tslibs.dtypes, pandas._libs.tslibs.conversion,
pandas._libs.tslibs.base, pandas._libs.tslibs.offsets,
pandas._libs.tslibs.timestamps, pandas._libs.tslibs.nattype,
pandas._libs.tslibs.timezones, pandas._libs.tslibs.fields,
pandas._libs.tslibs.timedeltas, pandas._libs.tslibs.tzconversion (total: 17)
```

### Root Cause

The crash occurs due to a conflict between Python's thread state management and WASI's single-threaded environment.

1. **GIL Operations in Cython**: Pandas' Cython modules extensively use `with nogil:` blocks, which expand to `Py_BEGIN_ALLOW_THREADS` / `Py_END_ALLOW_THREADS` macros.

2. **Thread State Tracking**: These macros manipulate Python's internal thread state via `_PyThreadState_Attach` and `_PyThreadState_Detach`.

3. **WASI Single-Thread Conflict**: In WASI's single-threaded environment, the thread state tracking encounters an inconsistent state during module initialization, causing the fatal error.

4. **Timing**: The crash occurs after 17 extension modules load successfully, during the import of subsequent modules (likely `parsing` or `vectorized`).

### Technical Details

The error `_PyThreadState_Attach: non-NULL old thread state` means:
- Python expects the current thread's state pointer to be NULL when attaching a new state
- Instead, it finds a non-NULL value, indicating the thread state wasn't properly detached
- This is a safety check in Python's threading implementation that prevents state corruption

In normal multi-threaded Python:
```c
Py_BEGIN_ALLOW_THREADS  // Saves and clears thread state
// ... nogil code ...
Py_END_ALLOW_THREADS    // Restores thread state
```

In WASI, these operations may not properly handle the single-threaded case, leading to state tracking issues during nested module imports.

## Build Configuration

### Compilation Flags

Both NumPy and Pandas are built with:
- `-DNDEBUG` - Disable assertions (fixes NumPy's GIL assertions)
- `-DCYTHON_WITHOUT_ASSERTIONS=1` - Disable Cython assertions
- `-DCYTHON_PEP489_MULTI_PHASE_INIT=0` - Disable multi-phase init
- `-D__EMSCRIPTEN__=1` - Enable Emscripten compatibility mode

### Build Output

```
deps/wasi-pandas/
├── lib/
│   ├── libpandas_libs.a      # Core pandas._libs modules (~15MB)
│   └── libpandas_tslibs.a    # pandas._libs.tslibs modules (~6MB)
└── python/
    └── pandas/               # Pure Python source files
```

## WASI Stubs Created

Several stubs were created to support pandas in WASI:

### subprocess.py
`deps/wasi-stubs/python/subprocess.py`

WASI cannot spawn processes, so subprocess is stubbed:
```python
def run(*args, **kwargs):
    raise NotImplementedError("subprocess.run is not available in WASI builds")
```

### numpy.random
`deps/wasi-stubs/python/numpy/random/`

NumPy's random module isn't compiled for WASI. Stubs provide:
- `Generator` - Stub random number generator
- `BitGenerator` - Stub bit generator base class
- `RandomState` - Stub for legacy RandomState

### sysconfigdata
`deps/wasi-stubs/python/_sysconfigdata__wasi_wasm32-wasi.py`

Provides WASI-specific sysconfig data:
```python
build_time_vars = {
    'HOST_GNU_TYPE': 'wasm32-wasi',
    'SOABI': 'cpython-313-wasm32-wasi',
    'EXT_SUFFIX': '.cpython-313-wasm32-wasi.so',
    ...
}
```

## Frozen Stdlib Modules Added

The following modules were added to Python's frozen stdlib for pandas support:
- `shutil` - File operations
- `_strptime` - Date/time parsing
- `inspect`, `dis`, `opcode`, `_opcode_metadata`, `token`, `tokenize` - Introspection
- `<sysconfig.*>` - System configuration

## Previous Issues Resolved

### DateTime ABI Conflict (Fixed)

Previously, pandas' vendored NumPy datetime code conflicted with NumPy 2.x. This was resolved by:
1. Compiling pandas' vendored datetime files (`np_datetime.c`, `np_datetime_strings.c`)
2. Compiling `pd_datetime.c` and `date_conversions.c` for the `pandas_datetime` module
3. Using `--allow-multiple-definition` linker flag
4. Linking NumPy before pandas (NumPy's symbols take precedence)

### GIL Assertion Failures (Fixed)

NumPy's allocator had `assert(PyGILState_Check())` calls that failed in WASI. Fixed by adding `-DNDEBUG` to disable assertions.

### Missing Modules (Fixed)

Various missing frozen stdlib modules and stubs were added as discovered during testing.

## Potential Solutions for Thread State Issue

### 1. Stub Out Threading Macros

Add to WASI compilation flags:
```bash
-DPy_BEGIN_ALLOW_THREADS=
-DPy_END_ALLOW_THREADS=
```

**Pros**: Simple to implement
**Cons**: May cause issues with code that relies on these being actual operations

### 2. Patch Python's Thread State for WASI

Modify Python's `_PyThreadState_Attach` to handle WASI's single-threaded environment:

```c
#ifdef __wasi__
// Skip thread state validation in single-threaded WASI
#endif
```

**Pros**: Clean solution at the right level
**Cons**: Requires Python source modification and rebuild

### 3. Use Python Free-Threading Mode

Python 3.13 has experimental `--disable-gil` mode. Building Python with this might work better with WASI.

**Pros**: Modern approach aligned with Python's direction
**Cons**: Experimental, may have other issues

### 4. Post-Process Cython Output

Modify the generated C code to remove or stub `Py_BEGIN_ALLOW_THREADS` calls:

```bash
sed -i 's/Py_BEGIN_ALLOW_THREADS/\/\*Py_BEGIN_ALLOW_THREADS\*\//g' *.c
```

**Pros**: Targeted fix
**Cons**: Fragile, needs to be applied on each rebuild

### 5. Selective Module Loading

Only load pandas modules that don't trigger the threading issue, providing partial functionality.

**Pros**: Partial functionality available
**Cons**: Limited pandas features

## Files Modified

### Build Scripts
| File | Changes |
|------|---------|
| `scripts/build-numpy-wasi.sh` | Added `-DNDEBUG` to WASI_CFLAGS |
| `scripts/build-pandas-wasi.sh` | Added `-DNDEBUG`, `-DCYTHON_WITHOUT_ASSERTIONS`, datetime module compilation |
| `scripts/build-python-wasi.sh` | Added frozen stdlib modules (shutil, _strptime, inspect, etc.) |
| `scripts/build-python-project.py` | Fixed single-file module bundling, WASI stubs copying |

### Extension Registry
| File | Changes |
|------|---------|
| `extensions/__init__.py` | Added `pandas_parser`, `pandas_datetime` modules |

### New Stubs
| File | Purpose |
|------|---------|
| `deps/wasi-stubs/python/subprocess.py` | Subprocess stub for WASI |
| `deps/wasi-stubs/python/numpy/random/__init__.py` | NumPy random stub |
| `deps/wasi-stubs/python/numpy/random/_generator.py` | Generator/BitGenerator/RandomState stubs |
| `deps/wasi-stubs/python/_sysconfigdata__wasi_wasm32-wasi.py` | WASI sysconfigdata |

## Testing

### NumPy Test (Working)

```bash
./scripts/build-python-project.py /path/to/test-numpy
wasmtime run --dir /tmp/wasi-root::/ target/test_numpy.wasm
```

### Pandas Test (Crashes)

```bash
./scripts/build-python-project.py /path/to/test-pandas
wasmtime run --dir /tmp/wasi-root::/ target/test_pandas.wasm
# Crashes with thread state error after loading 17 modules
```

## Next Steps

1. **Test threading macro stubbing** - Try `-DPy_BEGIN_ALLOW_THREADS=` `-DPy_END_ALLOW_THREADS=`
2. **Profile module loading** - Identify exactly which module init causes the crash
3. **Test Python free-threading** - Build Python 3.13 with `--disable-gil`
4. **Upstream discussion** - Engage with CPython and Cython maintainers about WASI support

## References

- [CPython WASI Support](https://github.com/python/cpython/tree/main/Tools/wasm)
- [Cython Threading Documentation](https://cython.readthedocs.io/en/latest/src/userguide/parallelism.html)
- [WASI Threading Proposal](https://github.com/WebAssembly/wasi-threads)
- [Python GIL Removal PEP 703](https://peps.python.org/pep-0703/)
- [NumPy 2.0 Migration Guide](https://numpy.org/doc/stable/numpy_2_0_migration_guide.html)
