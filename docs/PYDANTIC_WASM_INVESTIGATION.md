# Pydantic WASM Investigation

This document details the investigation into why the full `pydantic` library (specifically `BaseModel`) crashes when used in WASI/WASM, while `pydantic_core` works correctly.

## Summary

**Status**: The full pydantic library cannot be used in WASI due to a crash during Python bytecode compilation of the large `_generate_schema.py` file (2884 lines).

**Root Cause**: The crash occurs in the WASM memory allocator (dlmalloc, function 15532) during Python's bytecode compilation phase. This appears to be triggered by the complexity/size of `pydantic/_internal/_generate_schema.py`.

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

The crash occurs when Python attempts to compile/load `pydantic/_internal/_generate_schema.py`. The import chain is:

```
from pydantic import BaseModel
  → pydantic.__getattr__('BaseModel')
  → import pydantic.main
  → import pydantic._internal._model_construction
  → import pydantic._internal._generate_schema
  → [CRASH in dlmalloc during bytecode compilation]
```

### WASM Backtrace

```
0: 0xa2c0f3 - <unknown>!<wasm function 15532>
1: 0xa2d91f - <unknown>!<wasm function 15535>
2: 0x17cf3 - <unknown>!<wasm function 106>
...
```

### Crash Details

Function 15532 is **dlmalloc** (the WASM memory allocator). The crash is at:

```
a2c0f1: 20 01                      |  local.get 1
a2c0f3: 28 02 04                   |  i32.load 2 4  <-- CRASH HERE
```

This is in a malloc freelist traversal loop. The crash happens when:
1. Python compiles the 2884-line `_generate_schema.py`
2. Compilation requires memory allocation
3. The allocator attempts to traverse its freelist
4. It encounters a corrupted or invalid pointer

### What Was Ruled Out

| Hypothesis | Status | Details |
|------------|--------|---------|
| Stack size too small | ❌ Ruled out | Tested with 100MB stack (`--max-stack 104857600`) |
| Memory limit too small | ❌ Ruled out | Tested with 1GB memory (`--max-memory 1073741824`) |
| pydantic_core import issue | ❌ Ruled out | All pydantic_core imports work fine |
| Import chain issue | ❌ Ruled out | All imports work, crash is in file body |

### Key Finding: File Complexity

Through progressive testing with a simplified version of `_generate_schema.py`:

| Test Version | Lines | Status |
|--------------|-------|--------|
| Minimal (just class stubs) | ~20 | ✅ Works |
| With stdlib imports | ~50 | ✅ Works |
| With pydantic_core imports | ~80 | ✅ Works |
| With all imports + constants | ~214 | ✅ Works |
| Full original file | 2884 | ❌ Crashes |

**Conclusion**: The crash is triggered by Python's bytecode compilation of the full file body (functions and class definitions), not by the imports or constants.

### Versions Tested

- pydantic: 2.12.5
- pydantic_core: 2.41.5
- Python: 3.13 (WASI build)
- wasmtime: via wadup

## Investigation Steps

### 1. Initial Testing
Confirmed that `import pydantic` works but `from pydantic import BaseModel` crashes.

### 2. Stack/Memory Testing
Tested with dramatically increased stack (100MB) and memory (1GB) limits. Crash persisted, ruling out simple resource limits.

### 3. Import Chain Analysis
Added debug print statements to trace the import chain:
- All pydantic_core items import successfully
- All pydantic internal modules import successfully
- Crash occurs specifically on `_generate_schema` import

### 4. Module Content Isolation
Created progressively larger versions of `_generate_schema.py`:
- Imports only: Works
- Imports + constants: Works
- Full file: Crashes

### 5. WASM Function Analysis
Analyzed function 15532 using `wasm-objdump`:
- Identified as dlmalloc memory allocator
- Crash at offset 0xa2c0f3 (memory load in freelist traversal)
- Indicates memory corruption during large allocation

## Root Cause Theory

The crash appears to be caused by memory corruption during Python's bytecode compilation of large/complex source files. When Python compiles `_generate_schema.py`:

1. Python's compiler parses the 2884-line file into an AST
2. The AST is compiled into bytecode
3. This process requires many memory allocations
4. At some point, the allocator's internal state becomes corrupted
5. A subsequent allocation crashes when traversing a corrupted freelist

This could be caused by:
- A bug in the WASI Python build's memory handling
- An edge case in dlmalloc when handling many allocations
- Insufficient memory initialization in WASM startup

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

1. **Report to CPython/WASI team** - This appears to be a Python WASI build issue with large file compilation
2. **Split _generate_schema.py** - Breaking up the large file might work around the issue
3. **Pre-compile bytecode** - Ship .pyc files instead of .py to avoid runtime compilation
4. **Investigate allocator configuration** - dlmalloc may have configurable parameters that affect this behavior

## Files Changed

- `examples/python-pydantic-test/` - Test module for investigation
- `crates/wadup-core/src/wasm.rs` - Added stderr logging on errors
- Current test uses `pydantic_core` directly (working solution)

## Related Issues

- The crash is in compiled C code (dlmalloc), not pydantic_core
- WASM function 15532 is the memory allocator, not pydantic-specific code
- The issue may affect other large Python files, not just pydantic

## Date

Investigation conducted: January 2026
