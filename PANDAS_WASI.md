# Pandas WASI Support

**Status: NOT WORKING**

This document describes the work done to add pandas support to WADUP's Python WASI builds, the blockers encountered, and potential solutions.

## Overview

Pandas is a popular Python data analysis library that relies heavily on C extensions for performance. These extensions are compiled from Cython (.pyx) files and include modules for algorithms, hash tables, datetime handling, and more.

## Build Infrastructure

The build infrastructure for pandas WASI is complete and functional:

### Build Script

`scripts/build-pandas-wasi.sh` implements a two-stage build process:

1. **Native Build Stage**: Uses meson to build pandas natively, which:
   - Runs Cython to generate C files from .pyx sources
   - Generates NumPy API headers needed for compilation
   - Produces ~51 C extension modules

2. **WASI Cross-Compilation Stage**: Compiles the generated C files for WASI:
   - Uses wasi-sdk's clang with appropriate flags
   - Links against WASI-compiled NumPy headers
   - Produces two static libraries:
     - `libpandas_libs.a` (~18MB, 31 object files)
     - `libpandas_tslibs.a` (~6.6MB, 16 object files)

### Build Output

```
deps/wasi-pandas/
├── lib/
│   ├── libpandas_libs.a      # Core pandas._libs modules
│   └── libpandas_tslibs.a    # pandas._libs.tslibs modules
└── python/
    └── pandas/               # Pure Python source files
```

## The Problem

Pandas cannot be used with WADUP due to fundamental incompatibilities between pandas' vendored NumPy datetime code and NumPy 2.x.

### Symbol Conflicts

Pandas vendors old NumPy datetime code (pre-2.x API) in `pandas/_libs/src/vendored/numpy/datetime/`. This vendored code has different function signatures than NumPy 2.x:

| Function | NumPy 2.x Signature | Pandas Vendored Signature |
|----------|---------------------|---------------------------|
| `get_datetime_metadata_from_dtype` | `(dtype*) -> metadata*` | `(dtype*, metadata*) -> void` |

When both are statically linked into a single WASM module:
- The linker keeps only one version of each symbol
- At runtime, code compiled against one API calls the other
- This causes crashes due to ABI mismatch

### Crash Analysis

```
wasm backtrace:
    0: 0xdd3f4e - <unknown>!<wasm function 18239>
    ...
```

The crash occurs during module initialization when pandas code attempts to use datetime functions with the wrong calling convention.

### Python Code Dependencies

Even without C extensions, pandas Python code fails because:
- `pandas/_libs/__init__.py` unconditionally imports C extension modules
- Imports like `from pandas._libs import lib` have no pure-Python fallback
- The library is designed assuming C extensions are always available

## What Was Tried

### 1. Skipping Vendored Datetime Files
```bash
# Skip numpy datetime vendored files - they conflict with NumPy 2.x
if [[ "$src" == *"numpy/datetime"* ]]; then
    continue
fi
```
**Result**: Still crashes because pandas modules reference these symbols

### 2. Disabling Datetime Modules
Removed `pandas._libs.tslib` and `pandas._libs.pandas_datetime` from module registration.

**Result**: Other modules (like `index.pyx`) still contain datetime code internally

### 3. Minimal Module Set
Reduced to just `hashing` and `json` modules that shouldn't need datetime.

**Result**: The entire library is still linked, bringing in all datetime code

### 4. Python-Only Bundle
Disabled all C extensions and libraries, only bundling Python files.

**Result**: Python import fails because `pandas._libs.lib` is required

### 5. Link Order Changes
Tried linking NumPy before pandas to ensure NumPy's datetime symbols are used.

**Result**: Doesn't help because pandas code expects its vendored API signatures

## Potential Solutions

### 1. Wait for Upstream Fix
Pandas may update their vendored NumPy datetime code for NumPy 2.x compatibility. This is the cleanest solution but depends on upstream timeline.

### 2. Symbol Renaming
Use `objcopy --redefine-sym` or similar to rename conflicting symbols in one library:
```bash
objcopy --redefine-sym get_datetime_metadata_from_dtype=pandas_get_datetime_metadata_from_dtype \
    libpandas_libs.a libpandas_libs_renamed.a
```
This would require renaming all conflicting symbols and updating references.

### 3. Older NumPy Version
Build NumPy 1.x that matches pandas' vendored datetime API. This sacrifices NumPy 2.x features but may provide compatibility.

### 4. Python Stubs
Create stub modules that satisfy `pandas._libs` imports without actual C code:
```python
# pandas/_libs/lib.py (stub)
def maybe_convert_objects(*args, **kwargs):
    raise NotImplementedError("C extension not available in WASI")
```
This would provide partial functionality for code paths that don't need C extensions.

### 5. Pandas Fork
Fork pandas and update the vendored datetime code to match NumPy 2.x API. This is significant maintenance burden.

## Files Modified

| File | Changes |
|------|---------|
| `scripts/build-pandas-wasi.sh` | Complete build script for pandas WASI |
| `scripts/build-python-project.py` | Added `--allow-multiple-definition` for khash symbols |
| `extensions/__init__.py` | Documentation about pandas status (removed from active extensions) |

## Related Work

- NumPy WASI works correctly, including `numpy.linalg` with LAPACK lite
- lxml WASI works correctly
- The pandas build infrastructure can be reused once the datetime conflict is resolved

## References

- [NumPy 2.0 Migration Guide](https://numpy.org/doc/stable/numpy_2_0_migration_guide.html)
- [Pandas C Extension Source](https://github.com/pandas-dev/pandas/tree/main/pandas/_libs)
- [WASI SDK](https://github.com/WebAssembly/wasi-sdk)
