# Go SQLite Parser for WADUP

A SQLite database analyzer that extracts table statistics using standard Go and pure Go SQLite library.

## Overview

This example demonstrates how to build WASM modules for WADUP using **standard Go** (not TinyGo). It showcases:

- Standard Go compilation with `GOOS=wasip1 GOARCH=wasm` target
- Pure Go SQLite library (`github.com/ncruces/go-sqlite3`) - no CGO required
- Reactor pattern with `process` export (module reuse like Rust/Python)
- Proper WASI filesystem access for SQLite databases
- Using the shared `go-wadup-guest` library for host FFI

## What It Does

1. Validates SQLite database header (first 16 bytes: "SQLite format 3\x00")
2. Opens database from WADUP's virtual filesystem at `/data.bin`
3. Queries `sqlite_master` table for all user tables (excludes `sqlite_*` system tables)
4. Counts rows in each table using `SELECT COUNT(*)`
5. Outputs results to `db_table_stats` table with columns:
   - `table_name` (String)
   - `row_count` (Int64)

Output is identical to the Rust `sqlite-parser` and Python `python-sqlite-parser` examples.

## Prerequisites

- **Go 1.21+** - Standard Go toolchain (not TinyGo)
- **Make** - Build orchestration

No WASI SDK or special compilers needed - standard Go has built-in WASI support!

## Building

```bash
make
```

Output: `target/go_sqlite_parser.wasm` (~8.3 MB)

## Running

```bash
# Build WADUP CLI (if not already built)
cd ../../
cargo build --release

# Run on a directory containing SQLite databases
./target/release/wadup \
  --modules examples/go-sqlite-parser/target \
  --input /path/to/data \
  --output results.db
```

## Architecture

### Reactor Pattern with `process` Export

Go modules use the **reactor pattern** (module reuse) just like Rust and Python modules:

```go
//go:wasmexport process
func process() int32 {
    // Called repeatedly for each file (module instance reused)
    if err := run(); err != nil {
        fmt.Fprintf(os.Stderr, "Error: %v\n", err)
        return 1
    }
    return 0
}

func main() {
    // Empty - module uses reactor pattern
    // Go runtime initializes on first _start call
}
```

**How It Works:**

1. WADUP loads module and calls `_start` once → Go runtime initializes
2. WADUP calls `process` for each file → Module instance reused
3. Same pattern as Rust/Python modules - no reload overhead
4. Module exports both `_start` (for initialization) and `process` (for processing)

### SQLite Database Access

Use **file URI format** with WASI-compatible flags:

```go
db, err := sql.Open("sqlite3", "file:/data.bin?mode=ro&immutable=1")
```

**Why this format?**
- `file:` prefix enables URI mode
- `mode=ro` - read-only mode (WASI filesystem is read-only)
- `immutable=1` - tells SQLite the database won't change during connection

Direct path access (`/data.bin`) may fail with WASI filesystem constraints.

### WADUP Guest Library

Import the shared `go-wadup-guest` library for host FFI:

```go
import "github.com/tordynnar/wadup2/go-wadup-guest"
```

The library provides:

**Table Builder Pattern:**
```go
table, err := wadup.NewTableBuilder("table_name").
    Column("col1", wadup.String).
    Column("col2", wadup.Int64).
    Build()
```

**Insert Rows:**
```go
err := table.InsertRow([]wadup.Value{
    wadup.NewString("value1"),
    wadup.NewInt64(42),
})
```

**Data Types:**
- `wadup.Int64` - 64-bit signed integer
- `wadup.Float64` - 64-bit floating point
- `wadup.String` - UTF-8 string

## Key Learnings: Go + WADUP

### ✅ What Works

**1. Standard Go (Not TinyGo)**
- Use `GOOS=wasip1 GOARCH=wasm` - built into standard Go 1.21+
- No special compiler setup needed
- Full standard library support

**2. Pure Go Libraries**
- `github.com/ncruces/go-sqlite3` - Pure Go SQLite (works!)
- Avoid libraries requiring CGO or system calls
- Check for WASI compatibility

**3. `//go:wasmexport process`**
- Export `process` function for reactor pattern (module reuse)
- Works with standard Go (no TinyGo needed!)
- Module instance reused across all files (efficient)

**4. File URIs for SQLite**
- Use `file:/data.bin?mode=ro&immutable=1` format
- Required for WASI filesystem compatibility
- Direct paths may fail

**5. Reactor Pattern**
- Module reused across files (same as Rust/Python)
- `_start` initializes Go runtime once
- `process` called repeatedly for each file

### ❌ What Doesn't Work

**1. TinyGo**
- `//go:wasmexport` functions can't be called after `main()` returns
- Reflection limitations break `database/sql`
- Limited standard library support
- Stick with standard Go

