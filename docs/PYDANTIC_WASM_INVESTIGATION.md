# Pydantic WASM Investigation

This document details the investigation into why the full `pydantic` library (specifically `BaseModel`) crashes when used in WASI/WASM, while `pydantic_core` works correctly.

## Summary

**Status**: The full pydantic library cannot be used in WASI due to a crash during Python bytecode compilation.

**Root Cause**: The crash occurs in the WASM memory allocator (dlmalloc, function 15532) during Python's bytecode compilation phase. **The specific trigger is methods with 28 or more if/elif branches.** The `match_type` method in `_generate_schema.py` has over 100 branches, which triggers the crash.

**Workaround**: Use `pydantic_core` directly instead of the high-level `pydantic` API.

### Key Findings

| Test | Result |
|------|--------|
| 100MB stack size (`--max-stack 104857600`) | ❌ Still crashes |
| 1GB memory limit (`--max-memory 1073741824`) | ❌ Still crashes |
| All pydantic_core imports | ✅ Work fine |
| All pydantic internal module imports | ✅ Work fine |
| `_generate_schema.py` imports only (~80 lines) | ✅ Works |
| `_generate_schema.py` imports + constants (~214 lines) | ✅ Works |
| Full `_generate_schema.py` (2884 lines) | ❌ Crashes |
| **Method with 27 if/elif branches** | ✅ Works |
| **Method with 28 if/elif branches** | ❌ Crashes |

### Minimal Reproduction with wasmtime CLI

The crash can be reproduced with wasmtime directly:

```bash
# Run the wadup-built module
wasmtime run \
  --dir=/tmp/wasm-app::/app \
  --invoke process \
  examples/python-large-file-test/target/python_large_file_test.wasm
```

The crash occurs when the module tries to import `large_module.py` which contains a method with 28 if/elif branches.

### Key Finding: Crash is wadup-specific

**Important:** The same Python code runs successfully with standalone Python WASM:

```bash
# This WORKS - same code, standalone Python
wasmtime run \
  --dir=. \
  --dir=/tmp \
  --env PYTHONPATH=/lib/python3.13 \
  official/python-3.13.0-wasi_sdk-24/python.wasm -- /tmp/large_module.py
```

The crash only occurs in wadup-built modules, which include:
- pydantic_core C extension
- Custom zipimport-based module loading
- C main.c entry point with process() function

### Crashing Code Pattern

```python
class CrashTrigger:
    def method_with_many_branches(self, obj: Any) -> str:
        if obj == 0:
            return "case_0"
        elif obj == 1:
            return "case_1"
        # ... 25 more elif branches ...
        elif obj == 27:
            return "case_27"
        return "default"
```

**The crash occurs at exactly 28 if/elif branches in wadup modules.** 27 branches works fine.

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

### 6. Binary Search for Crash Location
Binary search through `_generate_schema.py` to find exact crash point:
- Lines 1-1046: Works
- Lines 1-1167: Crashes
- The `match_type` method (lines 1047-1166) triggers the crash

### 7. Minimal Reproduction
Created minimal test case without pydantic:
- Generated Python files with if/elif chains
- Binary search for exact branch count
- Found: 27 branches works, 28 branches crashes
- Confirmed with 76-line minimal reproduction file

## Root Cause: wadup-specific Build Issue

The crash is specific to wadup-built Python WASM modules. **The same Python code works fine with standalone Python WASM.**

**Specific trigger in wadup modules**: Methods with **28 or more if/elif branches** crash during bytecode compilation.

The crash occurs in dlmalloc when:
1. Python's compiler generates bytecode for a method with 28+ if/elif branches
2. The bytecode compiler allocates memory for the branch table
3. dlmalloc's freelist traversal encounters an invalid pointer
4. The `i32.load` instruction at offset 0xa2c0f3 dereferences the invalid pointer

The root cause appears to be related to how wadup builds Python WASM modules:
- **pydantic_core C extension** - May interfere with memory allocation
- **zipimport-based loading** - Modules are loaded from /app/modules.zip
- **Custom entry point** - C main.c with process() function

The standalone Python WASM (`official/python-3.13.0-wasi_sdk-24/python.wasm`) handles the same Python code without issues, confirming this is a wadup build issue, not a general Python WASI issue.

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

## Important: This Issue May Affect Other Python Code

**This is not a pydantic-specific bug.** The crash occurs in Python's WASI runtime during bytecode compilation, not in pydantic code. Any Python code with methods containing 28+ if/elif branches will trigger the same crash.

### Implications

1. **Other libraries may be affected** - Any Python library with methods containing 28+ if/elif branches will crash
2. **The issue is in Python's WASI build** - Specifically in the bytecode compiler and memory allocator
3. **Exact trigger identified** - Methods with 28+ if/elif branches crash; 27 branches works fine

### Workarounds for Other Libraries

If you encounter similar crashes with other libraries:

1. **Check for large if/elif chains** - Look for methods with 28+ if/elif branches
2. **Refactor large switch statements** - Convert if/elif chains to dictionary dispatch:
   ```python
   # Instead of:
   if x == 0: return "a"
   elif x == 1: return "b"
   # ...

   # Use:
   dispatch = {0: "a", 1: "b", ...}
   return dispatch.get(x, "default")
   ```
3. **Pre-compile to .pyc** - Ship bytecode instead of source to avoid runtime compilation
4. **Split methods** - Break large methods into smaller helper methods

### Reporting

This issue should be reported to:
- The CPython WASI maintainers
- The wasmtime team (as it may be related to their dlmalloc implementation)

## Date

Investigation conducted: January 2026
