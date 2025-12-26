# WADUP System Design

## Overview

Web Assembly Data Unified Processing (WADUP) is a framework for extracting sub-content and metadata from content using user-provided WASM modules in a sandboxed, parallel processing environment.

## Design Decisions

Based on requirements analysis, the following key decisions were made:

1. **Module Execution Model**: Every loaded WASM module processes every piece of content. Modules internally decide whether to handle specific content.

2. **Error Handling**: If a WASM module crashes, panics, or times out, skip that content, log the error, mark it as failed in metadata, and continue processing other content.

3. **Table Schema Conflicts**: Allow modules to share tables if schemas match exactly. Validate column names, types, and order. Error if schemas differ.

4. **Concurrency Model**: Parallel processing with work-stealing and user-specified thread count. Sequential processing is supported by setting threads=1.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Entry Point                         │
│  (Arguments: --modules, --input, --threads, --fuel,          │
│   --max-memory, --max-stack, --max-recursion-depth)          │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│                  Content Processor                           │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Module       │  │  Metadata    │  │   Work Queue    │  │
│  │ Loader       │  │  Store       │  │  (Work-Stealing)│  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┴──────────────┐
        │                             │
┌───────▼────────┐          ┌─────────▼─────────┐
│ Worker Thread 1│   ...    │ Worker Thread N   │
│ ┌────────────┐ │          │ ┌────────────┐   │
│ │WASM Module │ │          │ │WASM Module │   │
│ │Instance 1  │ │          │ │Instance 1  │   │
│ │Instance 2  │ │          │ │Instance 2  │   │
│ │...         │ │          │ │...         │   │
│ └────────────┘ │          │ └────────────┘   │
└────────┬───────┘          └─────────┬─────────┘
         │                            │
         └────────────┬───────────────┘
                      │
            ┌─────────▼──────────┐
            │  SQLite Database   │
            │  - User Tables     │
            │  - Metadata Table  │
            └────────────────────┘
```

### Core Components

#### 1. Processor Core (`wadup-core` crate)

- **ContentProcessor**: Main orchestrator that coordinates all processing
- **ContentStore**: Thread-safe in-memory content storage with UUID indexing
- **WasmRuntime**: Manages wasmtime Engine and per-thread Module instances
- **ModuleRegistry**: Registry of loaded WASM modules and their metadata
- **MetadataStore**: SQLite database wrapper with schema validation
- **WorkQueue**: Lock-free work-stealing deque for parallel processing

#### 2. WASM Bindings (`wadup-bindings` crate)

Host functions exposed to WASM modules:
- `define_table(name, columns)` → table_id
- `insert_row(table_id, values)` → result
- `emit_subcontent_bytes(data, filename)` → result
- `emit_subcontent_slice(offset, length, filename)` → result
- `get_content_size()` → usize
- `read_content(offset, length, dest_ptr)` → result
- `get_content_uuid(dest_ptr)` → result

#### 3. Guest Library (`wadup-guest` crate)

Rust library for WASM module authors:
- Type-safe wrappers around raw WASM imports
- Builder patterns for ergonomic API
- Memory management helpers
- Compiles to `wasm32-unknown-unknown` target (pure WASM, no WASI)

#### 4. CLI (`wadup-cli` crate)

- Argument parsing using clap
- Resource limit configuration (fuel, memory, stack) applied to all modules
- Progress reporting
- Error formatting and logging configuration

## Detailed Component Design

### Content Lifecycle

```
File → Content{uuid} → [Module1, Module2, ...] → SubContent* → Queue
                                                → Metadata → SQLite
```

#### Content Structure

```rust
struct Content {
    uuid: Uuid,
    data: ContentData,
    filename: String,
    parent_uuid: Option<Uuid>,
}

