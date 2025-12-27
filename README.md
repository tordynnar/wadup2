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

WADUP modules are written in Rust and compiled to `wasm32-wasip1` (WASI target).

### Virtual Filesystem

Each WASM module runs in a sandboxed virtual filesystem where:
- **`/data.bin`** - The content being processed (read-only, zero-copy reference)
- **`/tmp/`** - Available for temporary files (read-write)

Modules can access content using standard file I/O operations. The `/data.bin` file is a zero-copy reference to the content data, implemented using `bytes::Bytes` for optimal memory efficiency.

### Example: File Size Counter

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
3. **File Processing** (runtime): The same module instances are reused to process all files assigned to that thread

**Key Benefits**:
- Module compilation happens once at startup, not per file
- WASM linear memory persists across files, allowing modules to maintain state if desired
- For Python modules using CPython, the interpreter is initialized once per thread and reused for all files
- Eliminates per-file initialization overhead (especially important for Python: ~20ms saved per file)

**Example**: Processing 1000 SQLite databases with the Python module:
- Without reuse: 1000 × 20ms = 20 seconds wasted on Python initialization
- With reuse: 1 × 20ms = 20ms total initialization (999× speedup)

This architecture makes WADUP suitable for batch processing large numbers of files efficiently.

### wadup-guest
Rust library for WASM module authors:
- **Content API**: Read content data and metadata
- **Table API**: Define schemas and insert rows
- **SubContent API**: Emit sub-content for recursive processing

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

- **byte-counter**: Counts and records file sizes (Rust)
- **zip-extractor**: Extracts files from ZIP archives (Rust)
- **sqlite-parser**: Parses SQLite databases using SQL queries (Rust)
- **python-sqlite-parser**: Parses SQLite databases using CPython 3.13.7 (Python)

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

**Python SQLite Parser** (CPython 3.13.7):
```bash
cd examples/python-sqlite-parser
./build.sh
```

This is a more complex build that:
- Downloads and compiles CPython 3.13.7 for WASI (~5-10 minute build)
- Builds SQLite 3.45.1 for WASI
- Freezes Python stdlib modules into the binary
- Creates a self-contained ~26MB WASM module

**Important**: The Python interpreter is initialized once per worker thread and reused across all files. Python global variables persist between files processed by the same thread. The module's `process()` function should be idempotent or explicitly reset state as needed.

See [examples/python-sqlite-parser/README.md](examples/python-sqlite-parser/README.md) for complete documentation, architecture details, and troubleshooting.

## Development

### Prerequisites

- Rust 1.70+
- wasm32-wasip1 target: `rustup target add wasm32-wasip1`
- WASI SDK (for modules with C dependencies): See [sqlite-parser README](examples/sqlite-parser/README.md)

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
