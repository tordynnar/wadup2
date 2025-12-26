# WADUP Implementation Plan

This document outlines a phased approach to implementing the WADUP framework. Each phase builds on the previous one and includes clear deliverables.

## Overview

The implementation is divided into 9 phases:

1. Foundation & Project Setup
2. WASM Runtime Integration
3. Host Functions Implementation
4. Content Processing Engine
5. Metadata Management
6. Error Handling & Resilience
7. Rust Guest Library
8. Integration Tests
9. CLI & Polish

## Phase 1: Foundation & Project Setup

**Goal**: Set up project structure, dependencies, and basic scaffolding

**Duration Estimate**: Foundation work

### Tasks

1. **Create Cargo Workspace**
   - Create root `Cargo.toml` with workspace definition
   - Create 4 member crates:
     - `crates/wadup-core`
     - `crates/wadup-bindings`
     - `crates/wadup-guest`
     - `crates/wadup-cli`
   - Set up appropriate `Cargo.toml` for each crate

2. **Add Dependencies**

   For `wadup-core`:
   ```toml
   wasmtime = "26"
   rusqlite = { version = "0.32", features = ["bundled"] }
   uuid = { version = "1.11", features = ["v4"] }
   crossbeam = "0.8"
   tracing = "0.1"
   anyhow = "1.0"
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```

   For `wadup-cli`:
   ```toml
   clap = { version = "4.5", features = ["derive"] }
   tracing-subscriber = "0.3"
   ```

   For `wadup-guest`:
   ```toml
   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```

3. **Define Core Data Structures**

   Create `crates/wadup-core/src/content.rs`:
   ```rust
   use uuid::Uuid;
   use std::sync::Arc;

   pub struct Content {
       pub uuid: Uuid,
       pub data: ContentData,
       pub filename: String,
       pub parent_uuid: Option<Uuid>,
       pub depth: usize,  // Recursion depth (0 for root content)
   }

   pub enum ContentData {
       Owned(Arc<Vec<u8>>),
       Borrowed {
           parent_uuid: Uuid,
           offset: usize,
           length: usize,
       },
   }
   ```

   Create `crates/wadup-bindings/src/types.rs`:
   ```rust
   use serde::{Deserialize, Serialize};

   #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
   pub enum DataType {
       Int64,
       Float64,
       String,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Column {
       pub name: String,
       pub data_type: DataType,
   }

   #[derive(Debug, Clone)]
   pub struct TableSchema {
       pub name: String,
       pub columns: Vec<Column>,
   }
   ```

4. **Implement UUID Generation**

   Add helper in `content.rs`:
   ```rust
   impl Content {
       pub fn new_root(data: Vec<u8>, filename: String) -> Self {
           Self {
               uuid: Uuid::new_v4(),
               data: ContentData::Owned(Arc::new(data)),
               filename,
               parent_uuid: None,
               depth: 0,  // Root content has depth 0
           }
       }
   }
   ```

5. **Implement SQLite Initialization**

   Create `crates/wadup-core/src/metadata.rs`:
   ```rust
   use rusqlite::Connection;
   use anyhow::Result;

   pub struct MetadataStore {
       conn: Connection,
   }

   impl MetadataStore {
       pub fn new(path: &str) -> Result<Self> {
           let conn = Connection::open(path)?;
           Self::init_tables(&conn)?;
           Ok(Self { conn })
       }

       fn init_tables(conn: &Connection) -> Result<()> {
           conn.execute(
               "CREATE TABLE IF NOT EXISTS __wadup_content (
                   uuid TEXT PRIMARY KEY,
                   filename TEXT NOT NULL,
                   parent_uuid TEXT,
                   processed_at INTEGER NOT NULL,
                   status TEXT NOT NULL,
                   error_message TEXT
               )",
               [],
           )?;
           Ok(())
       }
   }
   ```

### Deliverable

- Compilable Cargo workspace with all crates
- Basic data structures defined
- SQLite initialization working
- All crates compile with `cargo build`

### Validation

```bash
cargo build --all
cargo test --all
```

---

## Phase 2: WASM Runtime Integration

**Goal**: Load and manage WASM modules using wasmtime

### Tasks

1. **Implement WasmRuntime**

   Create `crates/wadup-core/src/wasm.rs`:
   ```rust
   use wasmtime::*;
   use anyhow::Result;
   use std::path::Path;

   pub struct WasmRuntime {
       engine: Engine,
       modules: Vec<ModuleInfo>,
   }

   pub struct ModuleInfo {
       pub name: String,
       pub module: Module,
   }

   pub struct ResourceLimits {
       pub fuel: Option<u64>,
       pub max_memory: Option<usize>,
       pub max_stack: Option<usize>,
   }

   impl WasmRuntime {
       pub fn new(limits: ResourceLimits) -> Result<Self> {
           let mut config = Config::new();
           config.wasm_multi_memory(true);
           config.async_support(false);

           // Configure fuel (CPU) limits if specified
           if limits.fuel.is_some() {
               config.consume_fuel(true);
           }

           // Configure stack size limit if specified
           if let Some(max_stack) = limits.max_stack {
               config.max_wasm_stack(max_stack);
           }

           // Note: Memory limits are set per-instance via Store, not Config

           let engine = Engine::new(&config)?;

           Ok(Self {
               engine,
               modules: Vec::new(),
           })
       }

       pub fn load_modules(&mut self, dir: &Path) -> Result<()> {
           // Load all .wasm files from directory
           for entry in std::fs::read_dir(dir)? {
               let entry = entry?;
               let path = entry.path();

               if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                   let name = path.file_stem()
                       .and_then(|s| s.to_str())
                       .unwrap_or("unknown")
                       .to_string();

                   let module = Module::from_file(&self.engine, &path)?;

                   // Validate module exports
                   self.validate_module(&module)?;

                   self.modules.push(ModuleInfo { name, module });
               }
           }

           Ok(())
       }

       fn validate_module(&self, module: &Module) -> Result<()> {
           // Check for required 'process' function
           let has_process = module.exports()
               .any(|export| export.name() == "process");

           if !has_process {
               anyhow::bail!("Module missing required 'process' function");
           }

           Ok(())
       }
   }
   ```

