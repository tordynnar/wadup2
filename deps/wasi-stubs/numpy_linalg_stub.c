/*
 * Stub implementation for numpy.linalg._umath_linalg
 * This provides a minimal module that registers but does nothing.
 * Actual linear algebra operations will fail with NotImplementedError.
 */

#define PY_SSIZE_T_CLEAN
#include <Python.h>

/* Module state */
typedef struct {
    PyObject *error;
} _umath_linalg_state;

static int _umath_linalg_traverse(PyObject *m, visitproc visit, void *arg) {
    _umath_linalg_state *st = (_umath_linalg_state*)PyModule_GetState(m);
    Py_VISIT(st->error);
    return 0;
}

static int _umath_linalg_clear(PyObject *m) {
    _umath_linalg_state *st = (_umath_linalg_state*)PyModule_GetState(m);
    Py_CLEAR(st->error);
    return 0;
}

static struct PyModuleDef _umath_linalg_module = {
    PyModuleDef_HEAD_INIT,
    "numpy.linalg._umath_linalg",
    "Stub implementation of numpy.linalg._umath_linalg for WASI",
    sizeof(_umath_linalg_state),
    NULL,  /* methods */
    NULL,  /* slots */
    _umath_linalg_traverse,
    _umath_linalg_clear,
    NULL   /* free */
};

PyMODINIT_FUNC
PyInit__umath_linalg(void)
{
    PyObject *m;

    m = PyModule_Create(&_umath_linalg_module);
    if (m == NULL)
        return NULL;

    _umath_linalg_state *st = (_umath_linalg_state*)PyModule_GetState(m);
    st->error = PyErr_NewException("numpy.linalg._umath_linalg.LinAlgError",
                                    PyExc_Exception, NULL);
    if (st->error == NULL) {
        Py_DECREF(m);
        return NULL;
    }
    Py_INCREF(st->error);
    if (PyModule_AddObject(m, "LinAlgError", st->error) < 0) {
        Py_DECREF(st->error);
        Py_DECREF(m);
        return NULL;
    }

    return m;
}
