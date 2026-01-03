# Pydantic on WASI

This document describes how Pydantic works on WASI (WebAssembly System Interface), including the investigation and fixes for crashes that were discovered.

## Executive Summary

**Pydantic 2.12.5 with BaseModel works on WASI.** This required:

1. Building `pydantic_core` (Rust library) as a static library for WASI
2. Bundling all Python dependencies (pydantic, annotated_types, typing_inspection, typing_extensions)
3. Patching pydantic to handle `importlib.metadata` circular import issues in Python 3.13
4. Adding `--stack-first` linker flag to prevent stack overflow into heap
5. Pre-compiling Python to bytecode (optional optimization)

### Fixes Applied (January 2026)

| Issue | Root Cause | Fix |
|-------|-----------|-----|
| snprintf memory corruption | Using `snprintf` before `PyRun_SimpleString` | Use compile-time string concatenation |
| Bytecode compilation crash | Missing `--stack-first` linker flag | Added `-Wl,--stack-first` and `-z stack-size=8388608` |

---

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
from pydantic import BaseModel, Field

class User(BaseModel):
    name: str
    age: int = Field(ge=0, le=150)
    email: str | None = None

def main():
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

---

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

---

## Alternative: Using pydantic_core Directly

If you don't need BaseModel, you can use pydantic_core directly:

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

---

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

---

## WASI Patches

The build script applies patches to handle `importlib.metadata` issues in Python 3.13 WASI:

### pydantic/plugin/_loader.py

Wraps the `importlib.metadata` import in try/except:

```python
try:
    import importlib.metadata as importlib_metadata
    _HAS_IMPORTLIB_METADATA = True
except (ImportError, ModuleNotFoundError):
    _HAS_IMPORTLIB_METADATA = False
    importlib_metadata = None
```

### pydantic/version.py

Wraps `importlib.metadata` usage in `version_info()` and `_ensure_pydantic_core_version()`.

### pydantic/networks.py

Replaces the direct `from importlib.metadata import version` with a wrapper function that handles WASI gracefully.

**Why these patches?** Python 3.13 uses "frozen modules" for `importlib.metadata`, which causes circular import issues in WASI.

---

## Technical Fixes

### Fix 1: snprintf Memory Corruption

**Root Cause**: Using `snprintf` before calling `PyRun_SimpleString` caused memory corruption when Python imported modules from a zipfile.

**Fix**: Replace runtime `snprintf` string formatting with compile-time C preprocessor string concatenation.

In `guest/python/src/main_bundled_template.c`, changed from:

```c
// BROKEN - causes memory corruption
char cmd[512];
snprintf(cmd, sizeof(cmd),
    "import %s as _m; _m.main() if hasattr(_m, 'main') else None",
    ENTRY_MODULE);
PyRun_SimpleString(cmd);
```

To:

```c
// FIXED - uses preprocessor string concatenation
#define IMPORT_CMD "import " ENTRY_MODULE " as _m; _m.main() if hasattr(_m, 'main') else None"
PyRun_SimpleString(IMPORT_CMD);
```

### Fix 2: Stack-First Linker Flag (THE ROOT CAUSE)

**Root Cause**: The `--stack-first` linker flag was missing. Without this flag, WASM linear memory places the stack after data sections. When Python's bytecode compiler uses significant stack space, the stack overflows into the heap, corrupting pointers.

**Symptoms**:
- Memory fault at impossible addresses (e.g., `0xa1d68e82` in 128 MB linear memory)
- Crashes in `label_exception_targets` → `_PyCfg_OptimizeCodeUnit` → `compiler_function`
- Only occurs during bytecode compilation of complex code

**Fix**: Added `--stack-first` and `-z stack-size=8388608` (8 MB stack) to linker flags in `scripts/build-python-project.py`:

```python
ldflags = [
    "-Wl,--allow-undefined",
    "-Wl,--export=process",
    "-Wl,--initial-memory=134217728",  # 128 MB
    "-Wl,--max-memory=268435456",      # 256 MB
    "-Wl,--no-entry",
    "-z", "stack-size=8388608",        # 8 MB stack (same as official CPython WASI)
    "-Wl,--stack-first",               # Critical: place stack before data to prevent corruption
]
```

**Memory layout difference**:

```
Without --stack-first:
[Data Sections][Heap→ ←Stack]  # Stack can overflow into heap!

With --stack-first:
[Stack][Data Sections][Heap→]  # Stack is isolated at start of memory
```

