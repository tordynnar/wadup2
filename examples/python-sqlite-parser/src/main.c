#define PY_SSIZE_T_CLEAN
#include <Python.h>

// Forward declaration of wadup module init function
extern PyObject* PyInit_wadup(void);

// Embedded Python script will be included here
// Generated from script.py during build
const char embedded_python_script[] =
#include "script.py.h"
;

// WASM export attribute for process() function
#define WASM_EXPORT __attribute__((visibility("default"))) \
                    __attribute__((export_name("process")))

// Main entry point called by WADUP
WASM_EXPORT
int process(void) {
    int result = 0;

    // Register wadup module BEFORE Py_Initialize
    // This is critical - must happen before interpreter starts
    if (PyImport_AppendInittab("wadup", PyInit_wadup) == -1) {
        return 1;
    }

    // Pre-configure Python to use UTF-8 mode to avoid needing encodings module
    PyPreConfig preconfig;
    PyPreConfig_InitIsolatedConfig(&preconfig);
    preconfig.utf8_mode = 1;  // Enable UTF-8 mode

    PyStatus status = Py_PreInitialize(&preconfig);
    if (PyStatus_Exception(status)) {
        return 1;
    }

    // Use simple initialization
    Py_Initialize();

    // Check if initialization succeeded
    if (!Py_IsInitialized()) {
        return 1;
    }

    // Execute embedded Python script
    if (PyRun_SimpleString(embedded_python_script) != 0) {
        // Print Python error if available
        PyErr_Print();
        result = 1;
    }

    // Clean up Python interpreter
    if (Py_FinalizeEx() < 0) {
        result = 1;
    }

    return result;
}