2. **Implement Module Registry**

   Add to `wasm.rs`:
   ```rust
   pub struct ModuleRegistry {
       modules: Vec<(String, Module)>,
   }

   impl ModuleRegistry {
       pub fn new() -> Self {
           Self { modules: Vec::new() }
       }

       pub fn register(&mut self, name: String, module: Module) {
           self.modules.push((name, module));
       }

       pub fn iter(&self) -> impl Iterator<Item = (&str, &Module)> {
           self.modules.iter().map(|(n, m)| (n.as_str(), m))
       }
   }
   ```

3. **Implement Per-Thread Instance Management**

   Create `crates/wadup-core/src/instance.rs`:
   ```rust
   use wasmtime::*;
   use anyhow::Result;

   pub struct ModuleInstance {
       store: Store<InstanceContext>,
       instance: Instance,
   }

   pub struct InstanceContext {
       // Will hold processing state
   }

   struct ResourceLimiterImpl {
       max_memory: usize,
   }

   impl ResourceLimiter for ResourceLimiterImpl {
       fn memory_growing(&mut self, current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
           Ok(desired <= self.max_memory)
       }

       fn table_growing(&mut self, _current: u32, _desired: u32, _maximum: Option<u32>) -> Result<bool> {
           Ok(true)
       }
   }

   impl ModuleInstance {
       pub fn new(
           engine: &Engine,
           module: &Module,
           limits: &ResourceLimits,
       ) -> Result<Self> {
           let mut store = Store::new(engine, InstanceContext {});

           // Set fuel limit if specified
           if let Some(fuel) = limits.fuel {
               store.add_fuel(fuel)?;
           }

           // Set memory limits if specified
           if let Some(max_memory) = limits.max_memory {
               store.limiter(|_| ResourceLimiterImpl {
                   max_memory,
               });
           }

           let mut linker = Linker::new(engine);

           // Add dummy host functions for now
           Self::add_host_functions(&mut linker)?;

           let instance = linker.instantiate(&mut store, module)?;

           Ok(Self { store, instance })
       }

       fn add_host_functions(linker: &mut Linker<InstanceContext>) -> Result<()> {
           // Dummy implementations for now
           linker.func_wrap("env", "get_content_size", || -> i32 { 0 })?;
           Ok(())
       }

       pub fn call_process(&mut self) -> Result<i32> {
           let process = self.instance
               .get_typed_func::<(), i32>(&mut self.store, "process")?;

           let result = process.call(&mut self.store, ())?;

           Ok(result)
       }
   }
   ```

