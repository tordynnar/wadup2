# WADUP - Web Assembly Data Unified Processing

A high-performance parallel processing framework that executes sandboxed WebAssembly modules on content, collecting metadata and extracting sub-content for recursive processing.

## Features

- **Parallel Processing**: Work-stealing threadpool for optimal CPU utilization
- **Module Reuse**: WASM modules loaded once at startup and reused across all files, eliminating per-file initialization overhead
- **Sandboxed Execution**: WASM modules run in isolated environments with configurable resource limits
- **Resource Control**: CPU (fuel), memory, stack size, and recursion depth limits
- **Metadata Collection**: SQLite database with automatic schema validation
- **Output Capture**: Module stdout/stderr captured and stored per-module in the database
- **Zero-Copy Architecture**: Memory-mapped file loading and SharedBuffer-based content slicing without duplication
- **Recursive Processing**: Sub-content automatically queued for processing
- **Ergonomic API**: Rust guest library for easy WASM module development

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/tordynnar/wadup2.git
cd wadup2

# Build everything (downloads dependencies, builds Python WASI, examples)
./scripts/build-all.sh

# Or just build the CLI
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

## Metadata Store

WADUP uses a SQLite database as its metadata store, with WAL mode enabled for concurrent access. The store manages two types of tables:

### System Tables

WADUP automatically creates two system tables:

#### `__wadup_content`

Tracks all processed content:

```sql
CREATE TABLE __wadup_content (
    uuid TEXT PRIMARY KEY,        -- Unique identifier for this content
    filename TEXT NOT NULL,       -- Original filename or extracted name
    parent_uuid TEXT,             -- Parent content UUID (NULL for top-level files)
    processed_at INTEGER NOT NULL, -- Unix timestamp of processing
    status TEXT NOT NULL,         -- 'success' or 'failed'
    error_message TEXT            -- Error details if status='failed'
);
```

The `parent_uuid` column enables tracking of content hierarchies. For example, when a ZIP extractor module emits files from an archive, those files reference the ZIP's UUID as their parent.

#### `__wadup_module_output`

Captures stdout and stderr from each module per content:

```sql
CREATE TABLE __wadup_module_output (
    content_uuid TEXT NOT NULL,      -- References __wadup_content(uuid)
    module_name TEXT NOT NULL,       -- Name of the WASM module
    stdout TEXT,                     -- Captured stdout (NULL if empty)
    stderr TEXT,                     -- Captured stderr (NULL if empty)
    stdout_truncated INTEGER NOT NULL, -- 1 if stdout exceeded 1MB limit
    stderr_truncated INTEGER NOT NULL, -- 1 if stderr exceeded 1MB limit
    PRIMARY KEY (content_uuid, module_name),
    FOREIGN KEY (content_uuid) REFERENCES __wadup_content(uuid)
);
```

This enables debugging and logging from modules. Output is captured silently (not echoed to terminal) and stored per-module. Each stream is limited to 1MB to prevent database bloat.

### Module-Defined Tables

Modules define custom tables to store extracted metadata. Each table automatically includes a `content_uuid` column as a foreign key:

```sql
-- Example: Module defines table "file_sizes" with column "size_bytes"
CREATE TABLE file_sizes (
    content_uuid TEXT NOT NULL,   -- Auto-added, references __wadup_content(uuid)
    size_bytes INTEGER,           -- Module-defined column
    FOREIGN KEY(content_uuid) REFERENCES __wadup_content(uuid)
);
```

### Data Types

Modules can use three data types for columns:

| Type | SQLite Type | Description |
|------|-------------|-------------|
| `Int64` | INTEGER | 64-bit signed integer |
| `Float64` | REAL | 64-bit floating point |
| `String` | TEXT | UTF-8 string |

### Schema Validation

When multiple modules or files define the same table, WADUP validates that schemas match exactly (same column names, same types, same order). Schema mismatches cause processing errors.

### Example Queries

