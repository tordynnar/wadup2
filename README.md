# WADUP - Web Assembly Data Unified Processing

A high-performance parallel processing framework that executes sandboxed WebAssembly modules on content, collecting metadata and extracting sub-content for recursive processing.

## Features

- **Parallel Processing**: Work-stealing threadpool for optimal CPU utilization
- **Module Reuse**: WASM modules loaded once at startup and reused across all files, eliminating per-file initialization overhead
- **Sandboxed Execution**: WASM modules run in isolated environments with configurable resource limits
- **Resource Control**: CPU (fuel), memory, stack size, and recursion depth limits
- **Metadata Collection**: Elasticsearch for scalable, searchable metadata storage
- **Output Capture**: Module stdout/stderr captured and stored per-module in JSON documents
- **Zero-Copy Architecture**: Memory-mapped file loading and SharedBuffer-based content slicing without duplication
- **Recursive Processing**: Sub-content automatically queued for processing
- **Ergonomic API**: Rust guest library for easy WASM module development

## Project Structure

```
wadup2/
├── crates/              # Rust crates (workspace)
│   ├── wadup-core/      # Processing engine
│   ├── wadup-guest/     # Rust guest library for WASM modules
│   └── wadup-cli/       # Command-line interface
├── guest/               # Guest libraries for other languages
│   ├── python/          # Python wadup library
│   └── go/              # Go wadup library
├── docker/              # Docker build containers
│   ├── rust/            # Rust → wasm32-wasip1
│   ├── go/              # Go → wasip1
│   ├── python/          # Python → wasm32-wasi (CPython bundled)
│   └── test/            # WADUP test runner
├── examples/            # Example WASM modules (Rust, Python, Go)
├── scripts/             # Build and utility scripts
├── wadup-web/           # Web IDE for module development
├── demo/                # Demo data and modules
└── docker-compose.yml   # Elasticsearch + Kibana
```

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/tordynnar/wadup2.git
cd wadup2

# Build the CLI
cargo build --release
# The binary will be at target/release/wadup

# Build Docker images for module compilation (one-time setup)
./docker/build-images.sh

# Build all example modules
./scripts/build-examples.sh
```

### Basic Usage

```bash
# Start Elasticsearch and Kibana
docker-compose up -d

# Run WADUP
wadup run \
  --modules ./modules \
  --input ./data \
  --es-url http://localhost:9200 \
  --es-index wadup \
  --threads 8

