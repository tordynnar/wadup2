# PyOnceLock WASI Investigation

This is a test case investigating the behavior of PyO3's `PyOnceLock` and `std::sync::OnceLock` on WASI (wasm32-wasip1).

## Background

During attempts to build pydantic_core for WASI, crashes were observed during module initialization. The hypothesis was that `PyOnceLock` uses threading primitives that don't exist on WASI.

## Test Results

**Surprisingly, this minimal test case PASSES on current wasmtime (37.0.0):**

```
TEST 1: simple_add() - PASSED (no OnceLock)
TEST 2: std::sync::OnceLock - PASSED
TEST 3: pyo3::sync::PyOnceLock - PASSED
```

This suggests that either:
1. Recent wasmtime versions have improved WASI threading primitive support
2. The pydantic_core crash is caused by something more complex than just PyOnceLock
3. The Rust toolchain's wasm32-wasip1 target now handles OnceLock differently

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

The pydantic_core crash may be caused by:
1. A specific interaction between multiple PyOnceLock usages
2. The version of PyO3 used by pydantic_core (may differ from our test)
3. Complex module initialization ordering
4. Specific once_cell features or configurations

To properly reproduce the pydantic_core crash, run the full pydantic build:
```bash
cd ..  # parent project
./scripts/build-pydantic-wasi.sh
./scripts/build-python-project.py examples/python-pydantic-test
```

## The Code

### Rust Extension (src/lib.rs)

```rust
use std::sync::OnceLock;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyType;

// Test std::sync::OnceLock
static STD_ONCELOCK: OnceLock<String> = OnceLock::new();

// Test pyo3::sync::PyOnceLock
static PY_ONCELOCK: PyOnceLock<Py<PyType>> = PyOnceLock::new();

#[pyfunction]
fn test_std_oncelock() -> String {
    STD_ONCELOCK.get_or_init(|| "initialized".to_string()).clone()
}

#[pyfunction]
fn test_py_oncelock(py: Python<'_>) -> PyResult<Py<PyType>> {
    let type_obj = PY_ONCELOCK.get_or_init(py, || {
        py.get_type::<pyo3::types::PyInt>().unbind()
    });
    Ok(type_obj.clone_ref(py))
}
```

### C Host (main.c)

Minimal Python embedding that registers the extension and runs test code.

## Potential Fixes

1. **PyO3 level**: Use `once_cell::unsync::OnceCell` on wasm32 targets
2. **PyO3 level**: Use `once_cell::race` module (non-blocking, may call init multiple times)
3. **PyO3 level**: Enable `critical-section` feature with WASI-compatible implementation

Example fix in PyO3:
```rust
// In pyo3/src/sync/once_lock.rs
#[cfg(not(target_arch = "wasm32"))]
pub struct PyOnceLock<T> {
    inner: once_cell::sync::OnceCell<T>,
}

#[cfg(target_arch = "wasm32")]
pub struct PyOnceLock<T> {
    inner: once_cell::unsync::OnceCell<T>,
}
```

## Related Issues

- This affects any PyO3-based extension using `PyOnceLock`, including:
  - pydantic-core (15+ usages)
  - Any extension caching Python types

- PyO3 already has wasm32 conditional code for tests but not for `PyOnceLock` itself.

## License

This demonstration code is released into the public domain.
