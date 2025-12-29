# Python SQLite Parser

A self-contained Python-based SQLite parser WASM module for WADUP that parses SQLite databases and extracts metadata.

## Overview

This example demonstrates how to create a fully self-contained WASM module containing:
- **CPython 3.13.7** interpreter compiled for wasm32-wasip1
- **SQLite 3.45.1** library compiled for WASI
- **Python standard library** (frozen into the binary)
- **Custom C extension** exposing WADUP host functions to Python
- **SQLite parsing script** embedded in the binary

The final WASM module is ~26MB and contains everything needed to run Python code that parses SQLite databases.

## Architecture

```
┌─────────────────────────────────────────┐
│   python_sqlite_parser.wasm (26MB)     │
├─────────────────────────────────────────┤
│  main.c                                 │
│  └─ Initializes Python interpreter     │
│  └─ Registers wadup C extension        │
│  └─ Executes embedded script.py        │
├─────────────────────────────────────────┤
│  wadup_module.c (Python C Extension)   │
│  └─ define_table() → WASM import       │
│  └─ insert_row() → WASM import          │
├─────────────────────────────────────────┤
│  CPython 3.13.7 (libpython3.13.a)      │
│  └─ _sqlite3 built-in C extension      │
│  └─ Frozen stdlib modules              │
│     - encodings, warnings, functools   │
│     - collections, datetime, sqlite3   │
│     - re, traceback, linecache, etc.   │
├─────────────────────────────────────────┤
│  SQLite 3.45.1 (libsqlite3.a)          │
│  └─ Compiled with WASI, no extensions  │
├─────────────────────────────────────────┤
│  Embedded Python Script                │
│  └─ Parses SQLite, calls wadup API     │
└─────────────────────────────────────────┘
```

## Requirements

### System Requirements
- **macOS** (tested on macOS with Apple Silicon)
- **Python 3.13** (for regenerating frozen modules)
- **WASI SDK 24.0** (automatically downloaded by build script)
- **Standard build tools**: make, tar, unzip, wget/curl

### Build Dependencies
The build script (`build-python.sh`) automatically handles:
- Downloading Python 3.13.7 source
- Downloading SQLite 3.45.1 amalgamation
- Downloading WASI SDK 24.0
- Building everything from source

## Building

### Quick Build
```bash
./build.sh
```

This runs the entire build process and produces `target/python_sqlite_parser.wasm`.

### Step-by-Step Build

#### 1. Build Python for WASI
```bash
./build-python.sh
```

This script:
1. Downloads and extracts Python 3.13.7 source
2. Downloads and compiles SQLite 3.45.1 for WASI
3. Modifies `freeze_modules.py` to freeze essential stdlib modules
4. Builds native Python (for cross-compilation tools)
5. Builds Python for wasm32-wasip1 target with:
   - Frozen encodings modules
   - Frozen stdlib (warnings, functools, collections, datetime, sqlite3, etc.)
   - _sqlite3 C extension built-in
   - Static linking of all libraries

Output: `python-wasi/lib/libpython3.13.a` (~42MB)

#### 2. Build WASM Module
```bash
make clean && make
```

This:
1. Embeds `script.py` into C header file
2. Compiles C sources (main.c, wadup_module.c)
3. Links with Python, SQLite, and support libraries
4. Produces final WASM module

Output: `target/python_sqlite_parser.wasm` (~26MB)

## File Structure

```
examples/python-sqlite-parser/
├── README.md                 # This file
├── build.sh                  # Main build script
├── build-python.sh           # CPython build automation
├── Makefile                  # WASM module build configuration
├── embed_script.sh           # Embeds Python script as C string
├── src/
│   ├── main.c               # Entry point, Python initialization
│   ├── wadup_module.c       # WADUP C extension for Python
│   └── script.py            # SQLite parsing logic
├── target/
│   └── python_sqlite_parser.wasm  # Final WASM module
└── python-wasi/             # CPython build output (gitignored)
    ├── lib/
    │   ├── libpython3.13.a  # Python library (~42MB)
    │   ├── libsqlite3.a     # SQLite library
    │   ├── libmpdec.a       # Decimal module library
    │   ├── libexpat.a       # XML parser library
    │   └── libHacl_*.a      # Cryptography libraries
    └── include/             # Python headers
```

