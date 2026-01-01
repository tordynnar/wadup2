# NumPy WASI Build - Complete Guide

This document describes how NumPy was successfully compiled and run on WASI (WebAssembly System Interface) for the WADUP project. This is similar to what [Pyodide](https://github.com/pyodide/pyodide) does with Emscripten, but targeting WASI instead.

## Summary

NumPy 2.4.0 now works on WASI with the following capabilities:
- Array creation and manipulation
- Mathematical operations (sum, mean, std, min, max)
- Type introspection (shape, dtype)
- Element-wise operations (arithmetic, comparison)
- **Linear algebra** (solve, inv, det, eig, svd, qr, norm, etc.)

**Test results:**
```
numpy_version: 2.4.0
array: [1.0, 2.0, 3.0, 4.0, 5.0]
shape: [5], dtype: float64
sum: 15.0, mean: 3.0, std: 1.414...
status: success
```

## Architecture

### Two-Stage Cross-Compilation

NumPy cannot be directly cross-compiled because its build system:
1. Uses a vendored meson with custom modules (`vendored-meson/meson/meson.py`)
2. Generates C code and headers during the build process
3. Runs native code to create dispatch tables

Our solution uses two stages:

```
┌─────────────────────────────────────────────────────────────────┐
│ Stage 1: Native Build                                           │
│   - Run NumPy's vendored meson on host machine                  │
│   - Generate headers (_numpyconfig.h, config.h, etc.)           │
│   - Generate C code (dispatch files, ufunc tables)              │
│   - Compile to native objects (for code generation only)        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Config Patching                                                  │
│   - Replace _numpyconfig.h with WASI-specific values            │
│   - Patch config.h to remove unavailable features               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Stage 2: WASI Cross-Compilation                                  │
│   - Compile generated C files with WASI SDK clang               │
│   - Include stub implementations for missing features           │
│   - Create static libraries (libnumpy_core.a, libnpymath.a)     │
└─────────────────────────────────────────────────────────────────┘
```

### Files Involved

| File | Purpose |
|------|---------|
| `scripts/build-numpy-wasi.sh` | Main build script |
| `deps/wasi-stubs/numpy/_numpyconfig.h` | WASI type size configuration |
| `deps/wasi-stubs/halffloat_stubs.c` | Half-precision float operations |
| `deps/wasi-stubs/numpy_trig_stubs.c` | Trigonometric ufunc stubs |
| `deps/wasi-stubs/hwy/highway.h` | Disable Highway SIMD |
| `extensions/__init__.py` | C extension registry |

## The Key Problem: Type Size Mismatch

### Discovery

The module compiled successfully but crashed at runtime during NumPy initialization with a WASM trap. After comparing with [Pyodide's approach](https://github.com/numpy/numpy/blob/main/tools/ci/emscripten/emscripten.meson.cross), the problem was identified:

**The native meson build generates `_numpyconfig.h` with host machine type sizes:**
```c
// Generated on arm64 macOS:
#define NPY_SIZEOF_LONG 8           // 64-bit
#define NPY_SIZEOF_PY_INTPTR_T 8    // 64-bit pointers
#define NPY_SIZEOF_INTP 8
#define NPY_SIZEOF_UINTP 8
```

**But WASI/wasm32 has different sizes:**
```c
// What WASI needs:
#define NPY_SIZEOF_LONG 4           // 32-bit
#define NPY_SIZEOF_PY_INTPTR_T 4    // 32-bit pointers
#define NPY_SIZEOF_INTP 4
#define NPY_SIZEOF_UINTP 4
```

This mismatch caused the C code to make incorrect assumptions about memory layout, leading to crashes during type initialization.

### Pyodide's Solution

Looking at Pyodide's [emscripten.meson.cross](https://github.com/numpy/numpy/blob/main/tools/ci/emscripten/emscripten.meson.cross):

```ini
[host_machine]
system = 'emscripten'
cpu_family = 'wasm32'
cpu = 'wasm'
endian = 'little'

[properties]
needs_exe_wrapper = true
skip_sanity_check = true
longdouble_format = 'IEEE_QUAD_LE'  # for numpy
```

Pyodide uses a proper cross-file to generate correct values. Since we're doing a two-stage build (native meson + WASI cross-compile), we instead replace the generated config.

### Our Solution

Created `deps/wasi-stubs/numpy/_numpyconfig.h` with correct WASI values:

```c
// deps/wasi-stubs/numpy/_numpyconfig.h

#define NPY_SIZEOF_SHORT 2
#define NPY_SIZEOF_INT 4
#define NPY_SIZEOF_LONG 4           // WASI: 4 bytes, not 8
#define NPY_SIZEOF_FLOAT 4
#define NPY_SIZEOF_COMPLEX_FLOAT 8
#define NPY_SIZEOF_DOUBLE 8
#define NPY_SIZEOF_COMPLEX_DOUBLE 16
#define NPY_SIZEOF_LONGDOUBLE 16    // WASI libc: 128-bit
#define NPY_SIZEOF_COMPLEX_LONGDOUBLE 32
#define NPY_SIZEOF_PY_INTPTR_T 4    // 32-bit pointers
#define NPY_SIZEOF_INTP 4           // 32-bit
#define NPY_SIZEOF_UINTP 4
#define NPY_SIZEOF_WCHAR_T 4
#define NPY_SIZEOF_OFF_T 8
#define NPY_SIZEOF_PY_LONG_LONG 8
#define NPY_SIZEOF_LONGLONG 8

#define NPY_NO_SMP 1                // No threading in WASI
#define NPY_ABI_VERSION 0x02000000
#define NPY_API_VERSION 0x00000013  // NumPy 2.1 API
```

The build script replaces the generated config:

```bash
# scripts/build-numpy-wasi.sh (lines 130-138)

echo "Replacing _numpyconfig.h with WASI-compatible version..."
NUMPYCONFIG_H="$NATIVE_BUILD/numpy/_core/_numpyconfig.h"
if [ -f "$NUMPYCONFIG_H" ] && [ -f "$DEPS_DIR/wasi-stubs/numpy/_numpyconfig.h" ]; then
    cp "$DEPS_DIR/wasi-stubs/numpy/_numpyconfig.h" "$NUMPYCONFIG_H"
    echo "  Replaced _numpyconfig.h with WASI values (NPY_SIZEOF_LONG=4, etc.)"
fi
```

## Other Required Fixes

### 1. WASM Code Path Detection

NumPy checks `__EMSCRIPTEN__` for WASM support, but WASI SDK only defines `__wasm__`. Solution:

```bash
WASI_CFLAGS="$WASI_CFLAGS -D__EMSCRIPTEN__=1"
```

### 2. Long Double Formatting

WASI libc doesn't support formatting `long double` by default. NumPy uses this in some places. Solution in `scripts/build-python-project.py`:

```python
# NumPy uses long double formatting which requires extra libc support
if "numpy" in c_extensions:
    link_cmd.append("-lc-printscan-long-double")
```

### 3. Half-Float Operations

NumPy's `halffloat.cpp` uses 64-bit assumptions (`BitCast<double, unsigned long>`) that don't work on 32-bit WASI. Created `deps/wasi-stubs/halffloat_stubs.c` with portable C implementations:

```c
// deps/wasi-stubs/halffloat_stubs.c

npy_half npy_float_to_half(float f) {
    union { float f; uint32_t i; } u = { f };
    uint32_t f_bits = u.i;
    // ... bit manipulation to convert float32 to float16
}

int npy_half_lt(npy_half h1, npy_half h2) {
    // Handle NaN, signed zero, and normal comparison
}
```

### 4. Trigonometric Ufunc Stubs

NumPy's ufunc loop functions have a specific signature:
```c
void FUNC(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data)
```

Most are defined in `libnumpy_core.a`, but some (cos, sin for float/double) needed stubs:

```c
// deps/wasi-stubs/numpy_trig_stubs.c

#define UNARY_LOOP \
    char *ip1 = args[0], *op1 = args[1]; \
    npy_intp is1 = steps[0], os1 = steps[1]; \
    npy_intp n = dimensions[0]; \
    for(npy_intp i = 0; i < n; i++, ip1 += is1, op1 += os1)

void FLOAT_cos(char **args, npy_intp const *dimensions,
               npy_intp const *steps, void *data) {
    UNARY_LOOP {
        *(npy_float *)op1 = cosf(*(npy_float *)ip1);
    }
}
```

### 5. Highway SIMD Disabled

NumPy optionally uses Highway for SIMD sorting, which isn't available in WASI:

```c
// deps/wasi-stubs/hwy/highway.h
#define NPY_DISABLE_HIGHWAY_SORT 1
```

### 6. Missing System Headers

WASI doesn't have some headers that the native build detected:

```bash
# Remove defines for headers WASI doesn't have
grep -v "HAVE_XLOCALE_H" "$CONFIG_H" > /tmp/config_patched.h
grep -v "HAVE_BACKTRACE" "$CONFIG_H" > /tmp/config_patched.h
```

### 7. Python Module Patches

Applied via the build script:

**`numpy/__init__.py`** - Remove source directory check that incorrectly triggers in WASM.

**`numpy/linalg/__init__.py`** - Replace with stub module (linalg C extension not compiled):
```python
class LinAlgError(Exception):
    pass

def _not_available(*args, **kwargs):
    raise NotImplementedError("numpy.linalg is not available in WASM builds")

solve = inv = det = eig = svd = ... = _not_available
```

**`numpy/matrixlib/defmatrix.py`** - Make linalg import lazy to avoid circular import.

### 8. Frozen Stdlib Modules

Added to Python WASI build (`scripts/build-python-wasi.sh`):
- `pickle`, `_compat_pickle` - Required by numpy
- `ast` - Required by numpy._core._internal
- `platform` - Required by numpy.lib

## C Extension Registration

In `extensions/__init__.py`:

```python
"numpy": {
    "modules": [
        ("numpy._core._multiarray_umath", "PyInit__multiarray_umath"),
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
},
```

## Build Output

```
Libraries:
-rw-r--r--  39286    libnpymath.a
-rw-r--r--  4898996  libnumpy_core.a  (4.9MB, 131 objects)
-rw-r--r--  2856480  libnumpy_linalg.a (2.8MB, 10 objects)

Final WASM module: ~43MB
```

## numpy.linalg Support

Linear algebra operations are now available! The `_umath_linalg` C extension provides:
- `solve`, `inv`, `det` - Linear systems and matrix operations
- `eig`, `eigh`, `eigvals`, `eigvalsh` - Eigenvalue decomposition
- `svd`, `svdvals` - Singular value decomposition
- `qr`, `cholesky` - Matrix decompositions
- `norm`, `cond`, `matrix_rank` - Matrix norms and condition

### Implementation

The linalg module is built from:
1. **lapack_lite** - NumPy's bundled LAPACK subset (f2c-converted Fortran)
   - `f2c.c`, `f2c_blas.c`, `f2c_*_lapack.c` (9 source files)
2. **umath_linalg.cpp** - Python bindings for LAPACK operations

The build script compiles these separately into `libnumpy_linalg.a`.

## What's NOT Working

These NumPy submodules are not yet available:

| Module | Reason |
|--------|--------|
| `numpy.fft` | Requires pocketfft compilation |
| `numpy.random` | Multiple C extensions needed |

## Comparison with Pyodide

| Aspect | Pyodide | WADUP |
|--------|---------|-------|
| Target | Emscripten/wasm32-emscripten | WASI/wasm32-wasi |
| Build | Proper cross-file for meson | Two-stage (native + cross) |
| Type config | Generated correctly | Replaced after native build |
| BLAS | `-Dallow-noblas=true` | Same |
| Threading | Disabled | Disabled (`NPY_NO_SMP=1`) |

### Why We Can't Use Pyodide's Cross-File Approach

We investigated whether using a meson cross-file (like Pyodide's `emscripten.meson.cross`) would simplify the WASI build. **Conclusion: it doesn't work for WASI.**

#### What We Tried

Created a WASI meson cross-file with:
```ini
[host_machine]
system = 'wasi'
cpu_family = 'wasm32'
cpu = 'wasm32'
endian = 'little'

[properties]
longdouble_format = 'IEEE_QUAD_LE'
skip_sanity_check = true

[binaries]
c = '/path/to/wasi-sdk/bin/clang'
# ... etc
```

Meson setup succeeded and correctly detected WASI type sizes (`sizeof(long)=4`).

#### Why It Failed

The ninja build failed with:
```
pyport.h:399: error: "LONG_BIT definition appears wrong for platform"
```

**Root cause:** Meson's `dependency('python')` in numpy's `meson.build` finds the **native** Python (e.g., macOS arm64), not WASI Python. This adds native Python headers to the include path:

```
-I/Users/.../python3.13/include/python3.13  # Native headers added by meson
```

The native `pyport.h` checks `sizeof(long) * CHAR_BIT == LONG_BIT`. With WASI compiler:
- `sizeof(long) = 4` (32-bit)
- But native headers expect `LONG_BIT = 64`

This is a fundamental meson limitation for cross-compiling Python extensions to a different architecture.

#### Why Pyodide Works

Pyodide uses Emscripten, which has special Python integration:
- Emscripten builds include Emscripten-specific Python headers
- The Python dependency resolves to Emscripten Python, not native Python

For WASI, there's no such integration in meson.

#### Alternatives Considered

1. **Patch numpy's meson.build** - Remove `dependency('python')` calls
   - Too complex, high maintenance burden

2. **Override include paths** - Put WASI Python headers first
   - Meson adds native Python headers after our args

3. **Two-stage build** (our approach)
   - Native build generates code, we cross-compile separately
   - Clean separation, no meson patching needed

The two-stage approach is actually simpler and more maintainable than trying to make meson cross-compilation work.

## References

- [Pyodide NumPy package](https://github.com/pyodide/pyodide/tree/main/packages/numpy)
- [NumPy Emscripten cross-file](https://github.com/numpy/numpy/blob/main/tools/ci/emscripten/emscripten.meson.cross)
- [Pyodide meta.yaml for NumPy](https://github.com/pyodide/pyodide/blob/main/packages/numpy/meta.yaml) - uses `-Dallow-noblas=true`
- [Python WASI pyconfig.h](build/python-wasi/include/pyconfig.h) - source of correct type sizes

## Commits

- `d8d5060` - Add NumPy WASI build infrastructure
- `7984c5d` - Fix NumPy build issues: trig stubs and long double formatting
- `3612998` - Fix NumPy WASI initialization crash with proper type sizes

## Usage

Build NumPy for WASI:
```bash
./scripts/build-numpy-wasi.sh
```

Build a Python project using NumPy:
```bash
# pyproject.toml
[tool.wadup]
entry-point = "my_module"
c-extensions = ["numpy"]

# Build
python3 scripts/build-python-project.py /path/to/project
```
