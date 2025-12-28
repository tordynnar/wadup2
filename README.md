# WADUP - Web Assembly Data Unified Processing

A high-performance parallel processing framework that executes sandboxed WebAssembly modules on content, collecting metadata and extracting sub-content for recursive processing.

## Features

- **Parallel Processing**: Work-stealing threadpool for optimal CPU utilization
- **Module Reuse**: WASM modules loaded once at startup and reused across all files, eliminating per-file initialization overhead
- **Sandboxed Execution**: WASM modules run in isolated environments with configurable resource limits
- **Resource Control**: CPU (fuel), memory, stack size, and recursion depth limits
- **Metadata Collection**: SQLite database with automatic schema validation
- **Zero-Copy Architecture**: Memory-mapped file loading and SharedBuffer-based content slicing without duplication
- **Recursive Processing**: Sub-content automatically queued for processing
- **Ergonomic API**: Rust guest library for easy WASM module development

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/tordynnar/wadup2.git
cd wadup2

# Build the project
cargo build --release

# The binary will be at target/release/wadup
```

### Basic Usage

```bash
wadup \
  --modules ./modules \
  --input ./data \
  --output results.db \
  --threads 8
```

## Writing WASM Modules

WADUP modules can be written in **Rust**, **Python**, **Go**, or **C#**, all compiled to the `wasm32-wasip1` (WASI) target.

### Virtual Filesystem

Each WASM module runs in a sandboxed virtual filesystem where:
- **`/data.bin`** - The content being processed (read-only, zero-copy reference)
- **`/tmp/`** - Available for temporary files (read-write)
- **`/metadata/`** - For file-based metadata output (C# modules)
- **`/subcontent/`** - For file-based sub-content emission (C# modules)

Modules can access content using standard file I/O operations. The `/data.bin` file is a zero-copy reference to the content data, implemented using `bytes::Bytes` for optimal memory efficiency.

### Language Support

WADUP supports four languages for writing modules:

| Language | Entry Point | Module Pattern | Interface | WASM Size | Build Time |
|----------|-------------|----------------|-----------|-----------|------------|
| **Rust** | `process()` | Reused | FFI API | ~2.5 MB | ~30s |
| **Python** | `process()` | Reused | FFI API | ~20 MB | ~5m (first) |
| **Go** | `process()` | Reused | FFI API | ~8.3 MB | ~10s |
| **C#** | `_start` | Reload-per-call | File-based | ~17 MB | ~15s |

**Rust** modules export a `process()` function and are reused across files (one instance processes all files per thread).

**Python** modules use embedded CPython 3.13.7 and are also reused (interpreter initialized once per thread).

**Go** modules export a `process()` function using `//go:wasmexport` and are reused like Rust/Python.

**C#** modules use the `_start` entry point with reload-per-call (fresh instance per file).