enum ContentData {
    Owned(Vec<u8>),           // Root content from file or emitted bytes
    Borrowed {                // Sub-content via offset/length
        parent_uuid: Uuid,
        offset: usize,
        length: usize,
    },
}
```

**Key Design**: Zero-copy sub-content references parent data. Parent Content must stay alive until all descendants are processed (depth-first processing guarantees this).

### Threading & Concurrency Model

#### Work-Stealing Queue

- Each thread has a local deque (using `crossbeam-deque`)
- Threads push new sub-content to their own deque (LIFO for depth-first)
- When a thread's deque is empty, it steals from another thread (FIFO)
- Lock-free implementation for optimal performance

#### Thread Workflow

```
1. Pop content from local deque (or steal from others)
2. For each loaded WASM module:
   a. Get or create per-thread instance
   b. Set up processing context (content, collectors)
   c. Call module's process() function
   d. Collect emitted sub-content and metadata
3. Push sub-content to local deque (depth-first)
4. Write metadata to SQLite (with synchronization)
5. Update __wadup_content table
6. Repeat until all queues empty
```

#### WASM Instance Management

- Each thread maintains its own instance of each WASM module
- Instances are reused across content items (state reset between calls)
- No shared WASM memory between threads
- Per-thread `Store<T>` context for wasmtime

### Metadata Store & Schema Validation

#### Built-in Metadata Table

```sql
CREATE TABLE __wadup_content (
    uuid TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    parent_uuid TEXT,
    processed_at INTEGER NOT NULL,
    status TEXT NOT NULL,  -- 'success' or 'failed'
    error_message TEXT
);
```

#### Schema Validation

```rust
struct TableSchema {
    name: String,
    columns: Vec<Column>,
}

struct Column {
    name: String,
    data_type: DataType,
}

enum DataType {
    Int64,
    Float64,
    String,
}
```

**Validation Rules**:
When a module calls `define_table()`:
1. Check if table exists in SQLite
2. If not, create it and store schema in registry
3. If yes, validate exact match: column names, types, and order
4. Reject with error if mismatch, return error to WASM module

**SQLite Access**:
- Use Write-Ahead Logging (WAL) mode for better concurrency
- Shared connection with synchronization (mutex or channel)
- Batch inserts where possible to reduce lock contention
- Consider dedicated writer thread if profiling shows bottleneck

#### UUID Tracking

All user-defined tables automatically get a `content_uuid TEXT` column to track which content generated each metadata row.

### WASM Runtime Configuration

The WASM runtime is configured with resource limits that apply uniformly to all loaded modules. These limits are specified via CLI arguments and enforced by wasmtime:

#### Fuel Limit (CPU Usage)

- **Purpose**: Limit CPU/computation time per module per content item
- **Mechanism**: Wasmtime's fuel system (configurable via `Config::consume_fuel(true)`)
- **Configuration**: `--fuel <amount>` CLI argument (e.g., `--fuel 10000000`)
- **Behavior**: Each module instance gets the specified amount of fuel before processing each content item
- **On Exhaustion**: Trap is raised, caught by error handler, content marked as failed
- **Default**: If not specified, fuel consumption is disabled (no CPU limit)

#### Memory Limit

- **Purpose**: Limit maximum linear memory size per module instance
- **Mechanism**: Wasmtime's `Config::max_wasm_stack()` for stack, memory limits via module instantiation
- **Configuration**: `--max-memory <bytes>` CLI argument (e.g., `--max-memory 67108864` for 64MB)
- **Behavior**: Each module instance is limited to the specified maximum memory
- **On Exhaustion**: Allocation fails within WASM, module should handle gracefully or trap
- **Default**: If not specified, uses wasmtime defaults

#### Stack Size Limit

- **Purpose**: Limit maximum call stack depth to prevent stack overflow
- **Mechanism**: Wasmtime's `Config::max_wasm_stack()`
- **Configuration**: `--max-stack <bytes>` CLI argument (e.g., `--max-stack 1048576` for 1MB)
- **Behavior**: Limits the maximum stack size for each module instance
- **On Exhaustion**: Trap is raised on stack overflow
- **Default**: If not specified, uses wasmtime defaults (typically 1MB)

**Resource Allocation Model**: Each module instance receives its own allocation of these resources. With N threads and M modules, there are N×M instances, each with independent fuel/memory/stack allocations. Resources are reset between content items (fuel is replenished for each new content).

### WASM Interface Specification

#### Module Entry Point

```rust
// Guest side (in WASM module)
#[no_mangle]
pub extern "C" fn process() -> i32 {
    // Return 0 for success, non-zero for error
}
```

#### Host Functions (imported by WASM)

```rust
// Table management
#[no_mangle]
fn define_table(
    name_ptr: i32,
    name_len: i32,
    columns_ptr: i32,
    columns_len: i32
) -> i32;

