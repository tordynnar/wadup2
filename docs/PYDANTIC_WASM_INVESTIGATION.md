# Pydantic WASM Investigation

This document details the investigation into why the full `pydantic` library (specifically `BaseModel`) crashes when used in WASI/WASM, while `pydantic_core` works correctly.

## Summary

**Status**: The full pydantic library cannot be used in WASI due to a crash during Python bytecode compilation.

**Root Cause**: The crash occurs in Python's **zipimport** when loading a sub-module from within a package stored in a zipfile. The specific trigger is **methods with 28+ if/elif branches in an imported sub-module** (not in `__init__.py`).

**Workaround**: Use `pydantic_core` directly instead of the high-level `pydantic` API.

### Key Findings (Updated January 2026)

| Test | Result |
|------|--------|
| 28 if/elif branches in `__init__.py` | ✅ Works |
| 28 if/elif branches in imported sub-module | ❌ Crashes |
| Module without C extensions (no pydantic_core, no lxml) | ❌ Still crashes |
| Module with only pydantic_core | ❌ Still crashes |
| Normal entry point (main calls code directly) | ❌ Still crashes |
| Reactor-style entry point (`--invoke process`) | ❌ Still crashes |

**The crash is caused by zipimport loading sub-modules, NOT by C extensions or entry point style.**

### Precise Trigger

The crash occurs when ALL of these conditions are met:
1. Python code is loaded from a **zipfile** (via `sys.path.insert(0, '/app/modules.zip')`)
2. A package's `__init__.py` **imports a sub-module** (e.g., `from mypackage import submodule`)
3. That **sub-module** contains a method with **28+ if/elif branches**

Code directly in `__init__.py` does NOT crash, even with 100+ if/elif branches.

### Minimal Reproduction

```bash
# This CRASHES - sub-module import with 28 branches
mkdir -p /tmp/wasm-app
wasmtime run --dir=/tmp/wasm-app::/app \
  examples/python-large-file-test/target/python_large_file_test.wasm

# This WORKS - same code directly in __init__.py
wasmtime run --dir=/tmp/wasm-app::/app \
  examples/python-large-file-test/target/test_inline.wasm
```

### Crashing Code Pattern

```python
# mypackage/__init__.py
from mypackage import large_module  # <-- Importing sub-module triggers crash

# mypackage/large_module.py
class CrashTrigger:
    def method_with_many_branches(self, obj: Any) -> str:
        if obj == 0:
            return "case_0"
        elif obj == 1:
            return "case_1"
        # ... 25 more elif branches ...
        elif obj == 27:
            return "case_27"  # 28th branch - CRASH!
        return "default"
```

### What Was Ruled Out

| Hypothesis | Status | Details |
|------------|--------|---------|
| pydantic_core C extension | ❌ Ruled out | Crash happens without any C extensions |
| lxml C extension | ❌ Ruled out | Crash happens without any C extensions |
| Reactor-style entry (`--no-entry`) | ❌ Ruled out | Crash happens with normal entry too |
| wadup library in bundle | ❌ Ruled out | Crash happens without wadup library |
| Stack/memory limits | ❌ Ruled out | Tested with 100MB stack, 1GB memory |

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

## Root Cause: zipimport Sub-Module Loading Bug

The crash is caused by a bug in Python's **zipimport** module when loading sub-modules from packages stored in zipfiles.

**Specific trigger**: When a package's `__init__.py` imports a sub-module, and that sub-module contains a method with **28+ if/elif branches**, the crash occurs during bytecode compilation.

### Why It Only Affects wadup Modules

wadup bundles Python code into `/app/modules.zip` and adds it to `sys.path`:
```python
sys.path.insert(0, '/app/modules.zip')
```

This causes Python to use **zipimport** to load modules. The bug is specifically in zipimport's sub-module loading path - it doesn't affect:
- Code in `__init__.py` (loaded differently)
- Modules loaded from filesystem (not zipimport)
- Standalone Python WASM (loads from filesystem)

### Technical Details

The crash occurs in dlmalloc (WASM function 15532) when:
1. zipimport loads a sub-module from a package in the zipfile
2. Python compiles the sub-module's bytecode
3. The bytecode compiler allocates memory for a method with 28+ branches
4. dlmalloc's freelist traversal encounters an invalid pointer
5. The `i32.load` instruction dereferences the corrupted pointer

The root cause is likely memory corruption in zipimport's code path that doesn't occur when loading from the filesystem or when loading `__init__.py` files.

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

### Potential Workarounds

1. **Pre-compile bytecode** - Ship `.pyc` files instead of `.py` to avoid runtime compilation
2. **Load from filesystem** - Extract modules to filesystem instead of loading from zipfile
3. **Inline code in `__init__.py`** - Move code from sub-modules into `__init__.py` (works but impractical for large libraries)
4. **Refactor large if/elif chains** - Convert to dictionary dispatch:
   ```python
   # Instead of 28 if/elif branches:
   dispatch = {0: "a", 1: "b", ...}
   return dispatch.get(x, "default")
   ```

### Long Term Fix

1. **Report to CPython WASI team** - This is a bug in Python's zipimport for WASI
2. **Investigate frozen zipimport** - The issue may be in the frozen `zipimport` module in libpython
3. **Test with newer Python versions** - May be fixed in future Python releases

## Files Changed

- `examples/python-pydantic-test/` - Test module for investigation
- `crates/wadup-core/src/wasm.rs` - Added stderr logging on errors
- Current test uses `pydantic_core` directly (working solution)

## Related Issues

- The crash is in compiled C code (dlmalloc), not pydantic_core
- WASM function 15532 is the memory allocator, not pydantic-specific code

## Important: This Issue May Affect Other Python Code

**This is not a pydantic-specific bug.** The crash occurs in Python's zipimport module when loading sub-modules from zipfiles. Any Python library that:
1. Is loaded from a zipfile
2. Has sub-modules (not just `__init__.py`)
3. Contains methods with 28+ if/elif branches in those sub-modules

...will trigger the same crash.

### Implications

1. **Other libraries may be affected** - Any library loaded from zipfile with large if/elif chains in sub-modules
2. **The issue is in Python's zipimport** - Specifically when loading sub-modules, not `__init__.py`
3. **Code in `__init__.py` is safe** - Even with 100+ if/elif branches

### Workarounds for Other Libraries

If you encounter similar crashes with other libraries:

1. **Check for sub-module imports** - The crash only happens when importing from sub-modules
2. **Move code to `__init__.py`** - Code directly in `__init__.py` doesn't crash
3. **Pre-compile to .pyc** - Ship bytecode to avoid runtime compilation
4. **Load from filesystem** - Extract modules instead of loading from zipfile
5. **Refactor large if/elif chains** - Convert to dictionary dispatch

### Reporting

This issue should be reported to the **CPython WASI team** as a bug in the frozen zipimport module.

## Date

Investigation conducted: January 2026
