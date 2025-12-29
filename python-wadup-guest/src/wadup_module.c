#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <string.h>
#include <stdio.h>

// In-memory accumulation of metadata
static PyObject* tables_list = NULL;
static PyObject* rows_list = NULL;
static int flush_counter = 0;

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

// Initialize the in-memory lists if needed
static int ensure_lists_initialized(void) {
    if (tables_list == NULL) {
        tables_list = PyList_New(0);
        if (!tables_list) return -1;
    }
    if (rows_list == NULL) {
        rows_list = PyList_New(0);
        if (!rows_list) return -1;
    }
    return 0;
}

// Python function: wadup.define_table(name, columns)
// columns: list of (name, type) tuples
// Accumulates table definition in memory
static PyObject* py_define_table(PyObject* self, PyObject* args) {
    const char* table_name;
    PyObject* columns_list;

    if (!PyArg_ParseTuple(args, "sO", &table_name, &columns_list)) {
        return NULL;
    }

    if (ensure_lists_initialized() < 0) {
        return NULL;
    }

    // Build columns JSON
    char* columns_json = build_columns_json(columns_list);
    if (!columns_json) {
        return NULL;
    }

    // Create table definition dict
    PyObject* table_dict = PyDict_New();
    if (!table_dict) {
        free(columns_json);
        return NULL;
    }

    PyObject* name_str = PyUnicode_FromString(table_name);
    PyObject* cols_str = PyUnicode_FromString(columns_json);
    free(columns_json);

    if (!name_str || !cols_str) {
        Py_XDECREF(name_str);
        Py_XDECREF(cols_str);
        Py_DECREF(table_dict);
        return NULL;
    }

    PyDict_SetItemString(table_dict, "name", name_str);
    PyDict_SetItemString(table_dict, "columns_json", cols_str);
    Py_DECREF(name_str);
    Py_DECREF(cols_str);

    PyList_Append(tables_list, table_dict);
    Py_DECREF(table_dict);

    Py_RETURN_NONE;
}

// Python function: wadup.insert_row(table_name, values)
// values: list of values (int/float/string)
// Accumulates row in memory
static PyObject* py_insert_row(PyObject* self, PyObject* args) {
    const char* table_name;
    PyObject* values_list;

    if (!PyArg_ParseTuple(args, "sO", &table_name, &values_list)) {
        return NULL;
    }

    if (ensure_lists_initialized() < 0) {
        return NULL;
    }

    // Build values JSON
    char* values_json = build_values_json(values_list);
    if (!values_json) {
        return NULL;
    }

    // Create row definition dict
    PyObject* row_dict = PyDict_New();
    if (!row_dict) {
        free(values_json);
        return NULL;
    }

    PyObject* name_str = PyUnicode_FromString(table_name);
    PyObject* vals_str = PyUnicode_FromString(values_json);
    free(values_json);

    if (!name_str || !vals_str) {
        Py_XDECREF(name_str);
        Py_XDECREF(vals_str);
        Py_DECREF(row_dict);
        return NULL;
    }

    PyDict_SetItemString(row_dict, "table_name", name_str);
    PyDict_SetItemString(row_dict, "values_json", vals_str);
    Py_DECREF(name_str);
    Py_DECREF(vals_str);

    PyList_Append(rows_list, row_dict);
    Py_DECREF(row_dict);

    Py_RETURN_NONE;
}

// Python function: wadup.flush()
// Writes accumulated metadata to /metadata/output_N.json
static PyObject* py_flush(PyObject* self, PyObject* args) {
    if (ensure_lists_initialized() < 0) {
        return NULL;
    }

    Py_ssize_t num_tables = PyList_Size(tables_list);
    Py_ssize_t num_rows = PyList_Size(rows_list);

    // Nothing to flush
    if (num_tables == 0 && num_rows == 0) {
        Py_RETURN_NONE;
    }

    // Build filename
    char filename[64];
    snprintf(filename, sizeof(filename), "/metadata/output_%d.json", flush_counter++);

    // Build JSON manually
    // Estimate size: tables + rows
    size_t buf_size = 1024 + num_tables * 256 + num_rows * 512;
    char* json = (char*)malloc(buf_size);
    if (!json) {
        PyErr_NoMemory();
        return NULL;
    }

    char* ptr = json;
    ptr += sprintf(ptr, "{\"tables\":[");

    // Write tables
    for (Py_ssize_t i = 0; i < num_tables; i++) {
        PyObject* table_dict = PyList_GetItem(tables_list, i);
        PyObject* name_obj = PyDict_GetItemString(table_dict, "name");
        PyObject* cols_obj = PyDict_GetItemString(table_dict, "columns_json");

        const char* name = PyUnicode_AsUTF8(name_obj);
        const char* cols = PyUnicode_AsUTF8(cols_obj);

        if (i > 0) ptr += sprintf(ptr, ",");
        ptr += sprintf(ptr, "{\"name\":\"%s\",\"columns\":%s}", name, cols);
    }

    ptr += sprintf(ptr, "],\"rows\":[");

    // Write rows
    for (Py_ssize_t i = 0; i < num_rows; i++) {
        PyObject* row_dict = PyList_GetItem(rows_list, i);
        PyObject* name_obj = PyDict_GetItemString(row_dict, "table_name");
        PyObject* vals_obj = PyDict_GetItemString(row_dict, "values_json");

        const char* name = PyUnicode_AsUTF8(name_obj);
        const char* vals = PyUnicode_AsUTF8(vals_obj);

        if (i > 0) ptr += sprintf(ptr, ",");
        ptr += sprintf(ptr, "{\"table_name\":\"%s\",\"values\":%s}", name, vals);
    }

    ptr += sprintf(ptr, "]}");

    // Write to file
    FILE* file = fopen(filename, "w");
    if (!file) {
        free(json);
        PyErr_Format(PyExc_IOError, "Failed to create metadata file '%s'", filename);
        return NULL;
    }

    size_t json_len = ptr - json;
    if (fwrite(json, 1, json_len, file) != json_len) {
        fclose(file);
        free(json);
        PyErr_Format(PyExc_IOError, "Failed to write metadata file '%s'", filename);
        return NULL;
    }

    fclose(file);
    free(json);

    // Clear the lists
    PyList_SetSlice(tables_list, 0, num_tables, NULL);
    PyList_SetSlice(rows_list, 0, num_rows, NULL);

    Py_RETURN_NONE;
}

// Module method definitions
static PyMethodDef WadupMethods[] = {
    {"define_table", py_define_table, METH_VARARGS,
     "Define a metadata table. Usage: define_table(name, [(col_name, col_type), ...])"},
    {"insert_row", py_insert_row, METH_VARARGS,
     "Insert a row into a table. Usage: insert_row(table_name, [val1, val2, ...])"},
    {"flush", py_flush, METH_NOARGS,
     "Flush accumulated metadata to file. WADUP auto-flushes on process() return."},
    {NULL, NULL, 0, NULL}
};

// Module definition
static struct PyModuleDef wadupmodule = {
    PyModuleDef_HEAD_INIT,
    "wadup",
    "WADUP file-based metadata API for Python WASM modules",
    -1,
    WadupMethods
};

// Module initialization function
PyMODINIT_FUNC PyInit_wadup(void) {
    return PyModule_Create(&wadupmodule);
}