// Row insertion
#[no_mangle]
fn insert_row(
    table_id: i32,
    row_data_ptr: i32,
    row_data_len: i32
) -> i32;

// Sub-content emission
#[no_mangle]
fn emit_subcontent_bytes(
    data_ptr: i32,
    data_len: i32,
    filename_ptr: i32,
    filename_len: i32
) -> i32;

#[no_mangle]
fn emit_subcontent_slice(
    offset: i32,
    length: i32,
    filename_ptr: i32,
    filename_len: i32
) -> i32;

// Content access
#[no_mangle]
fn get_content_size() -> i32;

#[no_mangle]
fn read_content(
    offset: i32,
    length: i32,
    dest_ptr: i32
) -> i32;

// Helper for current content UUID
#[no_mangle]
fn get_content_uuid(dest_ptr: i32) -> i32;  // Write 16 bytes
```

#### Data Serialization

- Pass strings as (ptr, len) pairs pointing to WASM linear memory
- Complex structures (columns, row data) use JSON serialization for simplicity
- Binary format could be considered for performance optimization later

### Error Handling Strategy

#### Error Types

1. **WASM Module Errors**: Trap, panic, fuel exhaustion, stack overflow
2. **Schema Errors**: Table definition mismatch
3. **I/O Errors**: File read, SQLite write
4. **Memory Errors**: Content too large, allocation failure, WASM memory limit exceeded
5. **Resource Errors**: Recursion depth exceeded, content bomb

#### Handling Strategy

- Wrap all WASM calls in wasmtime's trap handling
- Catch fuel exhaustion traps when fuel limits are enabled
- Catch stack overflow and memory allocation traps
- Log errors with full context (content UUID, module name, error message)
- Write failed content to `__wadup_content` table with status='failed'
- Continue processing other content (graceful degradation)

#### Resource Limits

To prevent resource exhaustion attacks (e.g., ZIP bombs):
- **Recursion depth limit** per content chain (CLI: `--max-recursion-depth`, default: 100)
- **Fuel limit** (CPU usage) per module per content (CLI: `--fuel`, optional)
- **Memory limit** per module instance (CLI: `--max-memory`, optional)
- **Stack size limit** per module instance (CLI: `--max-stack`, optional, default: wasmtime default)
- **Maximum sub-content count** per parent (configurable in code)
- **Maximum total content** in memory (configurable in code)

#### Logging

Use `tracing` crate for structured logging:
- **ERROR**: Processing failures, module crashes
- **WARN**: Schema mismatches, resource limit hits
- **INFO**: Progress updates, module loading
- **DEBUG**: Detailed operation traces

## Data Flow Example

```
┌──────────────┐
│ Input Files  │
│  file1.zip   │
│  file2.db    │
└──────┬───────┘
       │ Read & Create Content{uuid1}, Content{uuid2}
       ▼
┌──────────────────┐
│  Initial Queue   │
│  [uuid1, uuid2]  │
└──────┬───────────┘
       │ Work-Stealing Distribution
       ▼
┌────────────────────────────┐
│  Worker Thread 1           │
│  Pop uuid1                 │
│  ┌──────────────────────┐  │
│  │ Module A: ZIP Parser │  │
│  │ → emit uuid3, uuid4  │  │
│  │ → metadata: none     │  │
│  └──────────────────────┘  │
│  ┌──────────────────────┐  │
│  │ Module B: Byte Count │  │
│  │ → metadata: size     │  │
│  └──────────────────────┘  │
│  Push uuid3, uuid4         │
└────────┬───────────────────┘
         │
         ▼ Depth-first: Process uuid3
