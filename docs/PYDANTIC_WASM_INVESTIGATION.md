# Pydantic WASM Investigation

This document details the investigation into why Python modules crashed when loaded from zipfiles in WASI/WASM, and the fix that was applied.

## Summary

**Status**: ✅ FIXED (January 2026)

**Root Cause**: Using `snprintf` before calling `PyRun_SimpleString` caused memory corruption when Python imported modules from a zipfile. This was a bug in the interaction between the C standard library's `snprintf` and Python's WASI build.

**Fix**: Replace runtime `snprintf` string formatting with compile-time C preprocessor string concatenation.

## The Fix

In `guest/python/src/main_bundled_template.c`, the original code used:

```c
// BROKEN - causes memory corruption
char cmd[512];
snprintf(cmd, sizeof(cmd),
    "import %s as _m; _m.main() if hasattr(_m, 'main') else None",
    ENTRY_MODULE);
PyRun_SimpleString(cmd);
```

The fix uses compile-time string concatenation instead:

```c
// FIXED - uses preprocessor string concatenation
#define IMPORT_CMD "import " ENTRY_MODULE " as _m; _m.main() if hasattr(_m, 'main') else None"
PyRun_SimpleString(IMPORT_CMD);
```

## Technical Details

### Symptoms

When loading Python modules from a zipfile (via `sys.path.insert(0, '/app/modules.zip')`), the crash occurred:
1. Only when using `snprintf` to build the import command string
2. Only when the imported module loaded sub-modules with complex code (28+ if/elif branches)
3. The crash was in dlmalloc (the WASM memory allocator) with "memory fault at wasm address"

### What Was Ruled Out

Through extensive testing, the following were ruled out as causes:

| Hypothesis | Status | Details |
|------------|--------|---------|
| Python's frozen zipimport module | ❌ Ruled out | Official Python WASM works fine with same zipfile |
| pydantic_core C extension | ❌ Ruled out | Crash happened without any C extensions |
| lxml C extension | ❌ Ruled out | Crash happened without any C extensions |
| Reactor-style entry (`--no-entry`) | ❌ Ruled out | Crash happened with normal entry too |
| WASI SDK 29 vs 24 | ❌ Ruled out | Both SDKs had the issue |
| Stack/memory limits | ❌ Ruled out | Tested with 100MB stack, 1GB memory |

### The Discovery Process

1. **Comparison testing** - Discovered that official Python WASM handled the same zipfile import perfectly
2. **Code structure analysis** - Found that using literal strings worked, but `snprintf`-built strings caused crashes
3. **Minimal reproduction** - Created test cases showing:
   - `PyRun_SimpleString("import module; module.main()")` → ✅ Works
   - `snprintf(cmd, ...); PyRun_SimpleString(cmd);` → ❌ Crashes

### Why snprintf Causes the Problem

The exact mechanism is unclear, but the issue appears to be memory corruption that occurs when:
1. `snprintf` writes to a stack-allocated buffer
2. The buffer is then passed to `PyRun_SimpleString`
3. Python's import machinery (specifically zipimport) allocates memory
4. The allocator encounters corrupted freelist pointers

This is likely a bug in:
- The WASI libc's `snprintf` implementation, OR
- Python's memory allocator's interaction with stack-allocated strings in WASI

## Testing

The fix was verified by:
1. Rebuilding the python-large-file-test example
2. Running integration tests - 10 of 11 pass (pydantic test has unrelated issue)
3. Testing with wadup CLI - modules load and execute correctly

```bash
# Test command
./target/release/wadup run \
  --modules /tmp/test/modules \
  --input /tmp/test/input \
  --output /tmp/test/output.db

# Output shows successful execution:
# DEBUG: large_module imported successfully!
# SUCCESS!
```

## Workaround for Similar Issues

If you encounter similar crashes in WASI/Python builds:

1. **Avoid `snprintf` before Python calls** - Use compile-time string concatenation instead
2. **Use string literals** - Prefer `"literal"` over dynamically built strings
3. **Check for memory corruption patterns** - Crashes in dlmalloc freelist traversal indicate this issue

## Remaining Issues

The pydantic test still fails with "Table 'users' not found" - this is a separate issue unrelated to the memory corruption fix.

## Date

- Investigation conducted: January 2026
- Fix applied: January 2, 2026
