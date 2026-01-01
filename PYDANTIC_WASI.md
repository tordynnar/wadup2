# Pydantic on WASI

This document describes how Pydantic (including the full `BaseModel` API) was made to work on WASI (WebAssembly System Interface).

## Executive Summary

**Pydantic 2.12.5 with BaseModel works on WASI.** This required:

1. Building `pydantic_core` (Rust library) as a static library for WASI
2. Bundling all Python dependencies (pydantic, annotated_types, typing_inspection, typing_extensions)
3. Patching pydantic to handle `importlib.metadata` circular import issues in Python 3.13
4. Using increased stack size (8-64MB) for pydantic's deep import chains
5. Importing pydantic internal modules in a specific order

## Quick Start

### pyproject.toml

```toml
[project]
name = "my-project"
version = "0.1.0"

[tool.wadup]
entry-point = "my_module"
c-extensions = ["pydantic"]
```

### Python Code

```python
import wadup

def main():
    # Import pydantic modules in order to avoid deep import chains
    import pydantic._internal._config
    import pydantic._internal._fields
    import pydantic._internal._generate_schema
    import pydantic.main

    from pydantic import BaseModel, Field

    class User(BaseModel):
        name: str
        age: int = Field(ge=0, le=150)
        email: str | None = None

    # Create validated instances
    user = User(name="Alice", age=30, email="alice@example.com")

    # Use in your WADUP module
    wadup.define_table("users", [
        ("name", "String"),
        ("age", "Int64"),
        ("email", "String"),
    ])
    wadup.insert_row("users", [user.name, user.age, user.email or ""])
    wadup.flush()
```

### Running with Increased Stack

```bash
wadup --modules ./modules --input ./data --output results.db --max-stack 8388608
```

## Why the Special Import Order?

Pydantic has deep, recursive import chains that can exceed WASI stack limits:

```
pydantic → pydantic.main → pydantic._internal._decorators → pydantic._internal._core_utils → ...
```

By pre-importing the internal modules in order, we "warm up" the import cache and avoid the deep recursion:

```python
import pydantic._internal._config
import pydantic._internal._fields
import pydantic._internal._generate_schema
import pydantic.main

from pydantic import BaseModel, Field  # Now this works!
```

## WASI Patches

The build script applies three patches to handle `importlib.metadata` issues in Python 3.13 WASI:

### 1. pydantic/plugin/_loader.py

Wraps the `importlib.metadata` import in try/except:

```python
try:
    import importlib.metadata as importlib_metadata
    _HAS_IMPORTLIB_METADATA = True
except (ImportError, ModuleNotFoundError):
    _HAS_IMPORTLIB_METADATA = False
    importlib_metadata = None
```

### 2. pydantic/version.py

Wraps `importlib.metadata` usage in `version_info()` and `_ensure_pydantic_core_version()`.

### 3. pydantic/networks.py

Replaces the direct `from importlib.metadata import version` with a wrapper function that handles WASI gracefully.

**Why these patches?** Python 3.13 uses "frozen modules" for `importlib.metadata`, which causes circular import issues in WASI. The patches allow pydantic to gracefully degrade when the module isn't available.

## Building pydantic_core

The Rust-based `pydantic_core` library requires minimal modifications for WASI:

### Cargo.toml Changes

```bash
# Remove Windows-only feature
sed -i.bak 's/"generate-import-lib", //' Cargo.toml

# Change to static library (WASI doesn't support cdylib)
sed -i.bak 's/crate-type = \["cdylib", "rlib"\]/crate-type = ["staticlib", "rlib"]/' Cargo.toml
```

### PyO3 Cross-Compilation Config

```
implementation=CPython
version=3.13
shared=false
abi3=false
lib_name=python3.13
pointer_width=32
suppress_build_script_link_lines=true
```

### Build Command

```bash
export PYO3_CONFIG_FILE="$(pwd)/pyo3-wasi-config.txt"
export CARGO_TARGET_WASM32_WASIP1_LINKER="${WASI_SDK_PATH}/bin/wasm-ld"
cargo build --target wasm32-wasip1 --release
```

