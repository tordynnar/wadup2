# WADUP - Web Assembly Data Unified Processing

A high-performance parallel processing framework that executes sandboxed WebAssembly modules on content, collecting metadata and extracting sub-content for recursive processing.

## Features

- **Parallel Processing**: Work-stealing threadpool for optimal CPU utilization
- **Sandboxed Execution**: WASM modules run in isolated environments with configurable resource limits
- **Resource Control**: CPU (fuel), memory, stack size, and recursion depth limits
- **Metadata Collection**: SQLite database with automatic schema validation
- **Zero-Copy Sub-Content**: Efficient content slicing without duplication
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

WADUP modules are written in Rust and compiled to `wasm32-unknown-unknown`.

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

    // Get content size and insert into database
    let size = Content::size() as i64;
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

# Build
cargo build --target wasm32-unknown-unknown --release
```

The compiled `.wasm` file can then be placed in your modules directory.

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

WADUP consists of four main crates:

### wadup-core
The processing engine containing:
- **Content Store**: Zero-copy content management with Arc-based slicing
- **WASM Runtime**: wasmtime integration with resource limits
- **Metadata Store**: SQLite with schema validation and WAL mode
- **Processor**: Work-stealing parallel execution

### wadup-bindings
Host/guest communication layer:
- **Host Functions**: FFI exports (define_table, insert_row, emit_subcontent, etc.)
- **Processing Context**: Shared state between host and WASM
- **Type Definitions**: Shared data structures

### wadup-guest
Rust library for WASM module authors:
- **Content API**: Read content data and metadata
- **Table API**: Define schemas and insert rows
- **SubContent API**: Emit sub-content for recursive processing

### wadup-cli
Command-line interface for running WADUP processing jobs.

## Guest API Reference

### Content Access

```rust
// Get content size
let size = Content::size();

// Read entire content
let data = Content::read_all()?;

// Read content as UTF-8 string
let text = Content::read_string()?;

// Read specific range
let chunk = Content::read(offset, length)?;

// Get content UUID
let uuid = Content::uuid()?;
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

- **byte-counter**: Counts and records file sizes
- **zip-extractor**: Extracts files from ZIP archives
- **sqlite-parser**: Parses SQLite databases using SQL queries (requires WASI)

### Building the SQLite Parser Example

The sqlite-parser example requires the WASI SDK to compile SQLite's C code to WebAssembly. Use the provided build script:

```bash
cd examples/sqlite-parser
./build.sh
```

The build script will automatically:
- Detect your platform
- Download WASI SDK if not present
- Build the module for wasm32-wasip1 target

See [examples/sqlite-parser/README.md](examples/sqlite-parser/README.md) for detailed documentation.

## Development

### Prerequisites

- Rust 1.70+
- wasm32-unknown-unknown target: `rustup target add wasm32-unknown-unknown`

### Building

```bash
# Build all crates
cargo build --release

# Build example modules
cd examples/byte-counter
cargo build --target wasm32-unknown-unknown --release

cd ../simple-test
cargo build --target wasm32-unknown-unknown --release
```

### Testing

```bash
# Run the framework on test data
mkdir -p test-modules test-input

cp examples/byte-counter/target/wasm32-unknown-unknown/release/byte_counter.wasm test-modules/
echo "Hello, WADUP!" > test-input/test.txt

./target/release/wadup \
  --modules test-modules \
  --input test-input \
  --output test.db

# Query results
sqlite3 test.db "SELECT * FROM file_sizes"
```

## Design Documents

- [DESIGN.md](DESIGN.md) - Original design specification
- [SYSTEM_DESIGN.md](SYSTEM_DESIGN.md) - Complete system architecture
- [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) - Phased implementation guide

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]