```sql
-- List all processed files with status
SELECT filename, status, error_message FROM __wadup_content;

-- Find all content extracted from a specific file
SELECT c2.filename, c2.status
FROM __wadup_content c1
JOIN __wadup_content c2 ON c2.parent_uuid = c1.uuid
WHERE c1.filename = 'archive.zip';

-- Join module data with content info
SELECT c.filename, f.size_bytes
FROM file_sizes f
JOIN __wadup_content c ON c.uuid = f.content_uuid
WHERE c.status = 'success';

-- Trace content ancestry (recursive)
WITH RECURSIVE ancestry AS (
    SELECT uuid, filename, parent_uuid, 0 as depth
    FROM __wadup_content WHERE filename = 'nested/file.txt'
    UNION ALL
    SELECT c.uuid, c.filename, c.parent_uuid, a.depth + 1
    FROM __wadup_content c
    JOIN ancestry a ON c.uuid = a.parent_uuid
)
SELECT * FROM ancestry;

-- View module stdout/stderr for a file
SELECT c.filename, m.module_name, m.stdout, m.stderr
FROM __wadup_module_output m
JOIN __wadup_content c ON c.uuid = m.content_uuid
WHERE c.filename LIKE '%example.json%';

-- Find modules that produced errors (stderr)
SELECT DISTINCT module_name, COUNT(*) as error_count
FROM __wadup_module_output
WHERE stderr IS NOT NULL
GROUP BY module_name;
```

## Examples

See the `examples/` directory for working WASM modules:

**Rust Modules:**
- **byte-counter**: Counts and records file sizes
- **zip-extractor**: Extracts files from ZIP archives
- **sqlite-parser**: Parses SQLite databases using SQL queries

**Python Modules:**
- **python-sqlite-parser**: Parses SQLite databases using CPython 3.13.7
- **python-counter**: Demonstrates module reuse with global state
- **python-module-test**: Tests C extension imports (sqlite3, json, etc.)
- **python-multi-file**: Multi-file project with third-party dependencies (chardet, humanize, python-slugify)
- **python-numpy-test**: NumPy 2.4.0 array operations and linear algebra
- **python-pandas-test**: Pandas 2.3.3 DataFrame operations

**Go Modules:**
- **go-sqlite-parser**: Parses SQLite databases using pure Go SQLite library

All examples use the WASI target (`wasm32-wasip1`) to access the virtual filesystem.

### Building Examples

The easiest way to build all examples is:
```bash
./scripts/build-examples.sh
```

This builds all Rust, Python, and Go examples automatically.

**Individual Rust Examples** (byte-counter, zip-extractor, sqlite-parser):
```bash
cd examples/byte-counter
cargo build --target wasm32-wasip1 --release
```

See [examples/sqlite-parser/README.md](examples/sqlite-parser/README.md) for detailed documentation.

**Python Modules** (CPython 3.13.7):

Python modules use a standard `pyproject.toml` structure with pure-Python dependencies:

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
dependencies = ["chardet", "humanize"]  # pure-Python only

[tool.wadup]
entry-point = "python_counter"  # module with main() function
```

**Building Python modules:**

First, build the shared Python WASI runtime (one-time, ~5-10 minutes):
```bash
./scripts/build-python-wasi.sh
```

Then build individual Python modules using `build-python-project.py`:
```bash
./scripts/build-python-project.py examples/python-counter
./scripts/build-python-project.py examples/python-sqlite-parser
./scripts/build-python-project.py examples/python-multi-file
```

The build script:
1. Parses `pyproject.toml` for dependencies and entry point
2. Downloads pure-Python dependencies via `pip download --no-binary :all:`
3. Bundles project source, dependencies, and `wadup` library into a ZIP
4. Embeds the ZIP into a C file and compiles with CPython

**Third-party dependencies:**
- Pure-Python packages are fully supported
- Transitive dependencies are automatically resolved
- Dependencies are bundled into the WASM module

**Scientific Computing (NumPy & Pandas):**

NumPy 2.4.0 and Pandas 2.3.3 are supported as C extensions. To use them:

```toml
[tool.wadup]
entry-point = "my_module"
c-extensions = ["numpy"]           # NumPy only (~44 MB WASM)
# or
c-extensions = ["numpy", "pandas"] # NumPy + Pandas (~62 MB WASM)
```

NumPy provides array operations, linear algebra (`numpy.linalg`), and mathematical functions. Pandas provides DataFrames, Series, and data manipulation. Some features requiring OS-level support (random, fft, mmap) are not available.

For detailed build information and limitations:
- [NumPy WASI Build Guide](NUMPY_WASI.md)
- [Pandas WASI Build Guide](PANDAS_WASI.md)

The shared Python WASI build (`deps/wasi-python/`) includes:
- CPython 3.13.7 compiled for wasm32-wasip1
- SQLite 3.45.1 for WASI
- Frozen Python standard library (including logging, importlib, gettext, etc.)
- Compression libraries (zlib, bz2, lzma)

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

**Module Development (choose based on your language):**
- **Rust modules**: wasm32-wasip1 target (already installed above)
- **Python modules**: WASI SDK (auto-downloaded by build script)
- **Go modules**: Go 1.21+ (WASI support built-in, no extra tools needed)

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
