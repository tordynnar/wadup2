# PyOnceLock WASI Investigation

This is a test case investigating the behavior of PyO3's `PyOnceLock` and `std::sync::OnceLock` on WASI (wasm32-wasip1).

## Executive Summary

**BOTH PyOnceLock AND pydantic_core WORK ON WASI!**

```
[Python] Importing _pydantic_core...
[Python] Import succeeded!
[Python] pydantic_core version: 2.41.5
[Python] PydanticUndefined = PydanticUndefined
[Python] ALL TESTS PASSED!
```

The original hypothesis that PyOnceLock causes crashes on WASI is **incorrect** with current tooling:
- PyO3 0.26
- once_cell 1.21.3
- wasmtime 37.0.0
- WASI SDK 29.0
- Rust wasm32-wasip1 target

## Background

During earlier attempts to build pydantic_core for WASI, crashes were observed during module initialization. The hypothesis was that `PyOnceLock` uses threading primitives that don't exist on WASI.

## Test Results

### Minimal PyOnceLock Tests

**All tests PASS on current wasmtime (37.0.0) with PyO3 0.26:**

```
TEST 1: simple_add() - PASSED (no OnceLock)
TEST 2: std::sync::OnceLock - PASSED
TEST 3: pyo3::sync::PyOnceLock - PASSED
```

### Tested Patterns (All Pass!)

1. **std::sync::OnceLock** - Works
2. **pyo3::sync::PyOnceLock** - Works
3. **PyOnceLock during module init** (like pydantic_core) - Works
4. **PyOnceLock with Py::new()** (creates pyclass instance) - Works
5. **PyOnceLock that imports Python modules** (like `fractions.Fraction`) - Works
6. **Multiple chained PyOnceLock calls** during module init - Works

### Full pydantic_core Test

**pydantic_core 2.41.5 with FULL VALIDATION works on WASI!**

```
[Python] Testing SchemaValidator...
[Python] Created validator: SchemaValidator(title="str", validator=Str(...))
[Python] Validated "hello" -> hello

[Python] Testing SchemaSerializer...
[Python] Created serializer: SchemaSerializer(serializer=Str(...))
[Python] Serialized "world" -> world

[Python] ALL TESTS PASSED!
```

- Build: 39MB static library
- Module import: Works
- PydanticUndefined: Works
- SchemaValidator: Works
- SchemaSerializer: Works
- String validation/serialization: Works

### Conclusions

The original concerns about PyOnceLock on WASI are no longer valid. Either:
1. Recent tooling updates fixed the issues
2. The crashes were caused by something else entirely
3. The build/link configuration was incorrect in earlier attempts

## The Original Hypothesis

`PyOnceLock` (introduced in PyO3 0.26) uses `once_cell::sync::OnceCell` internally. The dependency chain was thought to be:

```
PyOnceLock (pyo3::sync)
    └── once_cell::sync::OnceCell
        └── std::sync::Once
            └── pthread primitives
                └── NOT AVAILABLE ON WASI ❌
```

### Why Emscripten Works

Pyodide uses `wasm32-unknown-emscripten` which provides pthread emulation stubs. Even in single-threaded mode, the stubs exist and return successfully.

## Files

```
pyo3_issue/
├── README.md           # This file
├── Cargo.toml          # Rust project configuration
├── src/
│   └── lib.rs          # Minimal PyO3 extension with PyOnceLock
├── main.c              # C host program embedding Python
├── build.sh            # Build script
├── run.sh              # Run script
├── deps/               # (created by build) WASI SDK and Python
└── build/              # (created by build) Build artifacts
```

## Prerequisites

1. **Rust** with wasm32-wasip1 target:
   ```bash
   rustup target add wasm32-wasip1
   ```

2. **wasmtime** runtime:
   ```bash
   curl https://wasmtime.dev/install.sh -sSf | bash
   ```

3. **Python WASI build** (from parent project):
   ```bash
   # Run from parent directory first if not already built
   cd ..
   ./scripts/build-python-wasi.sh
   ```

## Building

```bash
./build.sh
```

This will:
1. Download WASI SDK 29.0 (if not present)
2. Copy Python WASI build from parent project
3. Build the Rust extension for wasm32-wasip1
4. Compile the C host program
5. Link everything into `build/pyoncelock_demo.wasm`

## Running

```bash
./run.sh
```

