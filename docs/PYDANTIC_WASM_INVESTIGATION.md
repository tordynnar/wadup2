# Pydantic WASM Investigation

This document details the comprehensive investigation into why Python modules crashed when loaded from zipfiles in WASI/WASM, and the fixes that were applied.

## Summary

**Status**: ✅ FULLY FIXED (January 2026)

Three issues were discovered and fixed:

1. **snprintf memory corruption** - Using `snprintf` before `PyRun_SimpleString` caused memory corruption
2. **Stack overflow causing heap corruption** - Missing `--stack-first` linker flag caused stack to overflow into heap
3. **Bytecode compilation crash** - Large Python files crashed during runtime bytecode compilation (now fixed by #2)

**Root Cause**: The `--stack-first` linker flag was missing. This flag places the stack at the beginning of linear memory before data sections, preventing stack growth from corrupting the heap.

**Solution**: Added `--stack-first` and `-z stack-size=8388608` (8 MB stack) to linker flags in `scripts/build-python-project.py`.

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

## Fix 2: Stack-First Linker Flag (THE REAL FIX)

**Root Cause**: The `--stack-first` linker flag was missing from the WADUP build. Without this flag, the WASM linear memory layout places the stack after data sections. When the Python bytecode compiler uses significant stack space for complex control flow (many if/elif branches), the stack overflows into the heap, corrupting pointers.

**Symptoms**:
- Memory fault at impossible addresses (e.g., `0xa1d68e82` in 128 MB linear memory)
- Crashes in `label_exception_targets` → `_PyCfg_OptimizeCodeUnit` → `compiler_function`
- Only occurs during bytecode compilation of complex code
- Official Python WASI build works (because it uses `--stack-first`)

**Fix**: Added `--stack-first` and `-z stack-size=8388608` (8 MB stack, same as official CPython WASI) to linker flags in `scripts/build-python-project.py`:

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

**Why it matters**:
- Without `--stack-first`: stack grows downward into heap, causing corruption
- With `--stack-first`: stack is at the beginning of memory, can't corrupt heap
- The 8 MB stack size matches the official Python WASI configuration

**Testing confirmed**: After adding `--stack-first`, pydantic's full BaseModel works correctly with runtime bytecode compilation from zipfiles.

## Fix 3: Pre-compile Python to Bytecode (OPTIONAL OPTIMIZATION)

**Note**: This is no longer required after Fix 2, but kept for performance optimization.

Pre-compilation provides faster startup by avoiding runtime compilation:

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

## Testing

All 11 integration tests pass:

```bash
./scripts/run-integration-tests.sh

# Output:
# ⏱️  test_python_pydantic passed in 1.83s
# ...
# Tests passed: 11
# Tests failed: 0
```

The pydantic test uses full `pydantic.BaseModel`:

```python
from pydantic import BaseModel

class User(BaseModel):
    name: str
    age: int
    email: str

user = User(name="Alice", age=30, email="alice@example.com")
```

---

## Deep Investigation: Root Cause Analysis

### Overview

A comprehensive investigation was conducted to understand why Python's bytecode compiler crashes in our WASI build but not in the official Python WASI build.

### Investigation Methodology

1. **Created minimal reproducer** - Standalone C test harness that links Python WASI and imports modules from a zipfile
2. **Binary search for trigger** - Systematically reduced test module complexity to find the minimum trigger
3. **Memory analysis** - Compared memory configuration between our build and the official Python WASI
4. **WASI SDK version testing** - Tested SDK 24 vs 29
5. **Official Python WASI comparison** - Downloaded and tested `brettcannon/cpython-wasi-build`
6. **wasmtime CLI testing** - Verified crash occurs in wasmtime CLI, not just WADUP host
7. **Attempted fixes** - Tried reducing frozen modules and bundling stdlib differently

---

## Key Finding: wasmtime CLI Crash Verification

**Critical discovery**: The crash happens in wasmtime CLI, not just the WADUP host.

### Test Results

| Scenario | wasmtime CLI Result |
|----------|---------------------|
| WADUP module WITH pre-compilation | ✅ Works |
| WADUP module WITHOUT pre-compilation | ❌ Crashes (function 15506) |
| Official Python WASI (100 branches) | ✅ Works |

### wasmtime CLI Test Commands

**WADUP module (pre-compiled) - WORKS:**
```bash
wasmtime run \
    --dir /tmp/test::/app \
    --invoke process \
    examples/python-pydantic-test/target/python_pydantic_test.wasm

# Output: Python runs, only fails on filesystem access (expected)
```

**WADUP module (no pre-compilation) - CRASHES:**
```bash
WADUP_SKIP_PRECOMPILE=1 ./scripts/build-python-project.py examples/python-pydantic-test
wasmtime run \
    --dir /tmp/test::/app \
    --invoke process \
    examples/python-pydantic-test/target/python_pydantic_test.wasm

# Output:
# Error: failed to invoke `process`
# Caused by:
#     error while executing at wasm backtrace:
#         0: 0xa0ee8e - <unknown>!<wasm function 15506>
#         ... (bytecode compiler crash)
```

**Official Python WASI - WORKS:**
```bash
wasmtime run \
    --dir /tmp/python-official/lib::/usr/local/lib/python3.14 \
    --dir /tmp/test::/test \
    --env PYTHONHOME=/usr/local \
    /tmp/python-official/python.wasm /test/complex_schema.py

# Output:
# Testing complex schema generator with 100 branches...
# Result: {'type': 'uuid'}
# SUCCESS - bytecode compiled correctly!
```

**Conclusion**: The crash is in Python's bytecode compiler in WASI, not in the WADUP host code or wasmtime runtime.

---

## Memory Configuration Analysis

### Data Section Comparison

| Build | Code Size | Data Size | Data Segments | Bytecode Compilation |
|-------|-----------|-----------|---------------|---------------------|
| Official Python WASI 3.14 | 4.2 MB | **3.0 MB** | 2 | ✅ Works |
| WADUP Python Module | 10.2 MB | **8.1 MB** | 46,446 | ❌ Crashes |

### Memory Pages Comparison

| Build | Initial Memory | Max Memory |
|-------|---------------|------------|
| Official Python WASI | 640 pages (40 MB) | unlimited |
| WADUP Python Module | 2048 pages (128 MB) | 4096 pages (256 MB) |

**Key insight**: Despite having MORE total memory (128 MB vs 40 MB), WADUP modules crash because the bytecode compiler needs contiguous heap space, which is consumed by the larger data sections.

### Frozen Modules Impact

| Build | Frozen Modules | Data Section Impact |
|-------|----------------|---------------------|
| Official Python WASI | ~15 bootstrap modules | 3.0 MB |
| WADUP Python WASI | 40+ stdlib modules | 6.3 MB |

Our Python WASI build freezes an extensive stdlib:
- encodings, collections, email, html, importlib, json, logging, pathlib, re, sqlite3, sysconfig, tomllib, urllib, xml, zipfile, zoneinfo, and 40+ more modules

This results in **6.3 MB of static data** vs the official build's **3.0 MB**.

---

## Two Different Crash Modes

### Mode A: MemoryError (Low Memory Scenarios)

```
MemoryError during zipimport._compile_source
```

This occurs when there's insufficient heap for bytecode compilation. Seen in:
- Standalone test harness with 7.5 MB initial memory
- Simple modules with 60+ if/elif branches

### Mode B: Out of Bounds Memory Access (WADUP Modules)

```
memory fault at wasm address 0x53000004 in linear memory of size 0xa10000
```

Or in wasmtime CLI:
```
error while executing at wasm backtrace:
    0: 0xa0ee8e - <unknown>!<wasm function 15506>
```

This occurs even with 128 MB total memory, suggesting heap fragmentation or corruption in the bytecode compiler's memory allocation patterns.

### Crash Threshold by Complexity

| Module Complexity | Standalone Harness (7.5MB) | WADUP Module (128MB) |
|-------------------|---------------------------|---------------------|
| 50 if/elif branches | ✅ Works | ❌ Crashes |
| 60+ if/elif branches | ❌ MemoryError | ❌ Crashes |
| pydantic _generate_schema.py (49+ branches) | ❌ MemoryError | ❌ Crashes |
| Official Python WASI (100 branches) | N/A | ✅ Works |

---

## WASI SDK Version Testing

| WASI SDK | Crash Behavior |
|----------|---------------|
| SDK 29 | Crashes at function 15532 |
| SDK 24 | Crashes at function 15506 |

**Conclusion**: Both SDK versions crash. The function number difference is just due to code layout changes. WASI SDK version is NOT the root cause.

---

## Attempted Fix: Reduce Frozen Modules

### Hypothesis

Reducing frozen modules to the minimal set (like the official Python WASI) would free heap space for bytecode compilation.

### Changes Made

1. **Modified `build-python-wasi.sh`**:
   - Removed the extensive frozen stdlib modification
   - Added stdlib bundling to copy Python `Lib/*.py` to a separate directory

2. **Modified `build-python-project.py`**:
   - Added code to bundle Python stdlib from `build/python-wasi/stdlib/` into the zipfile

3. **Modified `main_bundled_template.c`**:
   - Changed initialization to use `PyConfig.module_search_paths` before `Py_InitializeFromConfig()`
   - This allows Python to find stdlib modules in the zipfile during initialization

### Results

| Configuration | Data Sections | Bytecode Compilation |
|---------------|--------------|---------------------|
| Official Python WASI | 3.0 MB | ✅ Works |
| WADUP with reduced frozen | 6.3 MB | ❌ Still crashes |
| Original WADUP (frozen stdlib) | 8.1 MB | ❌ Crashes |

### Why It Didn't Work

The reduction from 8.1 MB to 6.3 MB wasn't enough because:

1. **C extension libraries still required**: lxml (~2 MB) and pydantic_core (~1 MB) are bundled as static libraries
2. **Zipfile overhead**: The stdlib bundled in the zipfile still consumes memory when extracted
3. **Architectural difference**: Official Python WASI expects stdlib on filesystem, not in memory

### Conclusion

Partial reduction of frozen modules is insufficient. Would need to completely restructure the build to match the official Python WASI approach (expecting stdlib on filesystem), which conflicts with WADUP's goal of self-contained, sandboxed WASM modules.

---

## Official Python WASI Build Analysis

### Source

The official Python WASI build from `brettcannon/cpython-wasi-build`:
- GitHub: https://github.com/brettcannon/cpython-wasi-build
- Build method: `python3 Tools/wasm/wasi build`

### Key Characteristics

| Characteristic | Official Python WASI | WADUP Python |
|----------------|---------------------|--------------|
| Frozen modules | ~15 (bootstrap only) | 40+ (full stdlib) |
| Stdlib location | Filesystem (`/usr/local/lib/python3.X/`) | Embedded in binary |
| Self-contained | ❌ No (needs external files) | ✅ Yes |
| Data sections | 3.0 MB | 8.1 MB |
| Runtime bytecode compilation | ✅ Works | ❌ Crashes |

### Frozen Modules in Official Build

The official build freezes only essential bootstrap modules:

```
importlib._bootstrap
importlib._bootstrap_external
zipimport
abc
codecs
io
os
site
stat
posixpath
genericpath
_collections_abc
_sitebuiltins
importlib.util
importlib.machinery
runpy
```

### Architectural Difference

**Official Python WASI**:
- Minimal binary with bootstrap modules
- Expects stdlib on host filesystem
- Host must provide `/usr/local/lib/python3.X/` directory
- ~30 MB WASM file + ~15 MB stdlib on disk

**WADUP Python**:
- Self-contained binary with everything bundled
- No external dependencies
- Sandboxed execution (no host filesystem access)
- ~38 MB WASM file (everything included)

---

## Root Cause Summary

**Critical Finding: STACK OVERFLOW INTO HEAP**

The root cause was the missing `--stack-first` linker flag. Without this flag, the WASM linear memory layout is:

```
Without --stack-first:
[Data Sections][Heap→ ←Stack]

With --stack-first:
[Stack][Data Sections][Heap→]
```

When Python's bytecode compiler processes complex control flow (many if/elif branches), it uses significant stack space. Without `--stack-first`, the stack grows downward and corrupts the heap, causing:

| Memory Size | Faulting Address | Memory Bound | Analysis |
|-------------|------------------|--------------|----------|
| 128 MB | `0xa1d68e82` (2.7 GB) | `0x8000000` (128 MB) | Stack overflow corrupted heap pointers |
| 512 MB | `0xc300b0c6` (3.2 GB) | `0x20000000` (512 MB) | Same pattern - corrupted pointers |
| 1 GB | `0x745f6676` (1.9 GB) | `0x44020000` (~1 GB) | Same pattern - corrupted pointers |

The faulting addresses are **garbage values** written when the stack overwrote heap metadata. The official Python WASI build uses `--stack-first` which prevented this issue.

### Discovery Process

1. **Initial hypothesis**: Frozen module size/count causes memory exhaustion
2. **Tested**: Merging 46K data segments into 1 - Still crashed
3. **Tested**: Increasing memory to 512 MB, 1 GB - Still crashed
4. **Tested**: Various linker flags (-O2, --gc-sections) - Still crashed
5. **Built minimal Python WASI** from scratch with official config - Crashed
6. **Compared linker flags** with official `python3 Tools/wasm/wasi.py build` output
7. **Found**: Official uses `-z stack-size=8388608 -Wl,--stack-first`
8. **Added to WADUP**: Fixed!

### Key Comparison

| Configuration | Linker Flags | Result |
|--------------|--------------|--------|
| WADUP (before fix) | `--initial-memory=128MB` only | ❌ Crash |
| WADUP (after fix) | `--stack-first -z stack-size=8MB` | ✅ Works |
| Official Python WASI | `--stack-first -z stack-size=8MB` | ✅ Works |

### Why Pre-compilation Was a Workaround

Pre-compilation worked because it avoided triggering the stack-intensive bytecode compiler path. The real fix (--stack-first) allows runtime bytecode compilation to work correctly.

---

## Current Solution

**Pre-compilation is the recommended and implemented approach** because:

| Benefit | Description |
|---------|-------------|
| Reliable | All 11 integration tests pass |
| No runtime cost | Bytecode is ready to execute |
| Universal | Works with any complexity level |
| Minimal overhead | Bundle size increases slightly (.pyc > .py) but acceptable |
| Self-contained | Maintains WADUP's sandboxed execution model |

### Implementation

Pre-compilation is enabled by default in `scripts/build-python-project.py`. To disable for debugging:

```bash
WADUP_SKIP_PRECOMPILE=1 ./scripts/build-python-project.py <project>
```

---

## Future Considerations

If runtime bytecode compilation is ever needed, potential approaches include:

1. **Debug dlmalloc corruption** - Instrument dlmalloc to detect when pointer corruption occurs. The impossible addresses suggest a use-after-free or buffer overflow in the allocator bookkeeping.

2. **Reduce frozen modules to match official build** - The official Python WASI freezes only ~15 bootstrap modules. Matching this configuration might avoid the corruption, but requires providing stdlib on filesystem.

3. **Use official Python WASI architecture** - Require stdlib on filesystem instead of bundled. This works but breaks WADUP's sandboxed, self-contained model.

4. **Custom memory allocator** - Replace dlmalloc with a hardened allocator that can detect corruption earlier.

5. **Trace the exact corruption point** - Use WASM debugging tools to find the exact instruction that writes the corrupted pointer value.

**Ruled out approaches:**
- **Increasing memory** - Testing with 512 MB and 1 GB showed the same corruption pattern
- **Data segment consolidation** - Merging 46K segments into 1 segment did NOT fix the crash
- **Linker optimizations** - `-O2`, `--gc-sections`, `--stack-first` had no effect

For now, pre-compilation is the robust solution that works with WADUP's self-contained architecture.

---

## Timeline

| Date | Event |
|------|-------|
| January 2, 2026 | snprintf memory corruption fix applied |
| January 2, 2026 | Pre-compilation workaround applied |
| January 2, 2026 | Deep root cause investigation conducted |
| January 2, 2026 | Official Python WASI comparison completed |
| January 2, 2026 | wasmtime CLI crash verification completed |
| January 2, 2026 | Attempted frozen module reduction (unsuccessful) |
| January 2, 2026 | Memory increase testing (512 MB, 1 GB) - proved heap corruption |
| January 2, 2026 | Data segment investigation - merging 46K→1 segment did NOT fix crash |
| January 2, 2026 | Ruled out various linker optimizations |
| January 2, 2026 | Built minimal Python WASI from scratch |
| January 3, 2026 | **ROOT CAUSE FOUND: Missing `--stack-first` linker flag** |
| January 3, 2026 | Compared official build linker output - found `-z stack-size=8388608 -Wl,--stack-first` |
| January 3, 2026 | Applied fix to `scripts/build-python-project.py` |
| January 3, 2026 | Tested pydantic with `--stack-first` - **WORKS!** |
| January 3, 2026 | Documentation updated with final root cause |

---

## Files Modified

| File | Change |
|------|--------|
| `guest/python/src/main_bundled_template.c` | snprintf fix, preprocessor string concatenation |
| `scripts/build-python-project.py` | Added `--stack-first` and `-z stack-size=8388608` linker flags; Pre-compilation optimization |
| `scripts/build-python-wasi.sh` | Extensive frozen stdlib configuration |
| `docs/PYDANTIC_WASM_INVESTIGATION.md` | This documentation |
