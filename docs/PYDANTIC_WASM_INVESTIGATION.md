# Pydantic WASM Investigation

This document details the investigation into why the full `pydantic` library (specifically `BaseModel`) crashes when used in WASI/WASM, while `pydantic_core` works correctly.

## Summary

**Status**: The full pydantic library cannot be used in WASI due to a crash in the `pydantic_core` C extension during module initialization.

**Workaround**: Use `pydantic_core` directly instead of the high-level `pydantic` API.

## What Works

| Import | Status | Notes |
|--------|--------|-------|
| `import pydantic` | ✅ Works | Uses lazy loading, doesn't trigger full import |
| `import pydantic_core` | ✅ Works | Direct C extension import |
| `from pydantic_core import SchemaValidator, core_schema` | ✅ Works | Full validation functionality |
| `from pydantic import BaseModel` | ❌ Crashes | Triggers full import chain |
| `from pydantic import main` | ❌ Crashes | Same as above |

## Technical Details

### Crash Location

The crash occurs when importing `pydantic.main`, which is triggered by accessing `BaseModel` from the `pydantic` module. The import chain is:

```
from pydantic import BaseModel
  → pydantic.__getattr__('BaseModel')
  → import pydantic.main
  → import pydantic._internal._model_construction
  → import pydantic._internal._generate_schema
  → [CRASH in pydantic_core]
```

### WASM Backtrace

```
0: 0xa2c0f3 - <unknown>!<wasm function 15532>
1: 0xa2d91f - <unknown>!<wasm function 15535>
2: 0x17cf3 - <unknown>!<wasm function 106>
...
```

Function 15532 is in `pydantic_core` (high function numbers indicate compiled C code). The crash is a WASM trap, not a Python exception.

### Versions Tested

- pydantic: 2.12.5
- pydantic_core: 2.41.5
- Python: 3.13 (WASI build)

## Investigation Steps

### 1. Initial Testing

Confirmed that `import pydantic` works but `from pydantic import BaseModel` crashes.

### 2. Import Chain Analysis

Traced through pydantic's lazy import mechanism:
- `pydantic/__init__.py` uses `__getattr__` for lazy loading
- Accessing `BaseModel` triggers import of `pydantic.main`
- `pydantic.main` has many module-level imports

### 3. Dependency Isolation

Tested importing each dependency of `pydantic.main` individually:

| Module | Status |
|--------|--------|
| `pydantic_core` | ✅ Works |
| `typing_extensions` | ✅ Works |
| `typing_inspection` | ✅ Works |
| `pydantic.errors` | ✅ Works |
| `pydantic.warnings` | ✅ Works |
| `pydantic._internal._config` | ✅ Works |
| `pydantic._internal._decorators` | ✅ Works |
| `pydantic._internal._fields` | ✅ Works |
| `pydantic._internal._model_construction` | ❌ Crashes |

### 4. Further Isolation

The crash in `_model_construction` is caused by its import of `_generate_schema`:

```python
# pydantic/_internal/_model_construction.py line 25
from ._generate_schema import GenerateSchema, InvalidSchemaError
```

### 5. Root Cause

The crash appears to be in `pydantic_core` itself during schema generation initialization. This is likely caused by:

1. **Memory/stack issues** - Schema generation may require more stack than WASM allows
2. **Missing WASI syscalls** - The C code may call unsupported system functions
3. **pydantic_core WASM build issue** - The WASI build of pydantic_core may have bugs

## Recommendations

### Short Term (Current Solution)

Use `pydantic_core` directly for validation:

```python
from pydantic_core import SchemaValidator, core_schema

# Define schema using pydantic_core directly
user_schema = core_schema.typed_dict_schema({
    'name': core_schema.typed_dict_field(core_schema.str_schema()),
    'age': core_schema.typed_dict_field(core_schema.int_schema(ge=0)),
})

validator = SchemaValidator(user_schema)
validated = validator.validate_python({'name': 'Alice', 'age': 30})
```

### Long Term Options

1. **Investigate pydantic_core WASM build** - Debug the C extension compilation
2. **Try older pydantic versions** - Earlier versions may work better
3. **Increase WASM stack/memory limits** - May help if it's a resource issue
4. **Report to pydantic team** - This may be a known WASI compatibility issue

## Files Changed

- `examples/python-pydantic-test/` - Test module for investigation
- Current test uses `pydantic_core` directly (working solution)

## Related Issues

- pydantic uses `importlib.metadata` which can be slow/problematic in WASI
- The crash is in compiled C code, not Python, making debugging difficult
- WASM function 15532 is in pydantic_core but without debug symbols we can't identify the exact location

## Date

Investigation conducted: January 2026