See language-specific guides:
- [Rust Examples](examples/sqlite-parser/README.md)
- [Python Guide](examples/python-sqlite-parser/README.md)
- [Go Guide](examples/go-sqlite-parser/README.md)
- [C# Guide](examples/csharp-json-analyzer/README.md)

### Module Interface Methods

WADUP supports two methods for modules to output metadata:

#### 1. FFI API (Rust, Python, Go)

Modules import host functions directly via WebAssembly imports:

```
define_table(name_ptr, name_len, columns_ptr, columns_len) -> i32
insert_row(table_ptr, table_len, row_ptr, row_len) -> i32
emit_subcontent(name_ptr, name_len, data_ptr, data_len) -> i32
```

These are wrapped by language-specific guest libraries (`wadup-guest`, `python-wadup-guest`, `go-wadup-guest`) providing ergonomic APIs.

**Advantages:**
- Immediate processing (no file I/O overhead)
- Module reuse across files (reactor pattern)
- Zero-copy data sharing where possible

#### 2. File-Based Communication (C#)

Modules write JSON to special directories:

**Metadata** (`/metadata/*.json`):
```json
{
  "tables": [
    {"name": "my_table", "columns": [{"name": "col", "data_type": "Int64"}]}
  ],
  "rows": [
    {"table_name": "my_table", "values": [{"Int64": 42}]}
  ]
}
```

**Sub-Content** (paired files, zero-copy):
- `/subcontent/data_N.bin` - Raw binary data (written directly to `BytesMut`)
- `/subcontent/metadata_N.json` - Filename metadata (write last to trigger processing)
```json
{"filename": "extracted.txt"}
```

WADUP processes these files immediately when the metadata file is closed (via `fd_close`). The data is extracted as `Bytes` without copying (the `BytesMut` is frozen directly into `Bytes`), then passed to nested processing zero-copy.

**Advantages:**
- Works with languages that don't support custom WASM imports (like .NET WASI SDK)
- Uses standard file I/O (no FFI complexity)
- Incremental flushing supported (write multiple files during processing)
- Both metadata and sub-content supported via file-based interface
- **Zero-copy sub-content**: Data flows from WASM write → nested processing without copying

**Trade-offs:**
- Module reloaded for each file (command pattern)
- Per-file overhead (~200ms for .NET runtime initialization)

### Example: File Size Counter (Rust)

```rust
use wadup_guest::*;

#[no_mangle]
pub extern "C" fn process() -> i32 {
    if let Err(_) = run() {
        return 1;
    }
    0
}

fn run() -> Result<(), String> {
    // Define a table to store file sizes
    let table = TableBuilder::new("file_sizes")
        .column("size_bytes", DataType::Int64)
        .build()?;

    // Get content size from the virtual filesystem
    let metadata = std::fs::metadata(Content::path())
        .map_err(|e| format!("Failed to get metadata: {}", e))?;
    let size = metadata.len() as i64;

    // Insert into database
    table.insert(&[Value::Int64(size)])?;

    Ok(())
}
```

### Building WASM Modules

```bash
# Add to Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wadup-guest = { path = "../../crates/wadup-guest" }

# Build (requires WASI target)
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```

The compiled `.wasm` file can then be placed in your modules directory.

**Note**: For modules that use C dependencies (like `rusqlite`), you'll need the WASI SDK. See the [sqlite-parser example](examples/sqlite-parser/README.md) for details.

## CLI Options

```
Options:
  --modules <MODULES>
      Directory containing WASM modules

  --input <INPUT>
      Directory containing input files

  --output <OUTPUT>
      Output SQLite database path

  --threads <THREADS>
      Number of worker threads [default: 4]

  --fuel <FUEL>
      CPU limit per module per content (e.g., 10000000)

  --max-memory <MAX_MEMORY>
      Max memory in bytes per module instance (e.g., 67108864 for 64MB)

  --max-stack <MAX_STACK>
      Max stack size in bytes per module instance (e.g., 1048576 for 1MB)

  --max-recursion-depth <MAX_RECURSION_DEPTH>
      Maximum sub-content nesting levels [default: 100]

  -v, --verbose
      Verbose output
```

## Architecture

WADUP consists of three main crates:

### wadup-core
The processing engine containing:
- **SharedBuffer**: Zero-copy memory abstraction using `bytes::Bytes` with memory-mapped file loading
- **Content Store**: Zero-copy content management with SharedBuffer-based slicing
- **WASM Runtime**: wasmtime integration with resource limits and virtual filesystem
- **Metadata Store**: SQLite with schema validation and WAL mode
- **Processor**: Work-stealing parallel execution
- **Host Bindings**: FFI exports for WASM modules (define_table, insert_row, emit_subcontent, etc.)

### Module Lifecycle and Performance

WADUP is designed for efficient processing of many files:

1. **Module Loading** (startup): All `.wasm` files are loaded from the modules directory and compiled once
2. **Instance Creation** (per thread): Each worker thread creates one instance of each module
3. **File Processing** (runtime): Module instances handle files based on their pattern:
   - **Rust/Python/Go** (reactor): Same instance processes all files assigned to that thread
   - **C#** (command): Fresh instance created for each file

**Module Reuse Benefits** (Rust/Python/Go - Reactor Pattern):
- Module compilation happens once at startup, not per file
- WASM linear memory persists across files, allowing modules to maintain state if desired
- For Python modules using CPython, the interpreter is initialized once per thread and reused for all files
- Eliminates per-file initialization overhead (especially important for Python: ~20ms saved per file)

**Example**: Processing 1000 SQLite databases with the Python module:
- Without reuse: 1000 × 20ms = 20 seconds wasted on Python initialization
- With reuse: 1 × 20ms = 20ms total initialization (999× speedup)

**Reload-Per-Call Pattern** (C# - Command Pattern):
- C# modules use `_start` entry point and reload for each file
- Ensures clean state between files (no shared memory)
- .NET runtime initialization is ~200ms per file
- Uses file-based metadata output (writes to `/metadata/*.json`)
- Best for processing fewer, larger files where processing time dominates

This architecture makes WADUP suitable for batch processing large numbers of files efficiently.

### Guest Libraries

Language-specific libraries for WASM module authors:

**wadup-guest** (Rust):
- **Content API**: Read content data and metadata
- **Table API**: Define schemas and insert rows
- **SubContent API**: Emit sub-content for recursive processing
- Uses FFI imports to host functions

**python-wadup-guest** (Python):
- Embedded Python extension module providing `wadup.define_table()` and `wadup.insert_row()`
- C-based FFI bridge to host functions
- Used by all Python WASM modules

**go-wadup-guest** (Go):
- Pure Go library with `//go:wasmimport` FFI bindings
- Table builder API: `wadup.NewTableBuilder()`
- Value types: `wadup.Int64`, `wadup.String`, `wadup.Float64`

**csharp-wadup-guest** (C#):
- File-based metadata output (writes JSON to `/metadata/*.json`)
- File-based sub-content emission (writes JSON to `/subcontent/*.json`)
- Table builder API: `new TableBuilder("name").AddColumn(...).Build()`
- Value factory methods: `Value.FromInt64()`, `Value.FromString()`, `Value.FromFloat64()`
- `MetadataWriter.Flush()` writes and closes metadata files for immediate processing
- `SubContentWriter.Emit()` / `SubContentWriter.Flush()` for sub-content emission

### wadup-cli
Command-line interface for running WADUP processing jobs.

## Guest API Reference

### Content Access

Content is accessible as a file in the virtual filesystem:

```rust
use std::fs::File;
use std::io::Read;

// Get the content file path
let path = Content::path();  // Returns "/data.bin"

// Get content size
let metadata = std::fs::metadata(path)?;
let size = metadata.len();

// Read entire content
let mut file = File::open(path)?;
let mut data = Vec::new();
file.read_to_end(&mut data)?;

// Read content as UTF-8 string
let text = std::fs::read_to_string(path)?;

// Use with other file readers (e.g., ZIP, SQLite)
let file = File::open(path)?;
let archive = zip::ZipArchive::new(file)?;
```

### Metadata Tables

```rust
// Define a table
let table = TableBuilder::new("my_table")
    .column("name", DataType::String)
    .column("count", DataType::Int64)
    .column("ratio", DataType::Float64)
    .build()?;

// Insert rows
table.insert(&[
    Value::String("example".to_string()),
    Value::Int64(42),
    Value::Float64(3.14),
])?;
```

### Sub-Content Emission

```rust
// Emit owned bytes as sub-content
SubContent::emit_bytes(
    vec![1, 2, 3, 4],
    "extracted.bin"
)?;

// Emit slice of parent content (zero-copy)
SubContent::emit_slice(
    offset,
    length,
    "slice.dat"
)?;
```

## Database Schema

WADUP automatically creates a `__wadup_content` table to track processing:

```sql
CREATE TABLE __wadup_content (
    uuid TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    parent_uuid TEXT,           -- NULL for top-level content
    processed_at INTEGER NOT NULL,
    status TEXT NOT NULL,       -- 'success' or 'failed'
    error_message TEXT
);
```

Module-defined tables use `content_uuid` as a foreign key to `__wadup_content.uuid`.

## Examples

See the `examples/` directory for working WASM modules:

**Rust Modules:**
- **byte-counter**: Counts and records file sizes
- **zip-extractor**: Extracts files from ZIP archives
- **sqlite-parser**: Parses SQLite databases using SQL queries

**Python Modules:**
- **python-sqlite-parser**: Parses SQLite databases using CPython 3.13.7
- **python-counter**: Demonstrates module reuse with global state
- **python-module-test**: Tests C extension imports

**Go Modules:**
- **go-sqlite-parser**: Parses SQLite databases using pure Go SQLite library

**C# Modules:**
- **csharp-json-analyzer**: Analyzes JSON structure using System.Text.Json
  - Demonstrates file-based metadata output
  - Shows incremental flushing (multiple metadata files per run)
  - Creates two tables: `json_metadata` and `json_keys`

All examples use the WASI target (`wasm32-wasip1`) to access the virtual filesystem.

### Building Examples

**Rust Examples** (byte-counter, zip-extractor):
```bash
cd examples/byte-counter
cargo build --target wasm32-wasip1 --release
```

**SQLite Parser** (Rust with C dependencies):
```bash
cd examples/sqlite-parser
./build.sh
```

The build script will automatically:
- Detect your platform
- Download WASI SDK if not present
- Build the module for wasm32-wasip1 target

See [examples/sqlite-parser/README.md](examples/sqlite-parser/README.md) for detailed documentation.

**Python Modules** (CPython 3.13.7):

First, build the shared Python WASI runtime (one-time, ~5-10 minutes):
```bash
./scripts/build-python-wasi.sh
```

Then build individual Python modules:
```bash
cd examples/python-sqlite-parser
make

cd ../python-counter
make
```

The shared Python WASI build (`build/python-wasi/`) includes:
- CPython 3.13.7 compiled for wasm32-wasip1
- SQLite 3.45.1 for WASI
- Frozen Python standard library
- All dependencies (~45MB)

All Python modules link against this shared build, avoiding duplication.

**Important**: The Python interpreter is initialized once per worker thread and reused across all files. Python global variables persist between files processed by the same thread. The module's `process()` function should be idempotent or explicitly reset state as needed.

See [examples/python-sqlite-parser/README.md](examples/python-sqlite-parser/README.md) for complete documentation, architecture details, and troubleshooting.

**Go Modules** (Standard Go 1.21+):

```bash
cd examples/go-sqlite-parser
make
```

Go modules use standard Go (not TinyGo) with `GOOS=wasip1 GOARCH=wasm` target. No special setup required - standard Go has built-in WASI support!

**Key Features**:
- Pure Go libraries work (e.g., `github.com/ncruces/go-sqlite3`)
- `process()` export via `//go:wasmexport` for reactor pattern
- Fast build times (~10 seconds)
- Moderate WASM size (~8.3 MB)

See [examples/go-sqlite-parser/README.md](examples/go-sqlite-parser/README.md) for complete guide, best practices, and what works/doesn't work with Go+WASM.

**C# Modules** (.NET 8 with Wasi.Sdk):

First, install the WASI workload:
```bash
dotnet workload install wasi-experimental
```

Then build:
```bash
cd examples/csharp-json-analyzer
make
```

**Key Features**:
- .NET 8 with `Wasi.Sdk` NuGet package
- `WasmSingleFileBundle` for single .wasm output (~17 MB)
- File-based metadata output (writes to `/metadata/*.json`)
- Incremental flushing supported (process metadata immediately on file close)
- Uses command pattern (module reloaded for each file)

**Important**: C# modules use file-based communication because .NET WASI SDK doesn't support custom WASM imports. The `csharp-wadup-guest` library handles JSON serialization and file management.

See [examples/csharp-json-analyzer/README.md](examples/csharp-json-analyzer/README.md) for complete guide and API reference.

## Development

### Prerequisites

**Core Framework:**
- Rust 1.70+
- wasm32-wasip1 target: `rustup target add wasm32-wasip1`

**Module Development (choose based on your language):**
- **Rust modules**: wasm32-wasip1 target (already installed above)
- **Python modules**: WASI SDK (auto-downloaded by build script)
- **Go modules**: Go 1.21+ (WASI support built-in, no extra tools needed)
- **C# modules**: .NET 8 SDK + WASI workload (`dotnet workload install wasi-experimental`)

### Building

```bash
# Build all crates
cargo build --release

# Build example modules
cd examples/byte-counter
cargo build --target wasm32-wasip1 --release

cd ../zip-extractor
cargo build --target wasm32-wasip1 --release

# For sqlite-parser, use the build script
cd ../sqlite-parser
./build.sh

# For Go modules
cd ../go-sqlite-parser
make

# For C# modules
cd ../csharp-json-analyzer
make
```

### Testing

```bash
# Run the framework on test data
mkdir -p test-modules test-input

cp examples/byte-counter/target/wasm32-wasip1/release/byte_counter.wasm test-modules/
echo "Hello, WADUP!" > test-input/test.txt

./target/release/wadup \
  --modules test-modules \
  --input test-input \
  --output test.db

# Query results
sqlite3 test.db "SELECT * FROM file_sizes"

# Run integration tests
cargo test --release --test integration_tests
```

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]
