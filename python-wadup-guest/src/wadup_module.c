#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <string.h>
#include <stdio.h>

// Import WADUP host functions
// These are provided by the WASM runtime (imported from "env" module)
__attribute__((import_module("env")))
__attribute__((import_name("define_table")))
extern int32_t wadup_define_table(
    const uint8_t* name_ptr, size_t name_len,
    const uint8_t* columns_ptr, size_t columns_len
);

__attribute__((import_module("env")))
__attribute__((import_name("insert_row")))
extern int32_t wadup_insert_row(
    const uint8_t* table_ptr, size_t table_len,
    const uint8_t* row_ptr, size_t row_len
);

// Helper: Build JSON string for columns
// Input: Python list of tuples [(name, type), ...]
// Output: JSON string like [{"name":"table_name","data_type":"String"},...]
static char* build_columns_json(PyObject* columns_list) {
    if (!PyList_Check(columns_list)) {
        PyErr_SetString(PyExc_TypeError, "columns must be a list");
        return NULL;
    }

    Py_ssize_t num_cols = PyList_Size(columns_list);

    // Allocate buffer (rough estimate: 100 bytes per column)
    size_t buf_size = num_cols * 100 + 100;
    char* json = (char*)malloc(buf_size);
    if (!json) {
        PyErr_NoMemory();
        return NULL;
    }

    char* ptr = json;
    ptr += sprintf(ptr, "[");

    for (Py_ssize_t i = 0; i < num_cols; i++) {
        PyObject* col_tuple = PyList_GetItem(columns_list, i);
        if (!PyTuple_Check(col_tuple) || PyTuple_Size(col_tuple) != 2) {
            free(json);
            PyErr_SetString(PyExc_TypeError, "each column must be a 2-tuple (name, type)");
            return NULL;
        }

        PyObject* name_obj = PyTuple_GetItem(col_tuple, 0);
        PyObject* type_obj = PyTuple_GetItem(col_tuple, 1);

        const char* name = PyUnicode_AsUTF8(name_obj);
        const char* type_str = PyUnicode_AsUTF8(type_obj);

        if (!name || !type_str) {
            free(json);
            return NULL;
        }

        if (i > 0) {
            ptr += sprintf(ptr, ",");
        }
        ptr += sprintf(ptr, "{\"name\":\"%s\",\"data_type\":\"%s\"}", name, type_str);
    }

    ptr += sprintf(ptr, "]");
    return json;
}

// Helper: Build JSON string for row values
// Input: Python list of values
// Output: JSON string like [{"Int64":42},{"String":"foo"},...]
static char* build_values_json(PyObject* values_list) {
    if (!PyList_Check(values_list)) {
        PyErr_SetString(PyExc_TypeError, "values must be a list");
        return NULL;
    }

    Py_ssize_t num_vals = PyList_Size(values_list);

    // Allocate buffer
    size_t buf_size = num_vals * 200 + 100;
    char* json = (char*)malloc(buf_size);
    if (!json) {
        PyErr_NoMemory();
        return NULL;
    }

    char* ptr = json;
    ptr += sprintf(ptr, "[");

    for (Py_ssize_t i = 0; i < num_vals; i++) {
        PyObject* val = PyList_GetItem(values_list, i);

        if (i > 0) {
            ptr += sprintf(ptr, ",");
        }

        if (PyLong_Check(val)) {
            long long int_val = PyLong_AsLongLong(val);
            ptr += sprintf(ptr, "{\"Int64\":%lld}", int_val);
        } else if (PyFloat_Check(val)) {
            double float_val = PyFloat_AsDouble(val);
            ptr += sprintf(ptr, "{\"Float64\":%f}", float_val);
        } else if (PyUnicode_Check(val)) {
            const char* str_val = PyUnicode_AsUTF8(val);
            if (!str_val) {
                free(json);
                return NULL;
            }
            // Escape quotes in string
            ptr += sprintf(ptr, "{\"String\":\"");
            const char* s = str_val;
            while (*s) {
                if (*s == '"' || *s == '\\') {
                    *ptr++ = '\\';
                }
                *ptr++ = *s++;
            }
            ptr += sprintf(ptr, "\"}");
        } else {
            free(json);
            PyErr_SetString(PyExc_TypeError, "value must be int, float, or string");
            return NULL;
        }
    }

    ptr += sprintf(ptr, "]");
    return json;
}

// Python function: wadup.define_table(name, columns)
// columns: list of (name, type) tuples
static PyObject* py_define_table(PyObject* self, PyObject* args) {
    const char* table_name;
    PyObject* columns_list;

    if (!PyArg_ParseTuple(args, "sO", &table_name, &columns_list)) {
        return NULL;
    }

    // Build columns JSON
    char* columns_json = build_columns_json(columns_list);
    if (!columns_json) {
        return NULL;
    }

    // Call host function
    int32_t result = wadup_define_table(
        (const uint8_t*)table_name, strlen(table_name),
        (const uint8_t*)columns_json, strlen(columns_json)
    );

    free(columns_json);

    if (result < 0) {
        PyErr_Format(PyExc_RuntimeError, "Failed to define table '%s'", table_name);
        return NULL;
    }

    Py_RETURN_NONE;
}

// Python function: wadup.insert_row(table_name, values)
// values: list of values (int/float/string)
static PyObject* py_insert_row(PyObject* self, PyObject* args) {
    const char* table_name;
    PyObject* values_list;

    if (!PyArg_ParseTuple(args, "sO", &table_name, &values_list)) {
        return NULL;
    }

    // Build values JSON
    char* values_json = build_values_json(values_list);
    if (!values_json) {
        return NULL;
    }

    // Call host function
    int32_t result = wadup_insert_row(
        (const uint8_t*)table_name, strlen(table_name),
        (const uint8_t*)values_json, strlen(values_json)
    );

    free(values_json);

    if (result < 0) {
        PyErr_Format(PyExc_RuntimeError, "Failed to insert row into table '%s'", table_name);
        return NULL;
    }

    Py_RETURN_NONE;
}

// Module method definitions
static PyMethodDef WadupMethods[] = {
    {"define_table", py_define_table, METH_VARARGS,
     "Define a metadata table. Usage: define_table(name, [(col_name, col_type), ...])"},
    {"insert_row", py_insert_row, METH_VARARGS,
     "Insert a row into a table. Usage: insert_row(table_name, [val1, val2, ...])"},
    {NULL, NULL, 0, NULL}
};

// Module definition
static struct PyModuleDef wadupmodule = {
    PyModuleDef_HEAD_INIT,
    "wadup",
    "WADUP host function bindings for Python WASM modules",
    -1,
    WadupMethods
};

// Module initialization function
PyMODINIT_FUNC PyInit_wadup(void) {
    return PyModule_Create(&wadupmodule);
}
