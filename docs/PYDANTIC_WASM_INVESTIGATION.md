# Pydantic WASM Investigation

This document details the comprehensive investigation into why Python modules crashed when loaded from zipfiles in WASI/WASM, and the fixes that were applied.

## Summary

**Status**: ✅ FULLY FIXED (January 2026)

Two issues were discovered and fixed:

1. **snprintf memory corruption** - Using `snprintf` before `PyRun_SimpleString` caused memory corruption
2. **Bytecode compilation crash** - Large Python files (like pydantic's `_generate_schema.py`) crashed during runtime bytecode compilation

**Solution**: Pre-compile all Python files to `.pyc` bytecode during the build process.

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

**Critical Finding: This is HEAP CORRUPTION, not heap exhaustion.**

Testing with increased memory proves the crash is caused by pointer corruption:

| Memory Size | Faulting Address | Memory Bound | Analysis |
|-------------|------------------|--------------|----------|
| 512 MB | `0xc300b0c6` (3.2 GB) | `0x20000000` (512 MB) | Address 6x beyond memory |
| 1 GB | `0x745f6676` (1.9 GB) | `0x44020000` (~1 GB) | Address 2x beyond memory |

The faulting addresses are **impossible values** - far beyond the allocated WASM linear memory. This proves a pointer got corrupted to garbage, not that the allocator ran out of space. If it were heap exhaustion, we'd see a `MemoryError` exception or allocation failure, not an out-of-bounds memory access at an impossible address.

### Contributing Factors

1. **NOT data segment count**: Testing confirmed that merging 46,446 data segments into 1 segment does NOT fix the crash. The segment count difference between official (2) and WADUP (46K) is not the cause.

2. **Frozen modules impact**: The extensive frozen stdlib (40+ modules, 6.3 MB) creates a different binary structure than the official build (15 modules, 3.0 MB). The exact mechanism is unknown.

3. **Complex control flow triggers corruption**: Files with many if/elif branches (like pydantic's `_generate_schema.py` with 49+ branches) cause specific allocation patterns in the AST/bytecode compiler that trigger the corruption.

4. **zipimport._compile_source is the trigger**: When Python imports a `.py` file from a zipfile, it calls `compile()` which eventually corrupts a pointer.

### Data Segment Investigation

Testing was done to determine if data segment count affects the crash:

| Configuration | Segments | Crash? |
|--------------|----------|--------|
| Original WADUP build | 46,305 | Yes |
| With -O2, --gc-sections, --stack-first | 46,305 | Yes |
| Manually merged to 1 segment | 1 | Yes |

**Conclusion**: Data segment count does NOT cause the crash. The corrupted pointer (0x100000370 = 4.3 GB address in 128 MB memory) appears regardless of segment layout.

### Why Official Build Works

The official Python WASI build works because:
- Minimal frozen modules (15 vs 40+) - fundamentally different binary structure
- Stdlib loaded from filesystem, not compiled from zipfile at runtime
- Different Python initialization sequence (expects external stdlib)
- The exact difference that prevents corruption is unknown but NOT segment count

### Why Pre-compilation Works

Pre-compilation bypasses the problematic code path entirely:

1. Host Python (native macOS/Linux) compiles `.py` to `.pyc` during build
2. `.py` files are removed from the bundle
3. `zipimport` loads `.pyc` directly without calling `compile()`
4. No bytecode compilation occurs at runtime in WASI

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
| January 2, 2026 | Pre-compilation fix applied |
| January 2, 2026 | Deep root cause investigation conducted |
| January 2, 2026 | Official Python WASI comparison completed |
| January 2, 2026 | wasmtime CLI crash verification completed |
| January 2, 2026 | Attempted frozen module reduction (unsuccessful) |
| January 2, 2026 | Memory increase testing (512 MB, 1 GB) - proved heap corruption, not exhaustion |
| January 2, 2026 | Data segment investigation - merging 46K→1 segment did NOT fix crash |
| January 2, 2026 | Ruled out linker optimizations (-O2, --gc-sections, --stack-first) |
| January 2, 2026 | Documentation completed |

---

## Files Modified

| File | Change |
|------|--------|
| `guest/python/src/main_bundled_template.c` | snprintf fix, preprocessor string concatenation |
| `scripts/build-python-project.py` | Pre-compilation with compileall, .py removal |
| `scripts/build-python-wasi.sh` | Extensive frozen stdlib configuration |
| `docs/PYDANTIC_WASM_INVESTIGATION.md` | This documentation |