## Key Implementation Details

### 1. Python Initialization (main.c)

The `process()` function:
1. Registers the `wadup` C extension module before initializing Python
2. Pre-configures Python to use UTF-8 mode (required for WASI)
3. Initializes the Python interpreter
4. Executes the embedded Python script
5. Properly cleans up and finalizes Python

```c
if (PyImport_AppendInittab("wadup", PyInit_wadup) == -1) {
    return 1;
}

PyPreConfig preconfig;
PyPreConfig_InitIsolatedConfig(&preconfig);
preconfig.utf8_mode = 1;
Py_PreInitialize(&preconfig);

Py_Initialize();
PyRun_SimpleString(embedded_python_script);
Py_FinalizeEx();
```

### 2. WADUP Python Extension (wadup_module.c)

A Python C extension that exposes WADUP host functions:

```python
import wadup

# Define a metadata table
wadup.define_table("table_name", [
    ("column1", "String"),
    ("column2", "Int64")
])

# Insert rows
wadup.insert_row("table_name", ["value1", 42])
```

Internally, these call WASM imports:
```c
__attribute__((import_module("env")))
__attribute__((import_name("define_table")))
extern int32_t wadup_define_table(
    const uint8_t* name_ptr, size_t name_len,
    const uint8_t* columns_ptr, size_t columns_len
);
```

### 3. Frozen Standard Library

The most critical challenge was making Python's standard library available without a filesystem. Solution: **frozen modules**.

Python's `Tools/build/freeze_modules.py` was modified to include:
- `encodings.*` - Required for Python initialization
- `warnings` - Required by sqlite3
- `functools`, `types`, `linecache`, `traceback` - Required by warnings
- `collections.*` - Required by various stdlib modules
- `datetime` - Required by sqlite3
- `operator`, `keyword`, `heapq`, `reprlib`, `weakref` - Dependencies
- `enum`, `copy` - More dependencies
- `re.*`, `sre_compile`, `sre_parse`, `sre_constants` - Regex support
- `contextlib` - Context manager support
- `sqlite3.*` - SQLite Python wrapper

These modules are compiled into C code and embedded in `libpython3.13.a`.

### 4. POSIX Function Stubs (Provided by WADUP Runtime)

WASI doesn't support many POSIX functions that Python expects. WADUP provides these as host imports, so no C stubs are needed in the guest module:

- Signal handling: `signal()`, `raise()`, `__SIG_DFL`, `__SIG_IGN`, `__SIG_ERR`
- Process info: `getpid()`
- Timing: `clock()`, `times()`
- Dynamic linking: `dlopen()`, `dlsym()`, `dlclose()`, `dlerror()`
- Signal info: `strsignal()`

These functions are stub implementations that either no-op or return safe default values.

### 5. SQLite for WASI

SQLite is compiled with:
- `-DSQLITE_OMIT_LOAD_EXTENSION=1` - No dynamic extension loading
- `-DSQLITE_THREADSAFE=0` - Single-threaded (WASI limitation)
- `-DSQLITE_ENABLE_FTS5=1` - Full-text search
- `-DSQLITE_ENABLE_JSON1=1` - JSON support

## Usage Example

```bash
# Create a test SQLite database
sqlite3 /tmp/test.db << EOF
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob');
CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT);
INSERT INTO posts VALUES (1, 'Post 1'), (2, 'Post 2');
EOF

# Process with WADUP
wadup --modules examples/python-sqlite-parser/target \
      --input /tmp \
      --output /tmp/results.db

# View results
sqlite3 /tmp/results.db "SELECT * FROM db_table_stats"
```