## Bundled Dependencies

The pydantic extension bundles these Python packages:

| Package | Version | Description |
|---------|---------|-------------|
| pydantic | 2.12.5 | High-level validation library (BaseModel, Field, etc.) |
| pydantic_core | 2.41.5 | Rust-based validation engine |
| typing_extensions | 4.15.0 | Backports of typing features |
| annotated_types | 0.7.0 | Runtime type annotations |
| typing_inspection | 0.4.2 | Type introspection utilities |

## Verified Functionality

### BaseModel Features
- Field definitions with type annotations
- Field constraints (ge, le, gt, lt, min_length, max_length, etc.)
- Optional fields with `| None`
- Default values
- Nested models
- Data validation and coercion

### pydantic_core Features
- SchemaValidator with all basic types
- SchemaSerializer for JSON output
- ValidationError with detailed messages
- Custom error types

## Alternative: Using pydantic_core Directly

If you don't need BaseModel, you can use pydantic_core directly without the import order workaround:

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

## Extension Registry

The pydantic extension in `extensions/__init__.py`:

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
        "wasi-pydantic/python/pydantic",
        "wasi-pydantic/python/annotated_types",
        "wasi-pydantic/python/typing_inspection",
    ],
    "python_files": [
        "wasi-pydantic/python/typing_extensions.py",
    ],
    "dependencies": [],
},
```

## Troubleshooting

### Stack Overflow on Import

**Symptom:** Module crashes immediately on `from pydantic import BaseModel`

**Solution:**
1. Use the import order pattern (import internal modules first)
2. Increase stack size with `--max-stack 8388608` (8MB) or higher

### "unknown variant" Error

**Symptom:** `Failed to parse metadata JSON: unknown variant 'Bool'`

**Solution:** WADUP only supports `Int64`, `Float64`, and `String` column types. Use `Int64` for booleans.

### Module Not Found

**Symptom:** `ModuleNotFoundError: No module named 'pydantic'`

**Solution:**
1. Verify `c-extensions = ["pydantic"]` in pyproject.toml
2. Check that `deps/wasi-pydantic/` exists (run `./scripts/build-pydantic-wasi.sh`)

### importlib.metadata Errors

**Symptom:** `ImportError: cannot import name '_meta' from partially initialized module 'importlib.metadata'`

**Solution:** This should be fixed by the WASI patches. If you see this, rebuild pydantic with `./scripts/build-pydantic-wasi.sh` (delete `deps/wasi-pydantic/` first to force rebuild).

## File Locations

| File | Description |
|------|-------------|
| `scripts/build-pydantic-wasi.sh` | Build script (compiles pydantic_core, bundles Python packages, applies WASI patches) |
| `deps/wasi-pydantic/lib/lib_pydantic_core.a` | Compiled static library (~39MB) |
| `deps/wasi-pydantic/python/pydantic_core/` | pydantic_core Python package |
| `deps/wasi-pydantic/python/pydantic/` | pydantic Python package (with WASI patches) |
| `deps/wasi-pydantic/python/typing_extensions.py` | typing_extensions module |
| `extensions/__init__.py` | Extension registry |
| `examples/python-pydantic-test/` | Example project using BaseModel |

## Version Information

- pydantic: 2.12.5
- pydantic_core: 2.41.5
- PyO3: 0.26
- Rust: 1.75+
- WASI SDK: 29.0+
- Python: 3.13

## Key Learnings

### 1. PyOnceLock Works on WASI

Despite WASI being single-threaded, `PyOnceLock` (which wraps `once_cell::sync::OnceCell`) works correctly. The synchronization primitives degrade gracefully.

### 2. importlib.metadata is Frozen in Python 3.13

Python 3.13 compiles `importlib.metadata` as a frozen module, making it impossible to override via user code. Libraries must gracefully handle its absence.

### 3. Import Order Matters

Deep import chains can exceed stack limits. Pre-importing modules in order avoids deep recursion.

### 4. Static Linking Required

WASI doesn't support dynamic libraries. Extensions must be compiled as `staticlib` and linked into the final WASM binary.
