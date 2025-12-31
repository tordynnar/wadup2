/*
 * Stub for array__unique_hash function from unique.cpp
 *
 * The original unique.cpp uses C++ exceptions which aren't fully
 * supported in WASI (the sized delete operator isn't available).
 * This stub provides a fallback that raises NotImplementedError.
 */

#define PY_SSIZE_T_CLEAN
#include <Python.h>

PyObject*
array__unique_hash(PyObject *module, PyObject *const *args,
                   Py_ssize_t len_args, PyObject *kwnames)
{
    (void)module;
    (void)args;
    (void)len_args;
    (void)kwnames;

    PyErr_SetString(PyExc_NotImplementedError,
        "_unique_hash is not available in WASI builds. "
        "Use np.unique() with return_inverse=False instead.");
    return NULL;
}