4. **Test Module Loading**

   Create a simple test WASM module:
   ```rust
   // In examples/simple-test/src/lib.rs
   #[no_mangle]
   pub extern "C" fn process() -> i32 {
       0 // Success
   }
   ```

   Build with: `cargo build --target wasm32-unknown-unknown`

   Create test in `wadup-core/src/wasm.rs`:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_load_module() {
           let limits = ResourceLimits {
               fuel: None,
               max_memory: None,
               max_stack: None,
           };
           let mut runtime = WasmRuntime::new(limits).unwrap();
           // Test with fixture
       }
   }
   ```

### Deliverable

- Can load WASM modules from directory
- Can validate module exports
- Can configure resource limits (fuel, memory, stack) via ResourceLimits
- Can instantiate modules with resource limits applied
- Can call `process()` function

### Validation

- Compile test WASM module
- Load it successfully with resource limits configured
- Call `process()` and get return value
- Verify fuel, memory, and stack limits are properly configured in Store/Config

---

## Phase 3: Host Functions Implementation

**Goal**: Implement complete WASM bindings for metadata and sub-content

### Tasks

1. **Design Host Function Context**

   Update `crates/wadup-bindings/src/context.rs`:
   ```rust
   use uuid::Uuid;
   use std::sync::Arc;

   pub struct ProcessingContext {
       pub content_uuid: Uuid,
       pub content_data: Arc<Vec<u8>>,
       pub subcontent: Vec<SubContentEmission>,
       pub metadata: Vec<MetadataRow>,
   }

   pub struct SubContentEmission {
       pub data: SubContentData,
       pub filename: String,
   }

   pub enum SubContentData {
       Bytes(Vec<u8>),
       Slice { offset: usize, length: usize },
   }

   pub struct MetadataRow {
       pub table_id: i32,
       pub values: Vec<Value>,
   }

   pub enum Value {
       Int64(i64),
       Float64(f64),
       String(String),
   }
   ```

2. **Implement Table Definition**

   Create `crates/wadup-bindings/src/host.rs`:
   ```rust
   pub fn define_table(
       mut caller: Caller<ProcessingContext>,
       name_ptr: i32,
       name_len: i32,
       columns_ptr: i32,
       columns_len: i32,
   ) -> Result<i32> {
       // Read table name from WASM memory
       let memory = caller.get_export("memory")
           .and_then(|e| e.into_memory())
           .ok_or_else(|| anyhow::anyhow!("No memory export"))?;

       let name = read_string(&caller, memory, name_ptr, name_len)?;
       let columns_json = read_string(&caller, memory, columns_ptr, columns_len)?;

       let columns: Vec<Column> = serde_json::from_str(&columns_json)?;

       // Validate and create table
       // (Will implement schema validation logic)

       Ok(0) // Return table ID
   }

   fn read_string(
       caller: &Caller<ProcessingContext>,
       memory: Memory,
       ptr: i32,
       len: i32,
   ) -> Result<String> {
       let mut buffer = vec![0u8; len as usize];
       memory.read(&caller, ptr as usize, &mut buffer)?;
       Ok(String::from_utf8(buffer)?)
   }
   ```

3. **Implement Row Insertion**

   Add to `host.rs`:
   ```rust
   pub fn insert_row(
       mut caller: Caller<ProcessingContext>,
       table_id: i32,
       row_data_ptr: i32,
       row_data_len: i32,
   ) -> Result<i32> {
       let memory = get_memory(&caller)?;
       let row_json = read_string(&caller, memory, row_data_ptr, row_data_len)?;
       let values: Vec<Value> = serde_json::from_str(&row_json)?;

       // Add to context's metadata collector
       caller.data_mut().metadata.push(MetadataRow {
           table_id,
           values,
       });

       Ok(0)
   }
   ```

4. **Implement Sub-Content Emission**

   Add to `host.rs`:
   ```rust
   pub fn emit_subcontent_bytes(
       mut caller: Caller<ProcessingContext>,
       data_ptr: i32,
       data_len: i32,
       filename_ptr: i32,
       filename_len: i32,
   ) -> Result<i32> {
       let memory = get_memory(&caller)?;

       let mut data = vec![0u8; data_len as usize];
       memory.read(&caller, data_ptr as usize, &mut data)?;

       let filename = read_string(&caller, memory, filename_ptr, filename_len)?;

       caller.data_mut().subcontent.push(SubContentEmission {
           data: SubContentData::Bytes(data),
           filename,
       });

       Ok(0)
   }

   pub fn emit_subcontent_slice(
       mut caller: Caller<ProcessingContext>,
       offset: i32,
       length: i32,
       filename_ptr: i32,
       filename_len: i32,
   ) -> Result<i32> {
       let memory = get_memory(&caller)?;
       let filename = read_string(&caller, memory, filename_ptr, filename_len)?;

       // Validate offset/length
       let content_size = caller.data().content_data.len();
       if offset < 0 || length < 0
           || (offset as usize + length as usize) > content_size {
           anyhow::bail!("Invalid offset/length for sub-content slice");
       }

       caller.data_mut().subcontent.push(SubContentEmission {
           data: SubContentData::Slice {
               offset: offset as usize,
               length: length as usize,
           },
           filename,
       });

       Ok(0)
   }
   ```

5. **Implement Content Access**

   Add to `host.rs`:
   ```rust
   pub fn get_content_size(caller: Caller<ProcessingContext>) -> i32 {
       caller.data().content_data.len() as i32
   }

   pub fn read_content(
       mut caller: Caller<ProcessingContext>,
       offset: i32,
       length: i32,
       dest_ptr: i32,
   ) -> Result<i32> {
       let memory = get_memory(&caller)?;
       let content = &caller.data().content_data;

       let offset = offset as usize;
       let length = length as usize;

       if offset + length > content.len() {
           anyhow::bail!("Read out of bounds");
       }

       memory.write(&mut caller, dest_ptr as usize, &content[offset..offset+length])?;

       Ok(0)
   }

   pub fn get_content_uuid(
       mut caller: Caller<ProcessingContext>,
       dest_ptr: i32,
   ) -> Result<i32> {
       let memory = get_memory(&caller)?;
       let uuid_bytes = caller.data().content_uuid.as_bytes();

       memory.write(&mut caller, dest_ptr as usize, uuid_bytes)?;

       Ok(0)
   }
   ```

6. **Link All Host Functions**

   Update instance creation to link all functions:
   ```rust
   fn add_host_functions(linker: &mut Linker<ProcessingContext>) -> Result<()> {
       linker.func_wrap("env", "define_table", define_table)?;
       linker.func_wrap("env", "insert_row", insert_row)?;
       linker.func_wrap("env", "emit_subcontent_bytes", emit_subcontent_bytes)?;
       linker.func_wrap("env", "emit_subcontent_slice", emit_subcontent_slice)?;
       linker.func_wrap("env", "get_content_size", get_content_size)?;
       linker.func_wrap("env", "read_content", read_content)?;
       linker.func_wrap("env", "get_content_uuid", get_content_uuid)?;
       Ok(())
   }
   ```

### Deliverable

- All host functions implemented
- WASM modules can call all bindings
- Sub-content and metadata collected correctly

### Validation

Create test WASM module that:
- Calls `get_content_size()`
- Calls `read_content()`
- Calls `define_table()`
- Calls `insert_row()`
- Calls `emit_subcontent_bytes()`

Verify collections work correctly.

---

## Phase 4: Content Processing Engine

**Goal**: Implement depth-first recursive processing with work-stealing parallelism

### Tasks

1. **Implement ContentStore**

   Update `crates/wadup-core/src/content.rs`:
   ```rust
   use std::collections::HashMap;
   use std::sync::{Arc, RwLock};
   use uuid::Uuid;

   pub struct ContentStore {
       store: Arc<RwLock<HashMap<Uuid, Arc<Vec<u8>>>>>,
   }

   impl ContentStore {
       pub fn new() -> Self {
           Self {
               store: Arc::new(RwLock::new(HashMap::new())),
           }
       }

       pub fn insert(&self, uuid: Uuid, data: Arc<Vec<u8>>) {
           self.store.write().unwrap().insert(uuid, data);
       }

       pub fn get(&self, uuid: &Uuid) -> Option<Arc<Vec<u8>>> {
           self.store.read().unwrap().get(uuid).cloned()
       }

       pub fn resolve(&self, content: &Content) -> Option<Arc<Vec<u8>>> {
           match &content.data {
               ContentData::Owned(data) => Some(data.clone()),
               ContentData::Borrowed { parent_uuid, offset, length } => {
                   let parent_data = self.get(parent_uuid)?;
                   let slice = parent_data[*offset..*offset+*length].to_vec();
                   Some(Arc::new(slice))
               }
           }
       }
   }
   ```

2. **Implement WorkQueue**

   Create `crates/wadup-core/src/queue.rs`:
   ```rust
   use crossbeam_deque::{Worker, Stealer, Steal};
   use crate::content::Content;

   pub struct WorkQueue {
       workers: Vec<Worker<Content>>,
       stealers: Vec<Stealer<Content>>,
   }

   impl WorkQueue {
       pub fn new(num_threads: usize) -> Self {
           let mut workers = Vec::new();
           let mut stealers = Vec::new();

           for _ in 0..num_threads {
               let worker = Worker::new_fifo();
               stealers.push(worker.stealer());
               workers.push(worker);
           }

           Self { workers, stealers }
       }

       pub fn get_worker(&mut self, index: usize) -> &Worker<Content> {
           &self.workers[index]
       }

       pub fn get_stealers(&self, exclude: usize) -> Vec<Stealer<Content>> {
           self.stealers.iter()
               .enumerate()
               .filter(|(i, _)| *i != exclude)
               .map(|(_, s)| s.clone())
               .collect()
       }
   }
   ```

3. **Implement File Loading**

   Create `crates/wadup-core/src/loader.rs`:
   ```rust
   use std::path::Path;
   use std::fs;
   use anyhow::Result;
   use crate::content::Content;

   pub fn load_files(input_dir: &Path) -> Result<Vec<Content>> {
       let mut contents = Vec::new();

       for entry in fs::read_dir(input_dir)? {
           let entry = entry?;
           let path = entry.path();

           if path.is_file() {
               let filename = path.file_name()
                   .and_then(|n| n.to_str())
                   .unwrap_or("unknown")
                   .to_string();

               let data = fs::read(&path)?;
               let content = Content::new_root(data, filename);

               contents.push(content);
           }
       }

       Ok(contents)
   }
   ```

4. **Implement Worker Thread Logic**

   Create `crates/wadup-core/src/worker.rs`:
   ```rust
   use crossbeam_deque::{Worker, Stealer, Steal};
   use crate::content::{Content, ContentStore};
   use crate::wasm::ModuleInstance;
   use anyhow::Result;

   pub struct WorkerThread {
       id: usize,
       worker: Worker<Content>,
       stealers: Vec<Stealer<Content>>,
       content_store: ContentStore,
       max_recursion_depth: usize,
   }

   impl WorkerThread {
       pub fn new(
           id: usize,
           worker: Worker<Content>,
           stealers: Vec<Stealer<Content>>,
           content_store: ContentStore,
           max_recursion_depth: usize,
       ) -> Self {
           Self { id, worker, stealers, content_store, max_recursion_depth }
       }

       pub fn run(&self, modules: Vec<ModuleInstance>) -> Result<()> {
           loop {
               let content = match self.get_work() {
                   Some(c) => c,
                   None => break, // No more work
               };

               self.process_content(content, &modules)?;
           }

           Ok(())
       }

       fn get_work(&self) -> Option<Content> {
           // Try local queue first
           if let Some(content) = self.worker.pop() {
               return Some(content);
           }

           // Try stealing from others
           loop {
               let mut retry = false;

               for stealer in &self.stealers {
                   match stealer.steal() {
                       Steal::Success(content) => return Some(content),
                       Steal::Empty => continue,
                       Steal::Retry => retry = true,
                   }
               }

               if !retry {
                   break;
               }
           }

           None
       }

       fn process_content(&self, content: Content, modules: &[ModuleInstance]) -> Result<()> {
           // Resolve content data
           let data = self.content_store.resolve(&content)
               .ok_or_else(|| anyhow::anyhow!("Content data not found"))?;

           // Process through each module
           for module in modules {
               // Set up context, call process(), collect results
               // When creating sub-content from module output, use:
               // Content::new_subcontent(&content, data, filename, self.max_recursion_depth)
               // (Will implement in detail)
           }

           Ok(())
       }
   }
   ```

5. **Implement Thread Pool Coordination**

   Create `crates/wadup-core/src/processor.rs`:
   ```rust
   use std::thread;
   use anyhow::Result;
   use crate::content::{Content, ContentStore};
   use crate::queue::WorkQueue;

   pub struct ContentProcessor {
       num_threads: usize,
       content_store: ContentStore,
       max_recursion_depth: usize,
   }

   impl ContentProcessor {
       pub fn new(num_threads: usize, max_recursion_depth: usize) -> Self {
           Self {
               num_threads,
               content_store: ContentStore::new(),
               max_recursion_depth,
           }
       }

       pub fn process(&self, initial_contents: Vec<Content>) -> Result<()> {
           let mut queue = WorkQueue::new(self.num_threads);

           // Add initial contents to queue 0
           for content in initial_contents {
               queue.get_worker(0).push(content);
           }

           // Spawn worker threads
           let mut handles = Vec::new();

           for i in 0..self.num_threads {
               let worker = queue.get_worker(i);
               let stealers = queue.get_stealers(i);
               let content_store = self.content_store.clone();
               let max_recursion_depth = self.max_recursion_depth;

               let handle = thread::spawn(move || {
                   // Worker logic here
                   // max_recursion_depth is available for creating sub-content
               });

               handles.push(handle);
           }

           // Wait for all threads
           for handle in handles {
               handle.join().unwrap()?;
           }

           Ok(())
       }
   }
   ```

### Deliverable

- Working multi-threaded processor
- Work-stealing queue functional
- Depth-first processing verified

### Validation

- Process single file through single module
- Process file that generates sub-content
- Verify depth-first order
- Test with multiple threads

---

## Phase 5: Metadata Management

**Goal**: Production-ready SQLite integration with schema validation

### Tasks

1. **Implement Full MetadataStore**

   Update `crates/wadup-core/src/metadata.rs`:
   ```rust
   use rusqlite::{Connection, params};
   use std::collections::HashMap;
   use std::sync::{Arc, Mutex};
   use anyhow::Result;

   pub struct MetadataStore {
       conn: Arc<Mutex<Connection>>,
       schemas: Arc<Mutex<HashMap<String, TableSchema>>>,
   }

   impl MetadataStore {
       pub fn new(path: &str) -> Result<Self> {
           let conn = Connection::open(path)?;

           // Enable WAL mode
           conn.execute("PRAGMA journal_mode=WAL", [])?;

           Self::init_tables(&conn)?;

           Ok(Self {
               conn: Arc::new(Mutex::new(conn)),
               schemas: Arc::new(Mutex::new(HashMap::new())),
           })
       }

       pub fn define_table(&self, schema: TableSchema) -> Result<i32> {
           let mut schemas = self.schemas.lock().unwrap();

           // Check if table already defined
           if let Some(existing) = schemas.get(&schema.name) {
               self.validate_schema_match(existing, &schema)?;
               return Ok(0); // Already exists, schema matches
           }

           // Create table in SQLite
           let conn = self.conn.lock().unwrap();
           self.create_table(&conn, &schema)?;

           // Store schema
           schemas.insert(schema.name.clone(), schema);

           Ok(0)
       }

       fn validate_schema_match(&self, existing: &TableSchema, new: &TableSchema) -> Result<()> {
           if existing.columns.len() != new.columns.len() {
               anyhow::bail!("Schema mismatch: different column count");
           }

           for (existing_col, new_col) in existing.columns.iter().zip(&new.columns) {
               if existing_col.name != new_col.name {
                   anyhow::bail!("Schema mismatch: column name '{}' vs '{}'",
                       existing_col.name, new_col.name);
               }
               if existing_col.data_type != new_col.data_type {
                   anyhow::bail!("Schema mismatch: column '{}' type mismatch",
                       existing_col.name);
               }
           }

           Ok(())
       }

       fn create_table(&self, conn: &Connection, schema: &TableSchema) -> Result<()> {
           let mut sql = format!("CREATE TABLE {} (", schema.name);
           sql.push_str("content_uuid TEXT NOT NULL, ");

           for col in &schema.columns {
               let sql_type = match col.data_type {
                   DataType::Int64 => "INTEGER",
                   DataType::Float64 => "REAL",
                   DataType::String => "TEXT",
               };
               sql.push_str(&format!("{} {}, ", col.name, sql_type));
           }

           sql.push_str("FOREIGN KEY(content_uuid) REFERENCES __wadup_content(uuid)");
           sql.push(')');

           conn.execute(&sql, [])?;

           Ok(())
       }

       pub fn insert_row(&self, table: &str, uuid: &str, values: &[Value]) -> Result<()> {
           let conn = self.conn.lock().unwrap();

           let placeholders = vec!["?"; values.len() + 1].join(", ");
           let sql = format!("INSERT INTO {} VALUES ({})", table, placeholders);

           let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(uuid.to_string())];
           for value in values {
               match value {
                   Value::Int64(v) => params.push(Box::new(*v)),
                   Value::Float64(v) => params.push(Box::new(*v)),
                   Value::String(v) => params.push(Box::new(v.clone())),
               }
           }

           conn.execute(&sql, params.as_slice())?;

           Ok(())
       }
   }
   ```

2. **Implement Batch Insertion**

   Add to `metadata.rs`:
   ```rust
   impl MetadataStore {
       pub fn insert_batch(&self, table: &str, uuid: &str, rows: Vec<Vec<Value>>) -> Result<()> {
           let conn = self.conn.lock().unwrap();
           let tx = conn.transaction()?;

           for row in rows {
               // Insert logic
           }

           tx.commit()?;
           Ok(())
       }
   }
   ```

3. **Implement Content Tracking**

   Add methods for `__wadup_content` table:
   ```rust
   impl MetadataStore {
       pub fn record_content_success(
           &self,
           uuid: &str,
           filename: &str,
           parent_uuid: Option<&str>,
       ) -> Result<()> {
           let conn = self.conn.lock().unwrap();

           conn.execute(
               "INSERT INTO __wadup_content
                (uuid, filename, parent_uuid, processed_at, status, error_message)
                VALUES (?1, ?2, ?3, ?4, 'success', NULL)",
               params![uuid, filename, parent_uuid, current_timestamp()],
           )?;

           Ok(())
       }

       pub fn record_content_failure(
           &self,
           uuid: &str,
           filename: &str,
           parent_uuid: Option<&str>,
           error: &str,
       ) -> Result<()> {
           let conn = self.conn.lock().unwrap();

           conn.execute(
               "INSERT INTO __wadup_content
                (uuid, filename, parent_uuid, processed_at, status, error_message)
                VALUES (?1, ?2, ?3, ?4, 'failed', ?5)",
               params![uuid, filename, parent_uuid, current_timestamp(), error],
           )?;

           Ok(())
       }
   }

   fn current_timestamp() -> i64 {
       std::time::SystemTime::now()
           .duration_since(std::time::UNIX_EPOCH)
           .unwrap()
           .as_secs() as i64
   }
   ```

### Deliverable

- Complete metadata management
- Schema validation working
- Batch operations implemented

### Validation

- Define same table from two modules (success)
- Define conflicting schemas (failure with clear error)
- Insert rows and verify in SQLite
- Verify UUID tracking

---

## Phase 6: Error Handling & Resilience

**Goal**: Graceful handling of WASM failures and resource limits

### Tasks

1. **Implement Fuel-Based CPU Limiting**

   Update `instance.rs`:
   ```rust
   impl ModuleInstance {
       pub fn call_process_with_fuel(&mut self, fuel: Option<u64>) -> Result<i32> {
           // Replenish fuel before each content processing
           if let Some(fuel_amount) = fuel {
               // Reset fuel to full amount for this content
               let _ = self.store.consume_fuel(0)?; // Check fuel is enabled
               self.store.add_fuel(fuel_amount)?;
           }

           // Call the process function
           match self.call_process() {
               Ok(result) => Ok(result),
               Err(e) => {
                   // Check if error is due to fuel exhaustion
                   let error_msg = e.to_string();
                   if error_msg.contains("fuel") || error_msg.contains("out of fuel") {
                       Err(anyhow::anyhow!("Module exceeded fuel limit (CPU limit)"))
                   } else {
                       Err(e)
                   }
               }
           }
       }

       pub fn call_process(&mut self) -> Result<i32> {
           let process = self.instance
               .get_typed_func::<(), i32>(&mut self.store, "process")?;

           let result = process.call(&mut self.store, ())?;

           Ok(result)
       }
   }
   ```

   Note: Fuel provides fine-grained CPU usage control. Each WASM instruction consumes fuel.
   When fuel reaches zero, execution traps. This is more precise than timeout-based limiting.

2. **Implement Trap Handling**

   Update worker to catch traps (fuel exhaustion, stack overflow, memory limit):
   ```rust
   fn process_with_module(
       &self,
       content: &Content,
       module: &mut ModuleInstance,
       fuel: Option<u64>
   ) -> Result<()> {
       match module.call_process_with_fuel(fuel) {
           Ok(0) => {
               // Success
               Ok(())
           }
           Ok(code) => {
               Err(anyhow::anyhow!("Module returned error code: {}", code))
           }
           Err(e) => {
               // Categorize the error for better logging
               let error_msg = e.to_string();
               let error_type = if error_msg.contains("fuel") {
                   "CPU limit exceeded (fuel exhausted)"
               } else if error_msg.contains("stack overflow") {
                   "Stack overflow"
               } else if error_msg.contains("memory") {
                   "Memory limit exceeded"
               } else {
                   "Module error"
               };

               tracing::error!(
                   content_uuid = %content.uuid,
                   module = %module.name,
                   error_type = error_type,
                   error = %e,
                   "Module failed"
               );
               Err(e)
           }
       }
   }
   ```

3. **Implement Recursion Depth Tracking**

   Update `Content`:
   ```rust
   pub struct Content {
       pub uuid: Uuid,
       pub data: ContentData,
       pub filename: String,
       pub parent_uuid: Option<Uuid>,
       pub depth: usize,
   }

   impl Content {
       pub fn new_subcontent(
           parent: &Content,
           data: ContentData,
           filename: String,
           max_depth: usize,
       ) -> Result<Self> {
           if parent.depth >= max_depth {
               anyhow::bail!("Max recursion depth exceeded (limit: {})", max_depth);
           }

           Ok(Self {
               uuid: Uuid::new_v4(),
               data,
               filename,
               parent_uuid: Some(parent.uuid),
               depth: parent.depth + 1,
           })
       }
   }
   ```

4. **Verify Resource Limit Implementation**

   Ensure all resource limits from CLI are properly enforced:
   - **Fuel limits**: Already implemented in Task 1 (per-content fuel replenishment)
   - **Memory limits**: Already implemented in Phase 2 (ResourceLimiter per Store)
   - **Stack limits**: Already implemented in Phase 2 (Config::max_wasm_stack)

   Test each limit:
   ```rust
   #[cfg(test)]
   mod tests {
       #[test]
       fn test_fuel_exhaustion() {
           // Create module with infinite loop
           // Set low fuel limit
           // Verify trap occurs
       }

       #[test]
       fn test_memory_limit() {
           // Create module that allocates large memory
           // Set memory limit
           // Verify allocation fails
       }

       #[test]
       fn test_stack_overflow() {
           // Create module with deep recursion
           // Verify stack overflow trap
       }
   }
   ```

5. **Implement Error Logging**

   Set up tracing subscriber in CLI:
   ```rust
   use tracing_subscriber;

   fn main() {
       tracing_subscriber::fmt()
           .with_max_level(tracing::Level::INFO)
           .init();

       // ...
   }
   ```

### Deliverable

- Fuel-based CPU limiting working
- Memory and stack limits enforced via wasmtime
- Trap catching functional (fuel exhaustion, stack overflow, memory limits)
- Recursion depth limits enforced
- Comprehensive error logging with error categorization

### Validation

- Create module with infinite loop → verify fuel exhaustion trap
- Create module that allocates excessive memory → verify memory limit trap
- Create module with deep recursion → verify stack overflow trap
- Create module that panics → verify panic caught
- Create deeply nested content → verify recursion depth limit works with custom `--max-recursion-depth` values
- Verify error messages clearly identify the type of failure

---

## Phase 7: Rust Guest Library

**Goal**: Ergonomic Rust library for WASM module authors

### Tasks

1. **Create Base Guest Library**

   Update `crates/wadup-guest/Cargo.toml`:
   ```toml
   [lib]
   crate-type = ["cdylib", "rlib"]

   [dependencies]
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```

2. **Declare External Host Functions**

   Create `crates/wadup-guest/src/ffi.rs`:
   ```rust
   extern "C" {
       pub fn define_table(name_ptr: *const u8, name_len: usize,
                          columns_ptr: *const u8, columns_len: usize) -> i32;
       pub fn insert_row(table_id: i32, row_ptr: *const u8, row_len: usize) -> i32;
       pub fn emit_subcontent_bytes(data_ptr: *const u8, data_len: usize,
                                    filename_ptr: *const u8, filename_len: usize) -> i32;
       pub fn emit_subcontent_slice(offset: usize, length: usize,
                                    filename_ptr: *const u8, filename_len: usize) -> i32;
       pub fn get_content_size() -> usize;
       pub fn read_content(offset: usize, length: usize, dest_ptr: *mut u8) -> i32;
       pub fn get_content_uuid(dest_ptr: *mut u8) -> i32;
   }
   ```

3. **Implement Safe Wrappers**

   Create `crates/wadup-guest/src/table.rs`:
   ```rust
   use crate::ffi;
   use anyhow::Result;

   pub struct TableBuilder {
       name: String,
       columns: Vec<Column>,
   }

   impl TableBuilder {
       pub fn new(name: impl Into<String>) -> Self {
           Self {
               name: name.into(),
               columns: Vec::new(),
           }
       }

       pub fn column(mut self, name: impl Into<String>, dtype: DataType) -> Self {
           self.columns.push(Column {
               name: name.into(),
               data_type: dtype,
           });
           self
       }

       pub fn define(self) -> Result<Table> {
           let columns_json = serde_json::to_string(&self.columns)?;

           unsafe {
               let result = ffi::define_table(
                   self.name.as_ptr(),
                   self.name.len(),
                   columns_json.as_ptr(),
                   columns_json.len(),
               );

               if result < 0 {
                   anyhow::bail!("Failed to define table");
               }
           }

           Ok(Table {
               id: 0,
               name: self.name,
               columns: self.columns,
           })
       }
   }

   pub struct Table {
       id: i32,
       name: String,
       columns: Vec<Column>,
   }

   impl Table {
       pub fn insert(&self) -> RowBuilder {
           RowBuilder::new(self.id, &self.columns)
       }
   }

   pub struct RowBuilder {
       table_id: i32,
       values: Vec<Value>,
   }

   impl RowBuilder {
       pub fn value(mut self, name: &str, value: impl Into<Value>) -> Self {
           self.values.push(value.into());
           self
       }

       pub fn execute(self) -> Result<()> {
           let values_json = serde_json::to_string(&self.values)?;

           unsafe {
               let result = ffi::insert_row(
                   self.table_id,
                   values_json.as_ptr(),
                   values_json.len(),
               );

               if result < 0 {
                   anyhow::bail!("Failed to insert row");
               }
           }

           Ok(())
       }
   }
   ```

4. **Implement Content Access**

   Create `crates/wadup-guest/src/content.rs`:
   ```rust
   use crate::ffi;
   use anyhow::Result;

   pub struct Content;

   impl Content {
       pub fn size() -> usize {
           unsafe { ffi::get_content_size() }
       }

       pub fn read(offset: usize, length: usize) -> Result<Vec<u8>> {
           let mut buffer = vec![0u8; length];

           unsafe {
               let result = ffi::read_content(offset, length, buffer.as_mut_ptr());

               if result < 0 {
                   anyhow::bail!("Failed to read content");
               }
           }

           Ok(buffer)
       }

       pub fn read_all() -> Result<Vec<u8>> {
           Self::read(0, Self::size())
       }
   }
   ```

5. **Implement SubContent Emission**

   Create `crates/wadup-guest/src/subcontent.rs`:
   ```rust
   use crate::ffi;
   use anyhow::Result;

   pub struct SubContent;

   impl SubContent {
       pub fn emit_bytes(data: &[u8], filename: &str) -> Result<()> {
           unsafe {
               let result = ffi::emit_subcontent_bytes(
                   data.as_ptr(),
                   data.len(),
                   filename.as_ptr(),
                   filename.len(),
               );

               if result < 0 {
                   anyhow::bail!("Failed to emit sub-content");
               }
           }

           Ok(())
       }

       pub fn emit_slice(offset: usize, length: usize, filename: &str) -> Result<()> {
           unsafe {
               let result = ffi::emit_subcontent_slice(
                   offset,
                   length,
                   filename.as_ptr(),
                   filename.len(),
               );

               if result < 0 {
                   anyhow::bail!("Failed to emit sub-content");
               }
           }

           Ok(())
       }
   }
   ```

6. **Create Example Modules**

   Create `examples/byte-counter/src/lib.rs`:
   ```rust
   use wadup_guest::*;

   #[no_mangle]
   pub extern "C" fn process() -> i32 {
       if let Err(e) = run() {
           eprintln!("Error: {}", e);
           return 1;
       }
       0
   }

   fn run() -> anyhow::Result<()> {
       let table = TableBuilder::new("file_sizes")
           .column("filename", DataType::String)
           .column("size", DataType::Int64)
           .define()?;

       let size = Content::size() as i64;

       table.insert()
           .value("filename", "test")
           .value("size", size)
           .execute()?;

       Ok(())
   }
   ```

### Deliverable

- Polished guest library
- Example modules working
- Documentation with examples

### Validation

- Build example modules to WASM
- Run through processor
- Verify clean, ergonomic API

---

## Phase 8: Integration Tests

**Goal**: Comprehensive end-to-end validation

### Test 1: SQLite Processing

Create `tests/test1_sqlite.rs`:

```rust
#[test]
fn test_sqlite_processing() {
    // Create test database
    let test_db = create_test_sqlite_db();

    // Create SQLite parser module
    build_example_module("sqlite-parser");

    // Run processor
    let processor = ContentProcessor::new(1);
    processor.process(vec![test_db])?;

    // Verify results
    let metadata = open_metadata_db();
    let rows = query_table(metadata, "sqlite_tables");

    assert_eq!(rows.len(), 3); // 3 tables in test DB
    assert!(rows.contains(&("users", 10)));
}
```

### Test 2: ZIP Processing

Create `tests/test2_zip.rs`:

```rust
#[test]
fn test_zip_processing() {
    // Create test ZIP
    let test_zip = create_test_zip();

    // Build modules
    build_example_module("zip-parser");
    build_example_module("byte-counter");

    // Run processor
    let processor = ContentProcessor::new(2);
    processor.process(vec![test_zip])?;

    // Verify sub-content created
    let metadata = open_metadata_db();
    let content_rows = query_table(metadata, "__wadup_content");

    assert_eq!(content_rows.len(), 3); // 1 ZIP + 2 files

    // Verify all processed by byte-counter
    let size_rows = query_table(metadata, "file_sizes");
    assert_eq!(size_rows.len(), 3);
}
```

### Test 3: Combined

Create `tests/test3_combined.rs`:

```rust
#[test]
fn test_combined_processing() {
    // Multiple input files
    let inputs = vec![
        create_test_sqlite_db(),
        create_test_zip(),
    ];

    // All modules
    build_example_module("sqlite-parser");
    build_example_module("zip-parser");
    build_example_module("byte-counter");

    // Run with multiple threads
    let processor = ContentProcessor::new(4);
    processor.process(inputs)?;

    // Verify all tables present
    let metadata = open_metadata_db();
    assert!(table_exists(metadata, "sqlite_tables"));
    assert!(table_exists(metadata, "file_sizes"));

    // Verify counts
    assert_eq!(total_content_count(metadata), 5);
}
```

### Additional Tests

- Error handling tests
- Schema conflict tests
- Large file tests
- Deep recursion tests

### Deliverable

Complete test suite covering all scenarios from DESIGN.md

---

## Phase 9: CLI & Polish

**Goal**: Production-ready tool with excellent UX

### Tasks

1. **Implement CLI with Clap**

   Update `crates/wadup-cli/src/main.rs`:
   ```rust
   use clap::Parser;

   #[derive(Parser)]
   #[command(name = "wadup")]
   #[command(about = "Web Assembly Data Unified Processing")]
   struct Cli {
       #[arg(long, help = "Directory containing WASM modules")]
       modules: PathBuf,

       #[arg(long, help = "Directory containing input files")]
       input: PathBuf,

       #[arg(long, help = "Output SQLite database path")]
       output: PathBuf,

       #[arg(long, default_value = "4", help = "Number of worker threads")]
       threads: usize,

       #[arg(long, help = "Fuel limit (CPU) per module per content (e.g., 10000000). If not set, no CPU limit.")]
       fuel: Option<u64>,

       #[arg(long, help = "Maximum memory in bytes per module instance (e.g., 67108864 for 64MB). If not set, uses wasmtime defaults.")]
       max_memory: Option<usize>,

       #[arg(long, help = "Maximum stack size in bytes per module instance (e.g., 1048576 for 1MB). If not set, uses wasmtime defaults.")]
       max_stack: Option<usize>,

       #[arg(long, default_value = "100", help = "Maximum recursion depth for sub-content (number of nesting levels allowed)")]
       max_recursion_depth: usize,

       #[arg(short, long, help = "Verbose output")]
       verbose: bool,
   }

   fn main() -> anyhow::Result<()> {
       let cli = Cli::parse();

       // Set up logging
       let level = if cli.verbose {
           tracing::Level::DEBUG
       } else {
           tracing::Level::INFO
       };

       tracing_subscriber::fmt()
           .with_max_level(level)
           .init();

       // Configure resource limits
       let limits = ResourceLimits {
           fuel: cli.fuel,
           max_memory: cli.max_memory,
           max_stack: cli.max_stack,
       };

       // Run processor
       let mut runtime = WasmRuntime::new(limits)?;
       runtime.load_modules(&cli.modules)?;

       let processor = ContentProcessor::new(
           cli.threads,
           cli.output,
           runtime,
           cli.max_recursion_depth,
       );

       let contents = load_files(&cli.input)?;

       processor.process(contents)?;

       println!("Processing complete!");

       Ok(())
   }
   ```

2. **Implement Progress Reporting**

   Add progress tracking:
   ```rust
   use std::sync::atomic::{AtomicUsize, Ordering};

   struct ProgressTracker {
       processed: AtomicUsize,
       total: AtomicUsize,
   }

   impl ProgressTracker {
       fn report(&self) {
           println!("Processed: {}/{}",
               self.processed.load(Ordering::Relaxed),
               self.total.load(Ordering::Relaxed));
       }
   }
   ```

3. **Implement Graceful Shutdown**

   Add Ctrl-C handling:
   ```rust
   use std::sync::atomic::{AtomicBool, Ordering};
   use std::sync::Arc;

   fn main() -> anyhow::Result<()> {
       let running = Arc::new(AtomicBool::new(true));
       let r = running.clone();

       ctrlc::set_handler(move || {
           println!("\nShutting down gracefully...");
           r.store(false, Ordering::SeqCst);
       })?;

       // Pass running flag to processor
       processor.process_with_flag(contents, running)?;

       Ok(())
   }
   ```

4. **Write Documentation**

   Create comprehensive README.md:
   - Installation instructions
   - Quick start guide
   - Example usage
   - Module development guide

   Add rustdoc comments throughout codebase.

5. **Performance Optimization**

   Profile and optimize:
   ```bash
   cargo install flamegraph
   cargo flamegraph --bin wadup -- --modules ./modules --input ./data --output ./out.db
   ```

   Optimize identified bottlenecks.

6. **Release Packaging**

   Create release builds:
   ```bash
   cargo build --release
   ```

   Create distribution packages.

### Deliverable

Production-ready WADUP tool ready for users

### Validation

- Run all integration tests
- Benchmark with large datasets
- Test CLI with various options
- Verify documentation accuracy

---

## Summary

This implementation plan provides a structured, incremental approach to building WADUP:

- **Phases 1-3**: Core infrastructure and WASM integration
- **Phases 4-6**: Processing engine and robustness
- **Phases 7-8**: Developer experience and validation
- **Phase 9**: Production readiness

Each phase has clear deliverables and validation criteria. The plan builds progressively, with each phase depending on the previous ones.

## Next Steps

After completing all phases:

1. Beta testing with real users
2. Performance benchmarking
3. Security audit
4. Additional guest libraries (Python, JavaScript)
5. Advanced features (streaming, distributed processing)
