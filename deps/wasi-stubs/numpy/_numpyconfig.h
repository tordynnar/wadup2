/*
 * NumPy configuration for WASI/wasm32 target.
 *
 * This file replaces the native-build-generated _numpyconfig.h with
 * values appropriate for wasm32 (32-bit pointers, 4-byte long, etc.)
 *
 * Based on: Python WASI pyconfig.h values
 *   SIZEOF_LONG = 4
 *   SIZEOF_DOUBLE = 8
 *   SIZEOF_LONG_DOUBLE = 16
 *   SIZEOF_VOID_P = 4
 */

/* #undef NPY_HAVE_ENDIAN_H */

#define NPY_SIZEOF_SHORT 2
#define NPY_SIZEOF_INT 4
#define NPY_SIZEOF_LONG 4           /* WASI: 4 bytes, not 8 like arm64 */
#define NPY_SIZEOF_FLOAT 4
#define NPY_SIZEOF_COMPLEX_FLOAT 8
#define NPY_SIZEOF_DOUBLE 8
#define NPY_SIZEOF_COMPLEX_DOUBLE 16
#define NPY_SIZEOF_LONGDOUBLE 16    /* WASI libc supports 128-bit long double */
#define NPY_SIZEOF_COMPLEX_LONGDOUBLE 32
#define NPY_SIZEOF_PY_INTPTR_T 4    /* WASI: 4 bytes (32-bit pointers) */
#define NPY_SIZEOF_INTP 4           /* WASI: 4 bytes (32-bit) */
#define NPY_SIZEOF_UINTP 4          /* WASI: 4 bytes (32-bit) */
#define NPY_SIZEOF_WCHAR_T 4
#define NPY_SIZEOF_OFF_T 8          /* Usually 64-bit even on 32-bit */
#define NPY_SIZEOF_PY_LONG_LONG 8
#define NPY_SIZEOF_LONGLONG 8

/*
 * Defined to 1 or 0. Note that Pyodide hardcodes NPY_NO_SMP (and other defines
 * in this header) for better cross-compilation, so don't rename them without a
 * good reason.
 */
#define NPY_NO_SMP 1                /* No threading in WASI */

#define NPY_VISIBILITY_HIDDEN __attribute__((visibility("hidden")))
#define NPY_ABI_VERSION 0x02000000
#define NPY_API_VERSION 0x00000015  /* 2.4 API */

#ifndef __STDC_FORMAT_MACROS
#define __STDC_FORMAT_MACROS 1
#endif
