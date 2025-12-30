# NumPy WASI Build Progress

This document tracks the progress of building NumPy for WebAssembly (WASI) as part of the WADUP project.

## Overview

NumPy is a complex C extension that requires significant work to compile for WASI. This is similar to what Pyodide does with Emscripten, but targeting WASI instead.

## What's Been Accomplished

### 1. Build Script (`scripts/build-numpy-wasi.sh`)

A two-stage build process:

1. **Stage 1: Native build** - Uses NumPy's vendored meson to generate headers and C files
2. **Stage 2: Cross-compile** - Compiles generated files using WASI SDK

Key features:
- Uses `python3 vendored-meson/meson/meson.py` (NumPy 2.x requires their vendored meson for custom 'features' module)
- Adds `-D__EMSCRIPTEN__=1` to trigger NumPy's WASM code paths (WASI SDK only defines `__wasm__`)
- Patches config.h to remove `HAVE_XLOCALE_H` and `HAVE_BACKTRACE` (WASI doesn't have these)
- Compiles 131 object files into `libnumpy_core.a` (4.7MB)

### 2. Stub Implementations

Created in `deps/wasi-stubs/`:

#### `halffloat_stubs.c`
NumPy's `halffloat.cpp` uses 64-bit assumptions (`BitCast<double, unsigned long>`) that don't work on 32-bit WASI. Created C stub implementations for:
- `npy_float_to_half`, `npy_half_to_float`
- `npy_halfbits_to_floatbits`, `npy_floatbits_to_halfbits`
- Comparison functions (`npy_half_lt`, `npy_half_eq`, etc.)
- `npy_half_nextafter`, `npy_half_spacing`, `npy_half_divmod`

#### `numpy_trig_stubs.c`
NumPy's generated dispatch code uses uppercase `FLOAT_`/`DOUBLE_` prefixed function names. Created wrappers for:
- Trigonometric: `FLOAT_cos`, `DOUBLE_sin`, `FLOAT_tan`, etc.
- Hyperbolic: `FLOAT_cosh`, `DOUBLE_sinh`, `FLOAT_tanh`, etc.
- Exponential/Log: `FLOAT_exp`, `DOUBLE_log`, `FLOAT_log2`, etc.
- Power: `FLOAT_sqrt`, `DOUBLE_pow`, `FLOAT_cbrt`, etc.
- Rounding: `FLOAT_ceil`, `DOUBLE_floor`, `FLOAT_trunc`, etc.

#### `hwy/highway.h`
Stub header to disable Highway SIMD sorting (not available in WASI):
```c
#define NPY_DISABLE_HIGHWAY_SORT 1
```

### 3. Python Patches

Applied via the build script:

#### `numpy/__init__.py`
Removed source directory check that was incorrectly triggering:
```python
# Before: try/except around __config__ import with source directory error
# After: Direct import without the check
```

#### `numpy/linalg/__init__.py`
Completely replaced with stub module since `_umath_linalg` C extension is not compiled:
```python
class LinAlgError(Exception):
    pass

def _not_available(*args, **kwargs):
    raise NotImplementedError("numpy.linalg is not available in WASM builds")

matrix_power = solve = inv = det = eig = svd = ... = _not_available
```

#### `numpy/matrixlib/defmatrix.py`
Made linalg import lazy to avoid circular import during initialization.

### 4. Frozen Stdlib Modules

Added to `scripts/build-python-wasi.sh`:
- `pickle`, `_compat_pickle` - Required by numpy
- `ast` - Required by numpy._core._internal
- `platform` - Required by numpy.lib
- `socket` - Common dependency

### 5. Extensions Registry

Updated `extensions/__init__.py`:
```python
"numpy": {
    "modules": [
        ("numpy._core._multiarray_umath", "PyInit__multiarray_umath"),
    ],
    "libraries": [
        "wasi-numpy/lib/libnumpy_core.a",
        "wasi-numpy/lib/libnpymath.a",
    ],
    "python_dirs": [
        "wasi-numpy/python/numpy",
    ],
}
```

## Current Status

### Working
- NumPy source compiles to WASI object files
- Libraries are created (`libnumpy_core.a`, `libnpymath.a`)
- Python patches prevent circular import issues
- WASM module builds successfully (41.8MB)

### Not Working
The module crashes at runtime with:
```
Support for formatting long double values is currently disabled.
To enable it, add -lc-printscan-long-double to the link command.
```

This is followed by a WASM trap, indicating a crash in a C function trying to format a long double value.

## Lessons Learned

### 1. NumPy's Build System
- NumPy 2.x requires their vendored meson (`vendored-meson/meson/meson.py`) for custom modules
- Generated headers are in `.so.p` and `.a.p` directories with platform-specific names
- Many source files are in subdirectories (`textreading/`, `stringdtype/`, `npysort/`)

### 2. WASI vs Emscripten
- NumPy checks `__EMSCRIPTEN__` for WASM support, but WASI SDK only defines `__wasm__`
- Solution: Add `-D__EMSCRIPTEN__=1` to trigger WASM code paths

### 3. Config Header Issues
- Native build generates `config.h` with defines like `HAVE_XLOCALE_H 1`
- WASI doesn't have these headers, but `#ifdef` checks existence, not value
- Solution: Remove the defines entirely with `grep -v`

### 4. C++ Narrowing
- Some C++ files have 64-bit to 32-bit narrowing that fails on WASI
- Solution: Add `-Wno-c++11-narrowing` and `-Wno-tautological-constant-out-of-range-compare`

### 5. Circular Imports
- `numpy._core._multiarray_umath` import triggers the full numpy package initialization
- numpy.linalg requires `_umath_linalg` C extension (not yet compiled)
- Solution: Stub out linalg module completely

### 6. Long Double Formatting
- WASI libc doesn't support formatting long double by default
- NumPy uses long double in some places
- Potential solution: Add `-lc-printscan-long-double` to linker flags

## Next Steps

1. **Fix long double crash**
   - Add `-lc-printscan-long-double` to linker flags
   - Or find and stub the code paths using long double printing

2. **Test basic operations**
   - Once import succeeds, verify array creation, basic math, sum/mean/etc.

3. **Additional C extensions**
   - `numpy.linalg._umath_linalg` - Requires LAPACK lite compilation
   - `numpy.fft._pocketfft_umath` - FFT operations
   - `numpy.random` modules - Random number generation

## Files Modified/Created

### New Files
- `deps/wasi-stubs/halffloat_stubs.c`
- `deps/wasi-stubs/numpy_trig_stubs.c`
- `deps/wasi-stubs/hwy/highway.h`
- `deps/wasi-stubs/numpy_linalg_stub.c`
- `examples/python-numpy-test/` (test project)

### Modified Files
- `scripts/build-numpy-wasi.sh` (major rewrite)
- `scripts/build-python-wasi.sh` (added frozen modules)
- `extensions/__init__.py` (added numpy entry)

## Reference

- [Pyodide NumPy package](https://github.com/pyodide/pyodide/tree/main/packages/numpy)
- [NumPy Emscripten workflow](https://github.com/numpy/numpy/blob/main/.github/workflows/emscripten.yml)
- [NumPy Meson cross-file](https://github.com/numpy/numpy/blob/main/tools/ci/emscripten/emscripten.meson.cross)