┌────────────────────────────┐
│  Worker Thread 1           │
│  Pop uuid3                 │
│  ┌──────────────────────┐  │
│  │ Module A: (no match) │  │
│  └──────────────────────┘  │
│  ┌──────────────────────┐  │
│  │ Module B: Byte Count │  │
│  │ → metadata: size     │  │
│  └──────────────────────┘  │
└────────┬───────────────────┘
         │
         ▼ Continue until queue empty

┌────────────────────────────┐
│  SQLite Database           │
│  __wadup_content:          │
│    uuid1, file1.zip, NULL  │
│    uuid2, file2.db, NULL   │
│    uuid3, file.txt, uuid1  │
│    uuid4, file2.txt, uuid1 │
│                            │
│  file_sizes:               │
│    content_uuid, size      │
│    uuid1, 1024             │
│    uuid3, 512              │
│    uuid4, 256              │
└────────────────────────────┘
```

## Technical Challenges & Solutions

### Challenge 1: Zero-Copy Sub-Content

**Problem**: Sub-content via offset/length must reference parent data, but parent might be deallocated.

**Solution**:
- Store all owned content in `Arc<Vec<u8>>` in ContentStore
- `ContentData::Borrowed` holds parent UUID for lookup
- Depth-first processing ensures parent outlives children
- Reference counting prevents premature deallocation
- ContentStore maintains `Arc` clones until processing complete

### Challenge 2: WASM Memory Access

**Problem**: Passing large content to WASM is expensive (requires copying to linear memory).

**Solution**:
- Provide `read_content(offset, length)` for streaming access
- WASM module allocates buffer in its linear memory
- Module requests chunks as needed
- Avoid full copy for large files
- For small content (<1MB), full copy is acceptable

### Challenge 3: SQLite Concurrency

**Problem**: SQLite has limited write concurrency, could become bottleneck.

**Solution**:
- Use WAL (Write-Ahead Logging) mode for better concurrency
- Batch inserts per thread (reduce transaction overhead)
- Consider dedicated writer thread with channel if needed
- Profile to verify if actually a bottleneck
- Most workloads are processing-bound, not DB-bound

### Challenge 4: Schema Validation Complexity

**Problem**: Multiple modules might define same table independently, need to detect conflicts.

**Solution**:
- Maintain in-memory schema registry (table name → schema)
- On first `define_table`, create table in SQLite and cache schema
- On subsequent calls, validate exact match before allowing
- Reject and return clear error if mismatch
- Error message includes: which modules, what schemas, what differs

### Challenge 5: Work-Stealing Depth-First

**Problem**: Coordinating depth-first processing with work-stealing is non-trivial.

**Solution**:
- Each thread has LIFO deque (push/pop from same end = stack)
- Local push/pop maintains depth-first within thread
- Other threads steal from opposite end (FIFO = oldest work)
- `crossbeam-deque::Worker` and `Stealer` implement this correctly
- Natural load balancing while preserving depth-first locality

### Challenge 6: Resource Bombs

**Problem**: Malicious or buggy WASM module could create infinite sub-content (e.g., ZIP bomb, decompression bomb) or consume excessive resources.

**Solution**:
- **Track recursion depth** in Content struct (parent chain length)
- **Limit max depth** via CLI: `--max-recursion-depth` (default: 100)
- **Limit total sub-content count** per parent (e.g., 10,000)
- **Fuel limits** (CLI: `--fuel`) to cap CPU usage per module per content
- **WASM memory limits** (CLI: `--max-memory`) enforced by wasmtime per module instance
- **Stack size limits** (CLI: `--max-stack`) to prevent deep call stacks
- **Monitor total system memory** usage across all instances

## Project Structure

```
wadup/
├── Cargo.toml                 # Workspace definition
├── README.md
├── DESIGN.md                  # Original design document
├── SYSTEM_DESIGN.md           # This file
├── IMPLEMENTATION_PLAN.md     # Phased implementation plan
│
├── crates/
│   ├── wadup-core/           # Core processor
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── content.rs    # Content, ContentData, ContentStore
│   │       ├── processor.rs  # ContentProcessor, orchestration
│   │       ├── wasm.rs       # WasmRuntime, ModuleRegistry
│   │       ├── metadata.rs   # MetadataStore, schema validation
│   │       ├── queue.rs      # WorkQueue, work-stealing
│   │       └── error.rs      # Error types
│   │
│   ├── wadup-bindings/       # WASM interface
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── host.rs       # Host function implementations
│   │       ├── context.rs    # Processing context for WASM calls
│   │       └── types.rs      # Shared types (DataType, etc.)
│   │
│   ├── wadup-guest/          # Guest library for module authors
│   │   ├── Cargo.toml        # target: wasm32-wasi
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── table.rs      # Table API
│   │       ├── content.rs    # Content access API
│   │       └── subcontent.rs # Sub-content emission API
│   │
│   └── wadup-cli/            # CLI application
│       ├── Cargo.toml
│       └── src/
│           └── main.rs       # CLI entry point
│
├── examples/                  # Example WASM modules
│   ├── sqlite-parser/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── zip-parser/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── byte-counter/
│       ├── Cargo.toml
│       └── src/lib.rs
│
└── tests/                     # Integration tests
    ├── fixtures/              # Test data
    │   ├── sample.db
    │   └── sample.zip
    ├── test1_sqlite.rs
    ├── test2_zip.rs
    └── test3_combined.rs