### Fix 3: Pre-compile Python to Bytecode (OPTIONAL)

Pre-compilation provides faster startup by avoiding runtime compilation:

```python
# Pre-compile all Python files to .pyc
compileall.compile_dir(bundle_dir, force=True, quiet=1, legacy=True)

# Remove .py files to force Python to use .pyc files
for py_file in list(bundle_dir.rglob('*.py')):
    pyc_file = py_file.with_suffix('.pyc')
    if pyc_file.exists():
        py_file.unlink()
```

To disable for debugging: `WADUP_SKIP_PRECOMPILE=1 ./scripts/build-python-project.py <project>`

---

## Troubleshooting

### Stack Overflow on Import

**Symptom:** Module crashes immediately on `from pydantic import BaseModel`

**Solution:** Increase stack size with `--max-stack 8388608` (8MB) or higher

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

**Solution:** Rebuild pydantic with `./scripts/build-pydantic-wasi.sh` (delete `deps/wasi-pydantic/` first to force rebuild).

---

## File Locations

| File | Description |
|------|-------------|
| `scripts/build-pydantic-wasi.sh` | Build script (compiles pydantic_core, bundles Python packages, applies WASI patches) |
| `deps/wasi-pydantic/lib/lib_pydantic_core.a` | Compiled static library (~39MB) |
| `deps/wasi-pydantic/python/pydantic_core/` | pydantic_core Python package |
| `deps/wasi-pydantic/python/pydantic/` | pydantic Python package (with WASI patches) |
| `deps/wasi-pydantic/python/typing_extensions.py` | typing_extensions module |
| `guest/python/src/main_bundled_template.c` | C template with snprintf fix |
| `scripts/build-python-project.py` | Build script with --stack-first fix |
| `examples/python-pydantic-test/` | Example project using BaseModel |

---

## Investigation Summary

A comprehensive investigation was conducted to understand why Python's bytecode compiler crashed in WADUP but not in the official Python WASI build.

### What Was Tested

| Approach | Result |
|----------|--------|
| Merging 46K data segments into 1 | ❌ Still crashed |
| Increasing memory to 512 MB, 1 GB | ❌ Still crashed |
| Linker optimizations (-O2, --gc-sections) | ❌ Still crashed |
| Reducing frozen modules | ❌ Still crashed |
| Adding `--stack-first` linker flag | ✅ **Fixed!** |

### Root Cause Discovery

1. Built minimal Python WASI from scratch
2. Compared linker output with official `python3 Tools/wasm/wasi.py build`
3. Found official uses `-z stack-size=8388608 -Wl,--stack-first`
4. Added to WADUP → Fixed!

### Key Insight

The faulting addresses (e.g., `0xa1d68e82` in 128 MB memory) were garbage values caused by stack overflow corrupting heap metadata. The `--stack-first` flag isolates the stack at the beginning of memory where it can't corrupt the heap.

---

## Key Learnings

### 1. --stack-first is Critical for WASI

Without `--stack-first`, deep call stacks (like Python's bytecode compiler) can overflow into the heap, causing mysterious pointer corruption.

### 2. PyOnceLock Works on WASI

Despite WASI being single-threaded, `PyOnceLock` (which wraps `once_cell::sync::OnceCell`) works correctly. The synchronization primitives degrade gracefully.

### 3. importlib.metadata is Frozen in Python 3.13

Python 3.13 compiles `importlib.metadata` as a frozen module, making it impossible to override via user code. Libraries must gracefully handle its absence.

### 4. Static Linking Required

WASI doesn't support dynamic libraries. Extensions must be compiled as `staticlib` and linked into the final WASM binary.

---

## Version Information

- pydantic: 2.12.5
- pydantic_core: 2.41.5
- PyO3: 0.26
- Rust: 1.75+
- WASI SDK: 24.0
- Python: 3.13

---

## Timeline

| Date | Event |
|------|-------|
| January 2, 2026 | snprintf memory corruption fix applied |
| January 2, 2026 | Pre-compilation workaround applied |
| January 2, 2026 | Deep root cause investigation conducted |
| January 3, 2026 | **ROOT CAUSE FOUND: Missing `--stack-first` linker flag** |
| January 3, 2026 | Applied fix to `scripts/build-python-project.py` |
| January 3, 2026 | All 11 integration tests passing |
