# Building pydantic_core for WASI

This document describes how `pydantic_core` (the Rust-based validation engine for Pydantic v2) was made to work on WASI (WebAssembly System Interface).

## Executive Summary

**pydantic_core 2.41.5 works on WASI with minimal modifications.** The only changes required are:
1. Change crate-type from `cdylib` to `staticlib` (WASI doesn't support dynamic linking)
2. Remove the `generate-import-lib` PyO3 feature (Windows-only)
3. Provide a PyO3 cross-compilation config file

No source code patches are needed. PyO3's `PyOnceLock` and `once_cell::sync::OnceCell` work correctly on WASI.

## Background Investigation

### Initial Hypothesis (Incorrect)

The original attempt to integrate pydantic_core failed, leading to the hypothesis that `PyOnceLock` (used extensively in pydantic_core for lazy static initialization) might not work on WASI due to threading limitations.

### Testing PyOnceLock

A minimal test case was created in `pyo3_issue/` to verify PyOnceLock behavior:

```rust
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use std::sync::OnceLock;

static RUST_ONCE: OnceLock<i32> = OnceLock::new();
static PY_ONCE: GILOnceCell<PyObject> = GILOnceCell::new();

#[pyfunction]
fn test_once_lock(py: Python<'_>) -> PyResult<(i32, String)> {
    let rust_val = RUST_ONCE.get_or_init(|| 42);
    let py_val = PY_ONCE.get_or_init(py, || {
        PyString::new(py, "initialized").into()
    });
    Ok((*rust_val, py_val.extract::<String>(py)?))
}
```

**Result: Both work correctly on WASI.** The test passes, and multiple calls return the same initialized values.

### Root Cause of Original Failure

The original pydantic_core build attempt failed because:
1. Incomplete patches were applied to source files
2. The patches tried to replace `PyOnceLock` with `thread_local!`, but were incomplete
3. This left the code in a broken state

**Solution:** Don't patch the source. Build pydantic_core as-is with only Cargo.toml modifications.

## Build Process

### Prerequisites

- WASI SDK 24.0+ installed at `/opt/wasi-sdk`
- Rust with `wasm32-wasip1` target: `rustup target add wasm32-wasip1`
- Python 3.13 WASI build (from `build/python-wasi/`)

### Build Script

The build script is at `scripts/build-pydantic-wasi.sh`. Key steps:

#### 1. Download Source

```bash
pip download --no-binary :all: pydantic-core==2.41.5 -d .
tar xf pydantic_core-2.41.5.tar.gz
```

#### 2. Patch Cargo.toml

Only two changes are needed:

```bash
# Remove Windows-only feature
sed -i.bak 's/"generate-import-lib", //' Cargo.toml

# Change to static library (WASI doesn't support cdylib)
sed -i.bak 's/crate-type = \["cdylib", "rlib"\]/crate-type = ["staticlib", "rlib"]/' Cargo.toml
```

#### 3. Create PyO3 Cross-Compilation Config

PyO3 needs to know the target Python configuration:

```bash
cat > pyo3-wasi-config.txt << 'EOF'
implementation=CPython
version=3.13
shared=false
abi3=false
lib_name=python3.13
pointer_width=32
suppress_build_script_link_lines=true
EOF

export PYO3_CONFIG_FILE="$(pwd)/pyo3-wasi-config.txt"
```

#### 4. Build for WASI

```bash
export WASI_SDK_PATH=/opt/wasi-sdk
export CC="${WASI_SDK_PATH}/bin/clang"
export AR="${WASI_SDK_PATH}/bin/ar"
export CARGO_TARGET_WASM32_WASIP1_LINKER="${CC}"

cargo build --target wasm32-wasip1 --release
```

#### 5. Output

The build produces:
- `target/wasm32-wasip1/release/lib_pydantic_core.a` (~39MB static library)

This is copied to `deps/wasi-pydantic/lib/lib_pydantic_core.a` along with the Python package files.

## Integration into WADUP

### Extension Registry

Add to `extensions/__init__.py`:

```python
"pydantic": {
    "modules": [
        ("_pydantic_core", "PyInit__pydantic_core"),
    ],
    "libraries": [
        "wasi-pydantic/lib/lib_pydantic_core.a",
    ],
    "python_dirs": [
        "wasi-pydantic/python/pydantic_core",
    ],
    "dependencies": [],
    "validation": [
        "wasi-pydantic/lib/lib_pydantic_core.a",
    ],
},
```

### Using in Projects

In your `pyproject.toml`:

```toml
[project]
name = "my-project"
version = "0.1.0"

[tool.wadup]
entry-point = "my_module"
c-extensions = ["pydantic"]
```

In your Python code:

```python
from pydantic_core import SchemaValidator, SchemaSerializer, ValidationError

# Create a validator
validator = SchemaValidator({"type": "str"})

# Validate data
try:
    result = validator.validate_python("hello")
    print(f"Valid: {result}")
except ValidationError as e:
    print(f"Invalid: {e}")

# Serialize to JSON
serializer = SchemaSerializer({"type": "int"})
json_bytes = serializer.to_json(42)  # b'42'
```

## Verified Functionality

The following pydantic_core features have been tested and work on WASI:

### SchemaValidator
- String validation
- Integer validation (with coercion and strict mode)
- Float validation (with string coercion)
- Boolean validation
- List validation with item schemas
- Dict validation with key/value schemas
- Validation error messages with detailed information

### SchemaSerializer
- JSON serialization of all basic types
- Proper encoding to bytes

## Why the Full Pydantic Library Doesn't Work

**Important:** Only `pydantic_core` works on WASI. The full `pydantic` library (with `BaseModel`, `Field`, etc.) does not work due to the following limitations:

### 1. Stack Overflow During Import

The pydantic library has deep, complex import chains:
```
pydantic → pydantic.main → pydantic._internal._decorators → pydantic._internal._core_utils → ...
```

These nested imports exceed WASI's default stack limits, causing a stack overflow during module initialization.

### 2. typing_extensions Dependency

Both `pydantic` and `pydantic_core`'s `core_schema.py` module depend on `typing_extensions`:
```python
from typing_extensions import TypeVar, deprecated, Sentinel
```

The `typing_extensions` package is not bundled by default, and bundling it adds complexity and size.

### 3. Complex Type System

Pydantic's `BaseModel` relies heavily on Python's type system, metaclasses, and runtime type introspection. These features work but add significant overhead and import complexity.

### Workaround: Use pydantic_core Directly

Instead of:
```python
from pydantic import BaseModel, Field

class Person(BaseModel):
    name: str
    age: int = Field(ge=0)
```

Use pydantic_core's schema dictionary format:
```python
from pydantic_core import SchemaValidator, ValidationError

person_schema = {
    "type": "typed-dict",
    "fields": {
        "name": {"type": "typed-dict-field", "schema": {"type": "str"}},
        "age": {"type": "typed-dict-field", "schema": {"type": "int", "ge": 0}},
    },
}
validator = SchemaValidator(person_schema)

try:
    result = validator.validate_python({"name": "Alice", "age": 30})
except ValidationError as e:
    print(f"Invalid: {e}")
```

This approach:
- Avoids the complex import chain
- Doesn't require typing_extensions
- Works reliably on WASI
- Provides the same validation capabilities

### Schema Reference

For schema dictionary syntax, see the [pydantic_core documentation](https://docs.pydantic.dev/latest/concepts/json_schema/) or the `core_schema.py` source file.

## Key Learnings

### 1. PyOnceLock Works on WASI

Despite WASI being single-threaded, `PyOnceLock` (which wraps `once_cell::sync::OnceCell`) works correctly. The synchronization primitives degrade gracefully on single-threaded targets.

### 2. No Source Patches Needed

The pydantic_core Rust source code requires no modifications. Only the Cargo.toml needs two simple changes for WASI compatibility.

### 3. Static Linking is Required

WASI doesn't support dynamic library loading (`dlopen`). All extensions must be:
- Compiled as static libraries (`staticlib`)
- Registered with `PyImport_AppendInittab` before `Py_Initialize()`
- Linked into the final WASM binary

### 4. PyO3 Cross-Compilation

When cross-compiling PyO3 for WASI:
- Set `PYO3_CONFIG_FILE` to a config file with target Python details
- Use `pointer_width=32` (WASM is 32-bit)
- Set `suppress_build_script_link_lines=true` to prevent native linking attempts

### 5. Module Registration Name

Register the module as `_pydantic_core` (not `pydantic_core._pydantic_core`). The Python package structure handles the import path.

## Troubleshooting

### "unknown variant `Bool`" Error

If you see: `Failed to parse metadata JSON: unknown variant 'Bool'`

This means your Python code is using `Bool` as a column type in `wadup.define_table()`. The WADUP host only supports:
- `Int64`
- `Float64`
- `String`

Use `Int64` for boolean values (0 = False, 1 = True).

### Module Not Found

If `import pydantic_core` fails:
1. Verify the extension is listed in `c-extensions` in pyproject.toml
2. Check that `deps/wasi-pydantic/lib/lib_pydantic_core.a` exists
3. Ensure the Python package is at `deps/wasi-pydantic/python/pydantic_core/`

### Build Fails with Linker Errors

Ensure WASI SDK environment is set:
```bash
export WASI_SDK_PATH=/opt/wasi-sdk
export CC="${WASI_SDK_PATH}/bin/clang"
export AR="${WASI_SDK_PATH}/bin/ar"
```

## File Locations

| File | Description |
|------|-------------|
| `scripts/build-pydantic-wasi.sh` | Build script for pydantic_core |
| `deps/wasi-pydantic/lib/lib_pydantic_core.a` | Compiled static library |
| `deps/wasi-pydantic/python/pydantic_core/` | Python package files |
| `extensions/__init__.py` | Extension registry (includes pydantic) |
| `examples/python-pydantic-test/` | Example project using pydantic_core |
| `pyo3_issue/` | Minimal test case for PyOnceLock investigation |

## Version Information

- pydantic_core: 2.41.5
- PyO3: 0.26
- Rust: 1.75+
- WASI SDK: 24.0+
- Python: 3.13
