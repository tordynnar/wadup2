/**
 * Test pydantic_core import on WASI.
 *
 * This links against the real pydantic_core library to see if it crashes.
 */

#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <stdio.h>
#include <stdlib.h>

// Declaration of pydantic_core's init function
extern PyObject* PyInit__pydantic_core(void);

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;

    fprintf(stderr, "[C] ============================================\n");
    fprintf(stderr, "[C] Testing pydantic_core import on WASI\n");
    fprintf(stderr, "[C] ============================================\n\n");

    // Register the extension module BEFORE Python initialization
    // Note: Register as just "_pydantic_core" for direct import
    fprintf(stderr, "[C] Registering _pydantic_core extension...\n");
    if (PyImport_AppendInittab("_pydantic_core", PyInit__pydantic_core) == -1) {
        fprintf(stderr, "[C] ERROR: Failed to register _pydantic_core\n");
        return 1;
    }
    fprintf(stderr, "[C] Extension registered successfully\n\n");

    // Pre-configure Python for UTF-8 mode
    fprintf(stderr, "[C] Pre-initializing Python...\n");
    PyPreConfig preconfig;
    PyPreConfig_InitIsolatedConfig(&preconfig);
    preconfig.utf8_mode = 1;

    PyStatus status = Py_PreInitialize(&preconfig);
    if (PyStatus_Exception(status)) {
        fprintf(stderr, "[C] ERROR: Python pre-initialization failed\n");
        return 1;
    }
    fprintf(stderr, "[C] Python pre-initialized\n\n");

    // Initialize Python interpreter
    fprintf(stderr, "[C] Initializing Python interpreter...\n");
    Py_Initialize();

    if (!Py_IsInitialized()) {
        fprintf(stderr, "[C] ERROR: Python initialization failed\n");
        return 1;
    }
    fprintf(stderr, "[C] Python interpreter initialized\n\n");

    // Test: Import pydantic_core._pydantic_core
    fprintf(stderr, "[C] ============================================\n");
    fprintf(stderr, "[C] Importing pydantic_core._pydantic_core\n");
    fprintf(stderr, "[C] ============================================\n\n");

    const char* test_code =
        "print('[Python] Importing _pydantic_core...')\n"
        "import _pydantic_core as pc\n"
        "print('[Python] Import succeeded!')\n"
        "print(f'[Python] pydantic_core version: {pc.__version__}')\n"
        "print('[Python] Testing PydanticUndefined...')\n"
        "print(f'[Python] PydanticUndefined = {pc.PydanticUndefined}')\n"
        "print()\n"
        "print('[Python] Testing SchemaValidator...')\n"
        "schema = {'type': 'str'}\n"
        "validator = pc.SchemaValidator(schema)\n"
        "print(f'[Python] Created validator: {validator}')\n"
        "result = validator.validate_python('hello')\n"
        "print(f'[Python] Validated \"hello\" -> {result}')\n"
        "print()\n"
        "print('[Python] Testing SchemaSerializer...')\n"
        "serializer = pc.SchemaSerializer(schema)\n"
        "print(f'[Python] Created serializer: {serializer}')\n"
        "result = serializer.to_python('world')\n"
        "print(f'[Python] Serialized \"world\" -> {result}')\n"
        "print()\n"
        "print('[Python] ALL TESTS PASSED!')\n";

    fprintf(stderr, "[C] Running Python code...\n\n");
    if (PyRun_SimpleString(test_code) != 0) {
        fprintf(stderr, "\n[C] ERROR: Test failed\n");
        PyErr_Print();
        Py_Finalize();
        return 1;
    }

    fprintf(stderr, "\n[C] ============================================\n");
    fprintf(stderr, "[C] SUCCESS - pydantic_core works on WASI!\n");
    fprintf(stderr, "[C] ============================================\n");

    Py_Finalize();
    return 0;
}