Output:
```
<content-id>|users|2
<content-id>|posts|2
```

## Common Issues and Solutions

### Issue: "No module named 'encodings'"
**Solution**: The encodings module must be frozen into the Python binary. The build script handles this by uncommenting `'<encodings.*>'` in `freeze_modules.py`.

### Issue: "No module named 'warnings'" (or functools, datetime, etc.)
**Solution**: These are dependencies of sqlite3. The build script freezes a comprehensive set of stdlib modules.

### Issue: "unknown import: env::signal"
**Solution**: WASI doesn't support signal handling. WADUP provides stub implementations as host imports. Make sure you're using a recent version of WADUP that includes the POSIX stub functions.

### Issue: Build fails with "No rule to make target 'Lib/re.py'"
**Solution**: `re` is a package directory, not a single file. Use `'<re.*>'` pattern instead of `'re'` in the frozen modules list.

### Issue: Module compiles but Python crashes on initialization
**Solution**: Check that all required libraries are linked in the correct order in the Makefile:
```makefile
-lpython3.13 \
libmpdec.a \
libexpat.a \
libsqlite3.a \
libHacl_Hash_SHA2.a \
-lm
```

## Performance Characteristics

- **Startup time**: ~20ms (Python initialization)
- **Memory usage**: ~30-40MB (Python interpreter + script)
- **Binary size**: 26MB (highly compressed, contains full Python)
- **SQLite performance**: Native speed (compiled C code)

## Integration Test

Run the integration test:
```bash
cargo test --release test_python_sqlite_parser
```

This verifies:
1. Module builds successfully
2. Python initializes correctly
3. SQLite3 imports and works
4. WADUP host functions are callable
5. Output matches expected format

## Comparison with Rust Implementation

| Aspect | Python Implementation | Rust Implementation |
|--------|----------------------|---------------------|
| Binary Size | ~26MB | ~400KB |
| Build Time | ~5-10 minutes | ~30 seconds |
| Dependencies | CPython + SQLite | rusqlite crate |
| Flexibility | Full Python stdlib | Rust ecosystem |
| Startup Time | ~20ms | ~1ms |
| Use Case | Complex logic, rapid prototyping | Production, performance-critical |

The Python implementation trades binary size and build complexity for the ability to write parsing logic in Python with full stdlib access.

## Technical Notes

### Why CPython 3.13.7?
- Improved WASI support compared to earlier versions
- Better cross-compilation tooling
- Stable frozen modules infrastructure

### Why Freeze Stdlib Instead of Using WASI Filesystem?
- WADUP modules receive data via `/data.bin` virtual file
- No general filesystem access for Python imports
- Frozen modules are compiled into the binary and always available
- Significantly faster import times

### Memory Management
- Initial memory: 128MB (configurable via `--initial-memory`)
- Maximum memory: 256MB (configurable via `--max-memory`)
- Python's garbage collector handles Python objects
- WASM linear memory managed by wasmtime runtime

### Security Considerations
- No filesystem access (except `/data.bin`)
- No network access
- No signal handling or process spawning
- Sandboxed execution in WASM runtime
- All system calls go through WASI interface

## Future Improvements

Potential enhancements:
1. **Reduce binary size**: Strip debug symbols, optimize Python build flags
2. **Faster builds**: Cache CPython build artifacts
3. **More stdlib modules**: Add additional frozen modules as needed
4. **Better error reporting**: Capture and format Python exceptions
5. **Performance tuning**: Optimize Python interpreter flags for WASM

## References

- [CPython WASI Support](https://github.com/python/cpython/blob/main/Tools/wasm/README.md)
- [WASI SDK](https://github.com/WebAssembly/wasi-sdk)
- [Python Frozen Modules](https://docs.python.org/3/library/sys.html#sys._stdlib_dir)
- [SQLite Compilation Options](https://www.sqlite.org/compile.html)

## License

Same as parent project.