```

## Performance Considerations

### Expected Bottlenecks

1. **WASM Module Execution**: Most time spent in user code
2. **File I/O**: Reading input files (mitigated by in-memory processing)
3. **SQLite Writes**: Batch inserts help significantly
4. **Memory Copies**: Zero-copy design minimizes this

### Optimization Strategies

1. **Parallelism**: Work-stealing across N threads
2. **Zero-Copy**: Borrowed sub-content via offset/length
3. **Batch Operations**: Group SQLite inserts
4. **Instance Reuse**: Avoid repeated WASM instantiation
5. **Memory Pooling**: Reuse allocations where possible

### Profiling Plan

- Use `cargo flamegraph` for CPU profiling
- Identify hot paths in WASM execution
- Measure SQLite transaction overhead
- Monitor memory allocation patterns
- Test with realistic workloads (large files, many modules)

## Security Considerations

### Sandbox Model

- WASM provides memory isolation by default
- Pure WASM with no WASI support - no access to filesystem, network, or system calls
- Host functions are the only interface to the outside world
- Validated and controlled through Rust type system

### Resource Limits

Comprehensive resource limits prevent denial-of-service attacks:

- **Fuel Limits**: Configurable CPU/computation limits via `--fuel` prevent infinite loops and excessive computation
- **Memory Limits**: Configurable per-module memory caps via `--max-memory` prevent excessive allocation
- **Stack Limits**: Configurable stack size via `--max-stack` prevents stack overflow attacks
- **Recursion Depth**: Configurable limit on content nesting depth via `--max-recursion-depth` (default: 100) prevents recursive content bombs
- **WASM limits enforced by wasmtime**: Native enforcement with zero overhead when not triggered
- **Recursion limit enforced by processor**: Checked before creating sub-content

### Input Validation

- Validate all data from WASM modules (offsets, lengths, table names)
- Sanitize SQL table/column names (prevent injection)
- Verify content references before dereferencing

## Open Questions

1. **Streaming Support**: Should we support content too large to fit in memory?
2. **Module Ordering**: Should we allow custom module execution order/priority?
3. **Built-in Modules**: Should we provide standard parsers out-of-the-box?
4. **Performance Target**: What throughput should we aim for? (e.g., 100 MB/s)

## Future Enhancements

- Module marketplace/registry
- Web-based GUI for exploring metadata
- Incremental processing (only process new/changed files)
- Distributed processing across multiple machines
- Additional guest libraries (Python, JavaScript via componentization)
- True streaming mode for files larger than RAM
- Alternative output formats (PostgreSQL, Parquet, JSON)
- Real-time processing mode (watch directories for changes)
- Module composition (pipe outputs between modules)