### Expected Output

All tests pass on current wasmtime versions:

```
[C] TEST 1: Import module and call simple_add()
[C] This does NOT use any OnceLock and should work
...
[Python] TEST 1 PASSED

[C] TEST 2: Call test_std_oncelock()
[C] This uses std::sync::OnceLock
...
[Python] TEST 2 PASSED - std::sync::OnceLock worked!

[C] TEST 3: Call test_py_oncelock()
[C] This uses pyo3::sync::PyOnceLock
...
[Python] TEST 3 PASSED - pyo3::sync::PyOnceLock worked!

[C] ALL TESTS PASSED
```

## Investigation Notes

### Current Status (Jan 2026)

**EVERYTHING WORKS!** Both PyOnceLock and full pydantic_core work on WASI.

Tested successfully:
- `std::sync::OnceLock`
- `pyo3::sync::PyOnceLock`
- Creating pyclass instances inside PyOnceLock
- Importing Python modules inside PyOnceLock
- Multiple chained PyOnceLock initializations during module init
- **Full pydantic_core 2.41.5 import and usage**

### How to Build pydantic_core for WASI

1. Get the source:
   ```bash
   cd deps
   tar xf pydantic_core-2.41.5.tar.gz
   cd pydantic-core-2.41.5
   ```

2. Patch Cargo.toml:
   - Remove `generate-import-lib` feature from pyo3
   - Change `crate-type` from `cdylib` to `staticlib`
   - Add `[workspace]` section

3. Build:
   ```bash
   PYO3_CONFIG_FILE=/path/to/pyo3-wasi-config.txt \
   CARGO_TARGET_WASM32_WASIP1_LINKER=/path/to/wasm-ld \
   cargo build --target wasm32-wasip1 --release
   ```

4. Link with Python and run via wasmtime

## The Code

### Rust Extension (src/lib.rs)

Tests multiple patterns that pydantic_core uses:

```rust
use std::sync::OnceLock;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyType;

// Test std::sync::OnceLock (like pydantic_core's version string)
static STD_ONCELOCK: OnceLock<String> = OnceLock::new();

// Test pyo3::sync::PyOnceLock caching a type
static PY_ONCELOCK: PyOnceLock<Py<PyType>> = PyOnceLock::new();

// PyOnceLock with pyclass - like pydantic_core's PydanticUndefinedType
static UNDEFINED_CELL: PyOnceLock<Py<UndefinedType>> = PyOnceLock::new();

// PyOnceLock that imports Python modules - like pydantic_core's FRACTION_TYPE
static FRACTION_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

#[pyclass]
pub struct UndefinedType {}

#[pymethods]
impl UndefinedType {
    #[staticmethod]
    pub fn get(py: Python<'_>) -> &Py<Self> {
        UNDEFINED_CELL.get_or_init(py, || Py::new(py, UndefinedType {}).unwrap())
    }
}

#[pymodule]
fn pyoncelock_demo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Initialize PyOnceLock during module init (exactly like pydantic_core)
    m.add("Undefined", UndefinedType::get(m.py()))?;
    Ok(())
}
```

### C Host (main.c)

Minimal Python embedding that registers the extension and runs test code.

## Status of Potential Fixes

**NO FIXES NEEDED!** Both PyOnceLock and pydantic_core work correctly on WASI with:
- PyO3 0.26
- once_cell 1.21.3
- wasmtime 37.0.0
- WASI SDK 29.0

The `once_cell::sync::OnceCell` implementation works correctly on wasm32-wasip1 without any special handling.

## Key Findings

1. **PyOnceLock works on WASI** - No patches needed to PyO3 or once_cell
2. **pydantic_core 2.41.5 with full validation works on WASI** - Builds as 39MB static library
3. **SchemaValidator and SchemaSerializer work** - Core validation/serialization is functional
4. **The original crash hypothesis was incorrect** - Current tooling works perfectly
5. **Pydantic V2 on WASI is now achievable** - Just needs proper build/link integration

## Files in This Directory

- `src/lib.rs` - Minimal PyOnceLock test extension
- `main.c` - Test harness for minimal extension
- `test_pydantic_core.c` - Test harness for full pydantic_core
- `build.sh` - Build minimal test
- `build-pydantic-test.sh` - Build pydantic_core test
- `run.sh` - Run minimal test

## License

This demonstration code is released into the public domain.