**2. CGO Libraries**
- `mattn/go-sqlite3` requires CGO compilation
- Complex WASI SDK setup and linking
- Use pure Go alternatives instead

**3. `modernc.org/sqlite`**
- Build constraints exclude WASI target
- May work with patches but not recommended
- `github.com/ncruces/go-sqlite3` is a better choice

**4. Direct Path Access**
- `sql.Open("sqlite3", "/data.bin")` may fail
- WASI filesystem has restrictions
- Always use file URIs

**5. TinyGo with `//go:wasmexport`**
- TinyGo's `//go:wasmexport` has limitations after `main()` returns
- Standard Go's `//go:wasmexport process` works perfectly!
- No need to use TinyGo - standard Go is better

## Technical Details

### Build Process

The Makefile simply invokes standard Go:

```makefile
GOOS=wasip1 GOARCH=wasm go build -o target/go_sqlite_parser.wasm .
```

No linking, no custom SDK, no multi-stage compilation needed!

### Runtime Behavior

**Module Load (once):**
1. WADUP creates WASM instance
2. Calls `_start()` to initialize Go runtime
3. `main()` returns immediately (empty function)

**File Processing (repeated for each file):**
4. WADUP calls `process()` exported function
5. Module processes file and returns status code
6. Instance reused for next file

This is the **reactor pattern** - module loaded once, used many times.

### Module Size

- **8.3 MB** - Larger than Rust (~2.5 MB) due to Go runtime
- Smaller than Python (~20 MB) with embedded interpreter
- Acceptable tradeoff for development speed and standard library access

### Performance

- Module reuse eliminates per-file initialization overhead
- Go runtime initialized once, not per file
- Same performance characteristics as Rust/Python modules
- SQLite operations dominate processing time

## Project Structure

```
examples/go-sqlite-parser/
├── README.md           # This file
├── Makefile            # Build configuration
├── go.mod              # Go module with dependencies
├── go.sum              # Dependency checksums
├── main.go             # Entry point and business logic
└── target/
    └── go_sqlite_parser.wasm  # Compiled module
```

## Dependencies

### Direct Dependencies

- `github.com/ncruces/go-sqlite3` v0.13.0 - Pure Go SQLite
- `github.com/tordynnar/wadup2/go-wadup-guest` v0.0.0 - WADUP FFI

### Indirect Dependencies

- `github.com/ncruces/julianday` - Date handling for SQLite
- `github.com/tetratelabs/wazero` - WASM runtime (used by go-sqlite3)
- `golang.org/x/sys` - System calls

## Comparison: Go vs Rust vs Python

| Feature | Go | Rust | Python |
|---------|-------|------|--------|
| Build Tool | `go build` | `cargo build` | WASI SDK + make |
| WASM Size | 8.3 MB | 2.5 MB | 20 MB |
| Build Time | ~10s | ~30s | ~5m (first build) |
| Entry Point | `process` | `process` | `process` |
| Module Pattern | Reused | Reused | Reused |
| Standard Library | Full | Full | Full |
| Learning Curve | Low | Medium | Low |

**Use Go When:**
- You prefer Go's syntax and ecosystem
- You want fast build times
- You need standard library access
- Module size is acceptable

**Use Rust When:**
- You need smallest WASM size
- You want maximum performance
- You prefer Rust's type system

**Use Python When:**
- You want fastest prototyping
- You need Python-specific libraries
- Build time is less critical

## Troubleshooting

### "unable to open database file"

Use file URI format:
```go
// ❌ Wrong
db, err := sql.Open("sqlite3", "/data.bin")

// ✅ Correct
db, err := sql.Open("sqlite3", "file:/data.bin?mode=ro&immutable=1")
```

### Import errors for go-wadup-guest

The `go.mod` should have a `replace` directive:
```go
replace github.com/tordynnar/wadup2/go-wadup-guest => ../../go-wadup-guest
```

Run `go mod tidy` if needed.

### Module builds but doesn't process files

Check that:
1. Your `main()` function doesn't return early (errors logged but not fatal)
2. You're calling WADUP FFI functions (`wadup.NewTableBuilder`, etc.)
3. `/data.bin` is being read correctly

### Build fails with "wasip1 not supported"

Update Go to 1.21 or later:
```bash
go version  # Should be 1.21+
```

Earlier versions don't have `wasip1` target.

## Related Examples

- **`sqlite-parser`** - Rust implementation (same functionality)
- **`python-sqlite-parser`** - Python implementation (same functionality)
- **`byte-counter`** - Simple Rust module example
- **`zip-extractor`** - Rust module with subcontent extraction

## Additional Resources

- [Go WASI Support](https://go.dev/blog/wasi)
- [go-sqlite3 Documentation](https://github.com/ncruces/go-sqlite3)
- [WADUP Guest Library](../../go-wadup-guest/)
- [WADUP Architecture](../../docs/architecture.md)

## License

This example is part of the WADUP project.