# View results in Kibana
open http://localhost:5601
```

## Writing WASM Modules

WADUP modules can be written in **Rust**, **Python**, or **Go**, all compiled to the `wasm32-wasip1` (WASI) target.

### Virtual Filesystem

Each WASM module runs in a sandboxed virtual filesystem where:
- **`/data.bin`** - The content being processed (read-only, zero-copy reference)
- **`/tmp/`** - Available for temporary files (read-write)
- **`/metadata/`** - For file-based metadata output (all languages)
- **`/subcontent/`** - For file-based sub-content emission (all languages)

Modules can access content using standard file I/O operations. The `/data.bin` file is a zero-copy reference to the content data, implemented using `bytes::Bytes` for optimal memory efficiency.

### Language Support

WADUP supports three languages for writing modules:

| Language | Entry Point | Module Pattern | WASM Size | Build Time |
|----------|-------------|----------------|-----------|------------|
| **Rust** | `process()` | Reused | ~2.5 MB | ~30s |
| **Python** | `main()` | Reused | ~29 MB | ~5m (first) |
| **Go** | `process()` | Reused | ~8.3 MB | ~10s |

All languages use file-based metadata output (writing JSON to `/metadata/*.json`). Guest libraries handle serialization automatically. All modules must export a `process()` function and are reused across files (one instance processes all files per thread).

**Rust** modules export a `process()` function using `#[no_mangle] pub extern "C" fn process()`.

**Python** modules use embedded CPython 3.13.7 with a `main()` function entry point. The C glue layer exports `process()`. Supports pure-Python third-party dependencies and C extensions (NumPy, Pandas) bundled into the WASM module.

**Go** modules export a `process()` function using `//go:wasmexport process`.

See language-specific guides:
- [Rust Examples](examples/sqlite-parser/README.md)
- [Python Guide](examples/python-sqlite-parser/README.md)
- [Go Guide](examples/go-sqlite-parser/README.md)

### Module Interface: File-Based Communication

All WADUP modules use file-based communication to output metadata. Modules write JSON to special directories in the virtual filesystem:

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
- Uses standard file I/O (works with any WASM-compatible language)
- Incremental flushing supported (write multiple files during processing)
- Both metadata and sub-content supported via file-based interface
- **Zero-copy sub-content**: Data flows from WASM write → nested processing without copying

**Module Pattern:**
All WADUP modules use the reactor pattern - they export a `process()` function and are reused across files with minimal overhead.

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

### Example: File Analyzer (Python)

```python
# src/my_module/__init__.py
import wadup

def main():
    """Entry point called by WADUP for each file."""
    # Read input file from virtual filesystem
    with open('/data.bin', 'rb') as f:
        data = f.read()

    # Define metadata table
    wadup.define_table("file_stats", [
        ("size_bytes", "Int64"),
        ("line_count", "Int64"),
    ])

    # Insert row
    wadup.insert_row("file_stats", [
        len(data),
        data.count(b'\n'),
    ])

    # Flush metadata to output
    wadup.flush()
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
wadup run [OPTIONS]

Options:
  --modules <MODULES>
      Directory containing WASM modules

  --input <INPUT>
      Directory containing input files

  --es-url <ES_URL>
      Elasticsearch URL [default: http://localhost:9200]

  --es-index <ES_INDEX>
      Elasticsearch index name [default: wadup]

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
- **Metadata Store**: Elasticsearch client with flat document structure
- **Processor**: Work-stealing parallel execution
- **Host Bindings**: FFI exports for WASM modules (define_table, insert_row, emit_subcontent, etc.)

### Module Lifecycle and Performance

WADUP is designed for efficient processing of many files:

1. **Module Loading** (startup): All `.wasm` files are loaded from the modules directory and compiled once
2. **Instance Creation** (per thread): Each worker thread creates one instance of each module
3. **File Processing** (runtime): Same instance processes all files assigned to that thread (reactor pattern)

**Module Reuse Benefits**:
- Module compilation happens once at startup, not per file
- WASM linear memory persists across files, allowing modules to maintain state if desired
- For Python modules using CPython, the interpreter is initialized once per thread and reused for all files
- Eliminates per-file initialization overhead (especially important for Python: ~20ms saved per file)

**Example**: Processing 1000 SQLite databases with the Python module:
- Without reuse: 1000 × 20ms = 20 seconds wasted on Python initialization
- With reuse: 1 × 20ms = 20ms total initialization (999× speedup)

This architecture makes WADUP suitable for batch processing large numbers of files efficiently.

### Guest Libraries

Language-specific libraries for WASM module authors:

**wadup-guest** (Rust):
- File-based metadata output (writes JSON to `/metadata/*.json`)
- **Table API**: `TableBuilder::new("name").column(...).build()`
- **SubContent API**: `SubContent::emit_bytes()`, `SubContent::emit_slice()`
- Automatic flush on module completion

**guest/python** (Python):
- Pure-Python `wadup` library providing `wadup.define_table()`, `wadup.insert_row()`, and `wadup.flush()`
- File-based communication (writes JSON to `/metadata/*.json`)
- Bundled into WASM modules along with project source and dependencies
- Supports pure-Python third-party dependencies (e.g., `chardet`, `humanize`)

**guest/go** (Go):
- File-based metadata output (writes JSON to `/metadata/*.json`)
- Table builder API: `wadup.NewTableBuilder("name").Column(...).Build()`
- Value types: `wadup.NewInt64()`, `wadup.NewString()`, `wadup.NewFloat64()`

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

## Elasticsearch & Kibana

WADUP stores metadata in Elasticsearch using a flat document structure. Each processing run produces multiple documents linked by `content_uuid`:

- **Content documents**: One per processed file (metadata, status)
- **Module output documents**: One per module (stdout/stderr)
- **Row documents**: One per table row emitted by modules

### Starting the Services

```bash
# Start Elasticsearch and Kibana
docker-compose up -d

# Elasticsearch: http://localhost:9200
# Kibana:        http://localhost:5601

# Stop services
docker-compose down

# Stop and remove all data
docker-compose down -v
```

### Document Types

All documents share `content_uuid` and `processed_at` fields for joining and time filtering.

**1. Content Document** (`doc_type: "content"`):
```json
{
  "doc_type": "content",
  "content_uuid": "4757c08a-2ded-4637-b170-eae8f52fd3c4",
  "filename": "sample.db",
  "parent_uuid": null,
  "processed_at": "2024-01-03T12:00:00Z",
  "status": "success",
  "error_message": null
}
```

**2. Module Output Document** (`doc_type: "module_output"`):
```json
{
  "doc_type": "module_output",
  "content_uuid": "4757c08a-2ded-4637-b170-eae8f52fd3c4",
  "module_name": "sqlite_parser",
  "processed_at": "2024-01-03T12:00:00Z",
  "stdout": "Parsed 3 tables",
  "stderr": null,
  "stdout_truncated": false,
  "stderr_truncated": false
}
```

**3. Row Document** (`doc_type: "row"`):
```json
{
  "doc_type": "row",
  "content_uuid": "4757c08a-2ded-4637-b170-eae8f52fd3c4",
  "_module": "sqlite_parser",
  "_table": "db_table_stats",
  "processed_at": "2024-01-03T12:00:00Z",
  "table_name": "users",
  "row_count": "100"
}
```

Key fields:
- **doc_type**: Document type (`"content"`, `"module_output"`, or `"row"`)
- **content_uuid**: Links all documents from the same content
- **processed_at**: Timestamp for time-based filtering in Kibana
- **_module**: Module that emitted this row (underscore prefix avoids conflicts)
- **_table**: Table name (underscore prefix avoids conflicts)
- Column values are flattened as key-value pairs (e.g., `table_name`, `row_count`)

### Using Kibana

1. Open http://localhost:5601
2. Go to **Management > Stack Management > Data Views**
3. Click **Create data view**
4. Enter `wadup*` as the index pattern
5. Select `processed_at` as the **Timestamp field**
6. Click **Save data view to Kibana**

#### Discover (Search & Browse)

1. Go to **Analytics > Discover**
2. Select your `wadup*` data view
3. Use the search bar for queries:

```
# Find all content documents
doc_type: "content"

# Find successful content
doc_type: "content" AND status: "success"

# Find content by filename
filename: "sample.db"

# Find all row documents for a table
doc_type: "row" AND _table: "db_table_stats"

# Find rows from a specific module
doc_type: "row" AND _module: "sqlite_parser"

# Find failed content
doc_type: "content" AND status: "failed"

# Find sub-content (has parent)
doc_type: "content" AND parent_uuid: *
```

#### Dev Tools (Raw Queries)

Go to **Management > Dev Tools** for direct Elasticsearch queries:

```json
# List all content documents
GET wadup/_search
{
  "query": { "term": { "doc_type": "content" } },
  "size": 100
}

# Find content by filename
GET wadup/_search
{
  "query": {
    "bool": {
      "must": [
        { "term": { "doc_type": "content" } },
        { "match": { "filename": "sample.db" } }
      ]
    }
  }
}

# Find all rows for a table
GET wadup/_search
{
  "query": {
    "bool": {
      "must": [
        { "term": { "doc_type": "row" } },
        { "term": { "_table": "db_table_stats" } }
      ]
    }
  }
}

# Find all documents for a specific content
GET wadup/_search
{
  "query": {
    "term": { "content_uuid": "4757c08a-2ded-4637-b170-eae8f52fd3c4" }
  }
}

# Count rows by table
GET wadup/_search
{
  "query": { "term": { "doc_type": "row" } },
  "size": 0,
  "aggs": {
    "by_table": {
      "terms": { "field": "_table.keyword" }
    }
  }
}

# Count content by status
GET wadup/_search
{
  "query": { "term": { "doc_type": "content" } },
  "size": 0,
  "aggs": {
    "by_status": {
      "terms": { "field": "status.keyword" }
    }
  }
}
```

### Command-Line Queries

Query directly with curl:

```bash
# Count all documents
curl -s "http://localhost:9200/wadup/_count" | jq

# Count content documents only
curl -s "http://localhost:9200/wadup/_count" -H "Content-Type: application/json" -d '
{"query": {"term": {"doc_type": "content"}}}'

# Find all rows for a table
curl -s "http://localhost:9200/wadup/_search?pretty" -H "Content-Type: application/json" -d '
{
  "query": {
    "bool": {
      "must": [
        { "term": { "doc_type": "row" } },
        { "term": { "_table": "db_table_stats" } }
      ]
    }
  }
}'

# Find content by filename
curl -s "http://localhost:9200/wadup/_search?pretty" -H "Content-Type: application/json" -d '
{
  "query": {
    "bool": {
      "must": [
        { "term": { "doc_type": "content" } },
        { "match": { "filename": "sample.db" } }
      ]
    }
  }
}'
```

### Data Types

Modules can use three data types for table columns. All values are stored as strings in Elasticsearch to avoid mapping conflicts:

| Type | Description | Example |
|------|-------------|---------|
| `Int64` | 64-bit signed integer | `"42"` |
| `Float64` | 64-bit floating point | `"3.14"` |
| `String` | UTF-8 string | `"hello"` |

## Examples

See the `examples/` directory for working WASM modules:

**Rust Modules:**
- **byte-counter**: Counts and records file sizes
- **zip-extractor**: Extracts files from ZIP archives
- **sqlite-parser**: Parses SQLite databases using SQL queries
- **simple-test**: Basic module for testing the framework

**Python Modules:**
- **python-sqlite-parser**: Parses SQLite databases using CPython 3.13.7
- **python-counter**: Demonstrates module reuse with global state
- **python-module-test**: Tests C extension imports (sqlite3, json, etc.)
- **python-multi-file**: Multi-file project with third-party dependencies (chardet, humanize, python-slugify)
- **python-pydantic-test**: Tests Pydantic data validation
- **python-lxml-test**: Tests lxml XML/HTML parsing
- **python-large-file-test**: Tests large if/elif chain handling

**Go Modules:**
- **go-sqlite-parser**: Parses SQLite databases using pure Go SQLite library

All examples use the WASI target (`wasm32-wasip1`) to access the virtual filesystem.

### Building Examples

All module builds use Docker containers, which include all required toolchains and dependencies.

**Prerequisites:**
```bash
# Build Docker images (one-time setup)
./docker/build-images.sh
```

This creates four images:
- `wadup-build-rust:latest` - Rust compiler with wasm32-wasip1 target
- `wadup-build-go:latest` - Go compiler with WASI support
- `wadup-build-python:latest` - CPython 3.13 + WASI SDK + pre-built C extensions (lxml, pydantic)
- `wadup-test-runner:latest` - WADUP runtime for testing modules

**Build all examples:**
```bash
./scripts/build-examples.sh
```

This builds all Rust, Python, and Go examples automatically using Docker.

**Individual Rust Examples** (byte-counter, zip-extractor, sqlite-parser):
```bash
cd examples/byte-counter
cargo build --target wasm32-wasip1 --release
```

See [examples/sqlite-parser/README.md](examples/sqlite-parser/README.md) for detailed documentation.

**Python Modules** (CPython 3.13):

Python modules use a standard `pyproject.toml` structure:

```
examples/python-counter/
├── pyproject.toml
└── src/
    └── python_counter/
        └── __init__.py   # contains main() entry point
```

**pyproject.toml format:**
```toml
[project]
name = "python-counter"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = ["chardet", "humanize"]  # pure-Python deps supported

[tool.wadup]
entry-point = "python_counter"  # module with main() function
```

**Building Python modules:**

Python modules are built using Docker, which includes all dependencies pre-built:
```bash
./scripts/build-examples.sh  # Builds all examples including Python
```

The Docker build process:
1. Parses `pyproject.toml` for dependencies and entry point
2. Installs pure-Python dependencies via pip
3. Bundles project source, dependencies, and `wadup` library into a ZIP
4. Pre-compiles all `.py` files to `.pyc` for faster startup
5. Embeds the ZIP into a C file and compiles with CPython + WASI SDK

**Third-party dependencies:**
- Pure-Python packages are fully supported (e.g., `chardet`, `humanize`, `python-slugify`)
- Transitive dependencies are automatically resolved
- C extensions: `lxml` and `pydantic` are pre-built in the Docker image
- Dependencies are bundled into the WASM module

**Important**: The Python interpreter is initialized once per worker thread and reused across all files. Python global variables persist between files processed by the same thread. The module's `main()` function should be idempotent or explicitly reset state as needed.

See [examples/python-sqlite-parser/README.md](examples/python-sqlite-parser/README.md) for complete documentation.

**Go Modules** (Standard Go 1.21+):

```bash
cd examples/go-sqlite-parser
GOOS=wasip1 GOARCH=wasm go build -o target/go_sqlite_parser.wasm .
```

Go modules use standard Go (not TinyGo) with `GOOS=wasip1 GOARCH=wasm` target. No special setup required - standard Go has built-in WASI support!

**Key Features**:
- Pure Go libraries work (e.g., `github.com/ncruces/go-sqlite3`)
- `process()` export via `//go:wasmexport` for reactor pattern
- Fast build times (~10 seconds)
- Moderate WASM size (~8.3 MB)

See [examples/go-sqlite-parser/README.md](examples/go-sqlite-parser/README.md) for complete guide, best practices, and what works/doesn't work with Go+WASM.

## Development

### Prerequisites

**Core Framework:**
- Rust 1.70+
- wasm32-wasip1 target: `rustup target add wasm32-wasip1`

**Module Development:**
- **Docker**: All module builds use Docker containers
- Build images once: `./docker/build-images.sh`

Alternatively, for local Rust development without Docker:
- wasm32-wasip1 target: `rustup target add wasm32-wasip1`

### Building

```bash
# Build all crates
cargo build --release

# Build example modules
cd examples/byte-counter
cargo build --target wasm32-wasip1 --release

cd ../zip-extractor
cargo build --target wasm32-wasip1 --release

# Or build all examples at once
./scripts/build-examples.sh
```

### Testing

```bash
# Start Elasticsearch
docker-compose up -d elasticsearch

# Run the framework on test data
mkdir -p test-modules test-input

cp examples/byte-counter/target/wasm32-wasip1/release/byte_counter.wasm test-modules/
echo "Hello, WADUP!" > test-input/test.txt

./target/release/wadup run \
  --modules test-modules \
  --input test-input \
  --es-index test

# Query results
curl -s "http://localhost:9200/test/_search?pretty"

# Run integration tests
./scripts/run-integration-tests.sh
```

## WADUP Web

WADUP Web is a browser-based IDE for developing, building, testing, and publishing WADUP modules. It provides a VS Code-like experience with:

- **Monaco Editor** with syntax highlighting (Catppuccin Macchiato theme)
- **Multi-language support**: Create Rust, Go, or Python modules
- **File management**: Tree view with drag-and-drop, rename, create, delete
- **Docker-based builds**: Compile modules to WebAssembly with real-time log streaming
- **Test samples**: Upload files to test modules against
- **Module publishing**: Draft/published version management

See [wadup-web/README.md](wadup-web/README.md) for setup and usage instructions.

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]
