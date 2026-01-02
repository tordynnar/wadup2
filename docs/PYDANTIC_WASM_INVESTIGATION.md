# Pydantic WASM Investigation

This document details the investigation into why Python modules crashed when loaded from zipfiles in WASI/WASM, and the fixes that were applied.

## Summary

**Status**: ✅ FULLY FIXED (January 2026)

Two issues were discovered and fixed:

1. **snprintf memory corruption** - Using `snprintf` before `PyRun_SimpleString` caused memory corruption
2. **Bytecode compilation crash** - Large Python files (like pydantic's `_generate_schema.py`) crashed during runtime bytecode compilation

## Fix 1: snprintf Memory Corruption

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

## Fix 2: Pre-compile Python to Bytecode

**Root Cause**: Python's bytecode compiler crashes in WASI when compiling complex files at runtime. The crash occurs in dlmalloc (the WASM memory allocator) when compiling files with many if/elif branches (like pydantic's `_generate_schema.py` with 49+ branches).

**Fix**: Pre-compile all Python files to `.pyc` bytecode during the build process, and remove the source `.py` files from the bundle. This forces Python to use the pre-compiled bytecode instead of compiling at runtime.

In `scripts/build-python-project.py`:

```python
# Pre-compile all Python files to .pyc
compileall.compile_dir(bundle_dir, force=True, quiet=1, legacy=True)

# Remove .py files to force Python to use .pyc files
# zipimport prefers .py files when both exist, so we remove .py
for py_file in list(bundle_dir.rglob('*.py')):
    pyc_file = py_file.with_suffix('.pyc')
    if pyc_file.exists():
        py_file.unlink()
```

## Technical Details

### Symptoms

When loading Python modules from a zipfile:
1. Simple modules with < 28 if/elif branches worked after the snprintf fix
2. Complex modules like pydantic's `_generate_schema.py` (49+ branches) still crashed
3. The crash was in dlmalloc during bytecode compilation, before any Python code executed

### Why Pre-compilation Works

- Python's bytecode compiler in WASI has issues with complex control flow
- The crash occurs in dlmalloc, suggesting memory allocation issues during compilation
- Pre-compiled `.pyc` files bypass the runtime compiler entirely
- The host Python (used during build) handles the compilation correctly

### Key Insight

The crash happened before any Python code executed - even debug print statements at the top of `_generate_schema.py` never ran. This proved the issue was in the bytecode compilation phase, not in the code itself.

## Testing

All 11 integration tests now pass:

```bash
./scripts/run-integration-tests.sh

# Output:
# ⏱️  test_python_pydantic passed in 1.83s
# ...
# Tests passed: 11
# Tests failed: 0
```

The pydantic test now uses full `pydantic.BaseModel`:

```python
from pydantic import BaseModel

class User(BaseModel):
    name: str
    age: int
    email: str

user = User(name="Alice", age=30, email="alice@example.com")
```

## Date

- Investigation conducted: January 2026
- snprintf fix applied: January 2, 2026
- Pre-compilation fix applied: January 2, 2026
