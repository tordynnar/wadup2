# Python WASM Runtime

This directory contains the shared runtime components for building Python WASM modules for WADUP.

## Contents

### Source Files

- **`src/main.c`** - Python interpreter initialization and execution framework
  - Initializes CPython interpreter for WASM environment
  - Registers the `wadup` extension module
  - Executes embedded Python scripts via `PyRun_SimpleString`
  - Maintains interpreter state across multiple `process()` calls

- **`src/signal_stubs.c`** - POSIX signal and dynamic loading stubs for WASI
  - Provides no-op implementations of signal functions (signal, raise, etc.)
  - Provides stub implementations of dynamic loading functions (dlopen, dlsym, etc.)
  - Required because WASI does not support POSIX signals or dynamic loading
  - Allows Python runtime to compile without modifications

- **`src/wadup_module.c`** - The `wadup` Python extension module
  - Provides `wadup.define_table(name, columns)` - Define output table schema
  - Provides `wadup.insert_row(table_name, values)` - Insert data rows
  - Converts Python data structures to JSON for host communication
  - Implements WASM imports from the `env` module for host interaction

## Usage

These source files are referenced by `scripts/build-python-module.sh` when building Python WASM modules. They are **not meant to be modified per-module** - they provide the common runtime that all Python modules share.

To create a new Python WASM module:

1. Create a directory in `examples/` (e.g., `examples/python-mymodule/`)
2. Create `src/script.py` with your Python code
3. Create a Makefile that delegates to `scripts/build-python-module.sh`
4. Run `make` to build

See existing Python examples in `examples/python-*/` for templates.

## Build Process

The build script:

1. Copies these C sources to a temporary build directory
2. Embeds your Python script into a C header file
3. Compiles all C sources with WASI SDK
4. Links with CPython WASI libraries
5. Produces a standalone WASM module

## Customization

If you need to customize the runtime behavior:

- **Add Python APIs**: Modify `src/wadup_module.c` to add new functions to the `wadup` module
- **Change initialization**: Modify `src/main.c` to adjust Python interpreter setup
- **Add WASI stubs**: Modify `src/signal_stubs.c` to add additional stub functions

Changes here will affect **all** Python WASM modules.

## Dependencies

- CPython 3.13+ compiled for WASI (see `scripts/build-python-wasi.sh`)
- WASI SDK 24.0 or later
- Required Python libraries: libpython3.13.a, libmpdec.a, libexpat.a, libsqlite3.a, etc.
