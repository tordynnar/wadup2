use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wadup_bindings::ProcessingContext;
use crate::metadata::MetadataStore;
use std::io::Write;

#[derive(Clone)]
pub struct ResourceLimits {
    pub fuel: Option<u64>,
    pub max_memory: Option<usize>,
    pub max_stack: Option<usize>,
}

// Wrapper to combine ProcessingContext with WASI support
pub struct StoreData {
    pub processing_ctx: ProcessingContext,
    pub wasi_ctx: WasiP1Ctx,
}

pub struct WasmRuntime {
    engine: Engine,
    modules: Vec<ModuleInfo>,
    limits: ResourceLimits,
}

pub struct ModuleInfo {
    pub name: String,
    pub module: Module,
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

        let engine = Engine::new(&config)?;

        Ok(Self {
            engine,
            modules: Vec::new(),
            limits,
        })
    }

    pub fn load_modules(&mut self, dir: &Path) -> Result<()> {
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

                tracing::info!("Loaded WASM module: {}", name);
                self.modules.push(ModuleInfo { name, module });
            }
        }

        if self.modules.is_empty() {
            anyhow::bail!("No WASM modules found in directory");
        }

        Ok(())
    }

    fn validate_module(&self, module: &Module) -> Result<()> {
        let has_process = module.exports()
            .any(|export| export.name() == "process");

        if !has_process {
            anyhow::bail!("Module missing required 'process' function");
        }

        Ok(())
    }

    pub fn create_instances(
        &self,
        metadata_store: MetadataStore,
    ) -> Result<Vec<ModuleInstance>> {
        let mut instances = Vec::new();

        for module_info in &self.modules {
            let instance = ModuleInstance::new(
                &self.engine,
                &module_info.module,
                &module_info.name,
                &self.limits,
                metadata_store.clone(),
            )?;
            instances.push(instance);
        }

        Ok(instances)
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }
}

struct ResourceLimiterImpl {
    max_memory: usize,
}

impl ResourceLimiter for ResourceLimiterImpl {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
        Ok(desired <= self.max_memory)
    }

    fn table_growing(&mut self, _current: usize, _desired: usize, _maximum: Option<usize>) -> Result<bool> {
        Ok(true)
    }
}

pub struct ModuleInstance {
    store: Store<StoreData>,
    instance: Instance,
    name: String,
    fuel_limit: Option<u64>,
    metadata_store: MetadataStore,
    _limiter: Option<Box<ResourceLimiterImpl>>,
    vfs_root: PathBuf,  // Virtual filesystem root directory
}

impl ModuleInstance {
    pub fn new(
        engine: &Engine,
        module: &Module,
        name: &str,
        limits: &ResourceLimits,
        metadata_store: MetadataStore,
    ) -> Result<Self> {
        // Create a dummy context for initialization
        let dummy_ctx = ProcessingContext::new(
            uuid::Uuid::nil(),
            Arc::new(Vec::new()),
        );

        // Create virtual filesystem root directory
        let vfs_root = std::env::temp_dir().join(format!("wadup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&vfs_root)?;

        // Create /tmp subdirectory for writable operations
        let vfs_tmp = vfs_root.join("tmp");
        std::fs::create_dir_all(&vfs_tmp)?;

        // Create empty data.bin file (will be populated in process_content)
        let data_file = vfs_root.join("data.bin");
        std::fs::File::create(&data_file)?;

        // Create WASI context mounting our virtual filesystem as root
        use wasmtime_wasi::{DirPerms, FilePerms};
        let wasi_ctx = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdio()
            .preopened_dir(
                &vfs_root,
                "/",
                DirPerms::all(),
                FilePerms::all(),
            )?
            .build_p1();

        let store_data = StoreData {
            processing_ctx: dummy_ctx,
            wasi_ctx,
        };

        let mut store = Store::new(engine, store_data);

        // Set fuel limit if specified
        if let Some(fuel) = limits.fuel {
            store.set_fuel(fuel)?;
        }

        // TODO: Set memory limits if specified
        // Memory limiting through ResourceLimiter requires more complex lifetime management
        // For now, we rely on wasmtime's default memory limits
        let _limiter_box = limits.max_memory.map(|max_memory| {
            Box::new(ResourceLimiterImpl { max_memory })
        });

        let mut linker = Linker::new(engine);

        // Add WASI Preview1 functions
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |data: &mut StoreData| &mut data.wasi_ctx)?;

        // Add host functions
        Self::add_host_functions(&mut linker)?;

        let instance = linker.instantiate(&mut store, module)?;

        Ok(Self {
            store,
            instance,
            name: name.to_string(),
            fuel_limit: limits.fuel,
            metadata_store,
            _limiter: _limiter_box,
            vfs_root,
        })
    }

    fn add_host_functions(linker: &mut Linker<StoreData>) -> Result<()> {
        use wadup_bindings::context::{MetadataRow, SubContentEmission, SubContentData};
        use wadup_bindings::types::{Column, Value, TableSchema};

        // Helper to get memory
        fn get_memory<T>(caller: &mut Caller<T>) -> Result<Memory> {
            caller.get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| anyhow::anyhow!("No memory export found"))
        }

        // Helper to read string
        fn read_string<T>(caller: &mut Caller<T>, memory: Memory, ptr: i32, len: i32) -> Result<String> {
            if ptr < 0 || len < 0 {
                anyhow::bail!("Invalid pointer or length");
            }
            let mut buffer = vec![0u8; len as usize];
            memory.read(caller, ptr as usize, &mut buffer)?;
            Ok(String::from_utf8(buffer)?)
        }

        linker.func_wrap(
            "env",
            "define_table",
            |mut caller: Caller<StoreData>, name_ptr: i32, name_len: i32, cols_ptr: i32, cols_len: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let table_name = read_string(&mut caller, memory, name_ptr, name_len)?;
                let columns_json = read_string(&mut caller, memory, cols_ptr, cols_len)?;
                let columns: Vec<Column> = serde_json::from_str(&columns_json)?;
                caller.data_mut().processing_ctx.table_schemas.push(TableSchema {
                    name: table_name,
                    columns,
                });
                Ok(0)
            },
        )?;

        linker.func_wrap(
            "env",
            "insert_row",
            |mut caller: Caller<StoreData>, table_ptr: i32, table_len: i32, row_ptr: i32, row_len: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let table_name = read_string(&mut caller, memory, table_ptr, table_len)?;
                let row_json = read_string(&mut caller, memory, row_ptr, row_len)?;
                let values: Vec<Value> = serde_json::from_str(&row_json)?;
                caller.data_mut().processing_ctx.metadata.push(MetadataRow {
                    table_name,
                    values,
                });
                Ok(0)
            },
        )?;

        linker.func_wrap(
            "env",
            "emit_subcontent_bytes",
            |mut caller: Caller<StoreData>, data_ptr: i32, data_len: i32, fname_ptr: i32, fname_len: i32| -> Result<i32> {
                if data_ptr < 0 || data_len < 0 {
                    anyhow::bail!("Invalid data pointer or length");
                }
                let memory = get_memory(&mut caller)?;
                let mut data = vec![0u8; data_len as usize];
                memory.read(&caller, data_ptr as usize, &mut data)?;
                let filename = read_string(&mut caller, memory, fname_ptr, fname_len)?;
                caller.data_mut().processing_ctx.subcontent.push(SubContentEmission {
                    data: SubContentData::Bytes(data),
                    filename,
                });
                Ok(0)
            },
        )?;

        linker.func_wrap(
            "env",
            "emit_subcontent_slice",
            |mut caller: Caller<StoreData>, offset: i32, length: i32, fname_ptr: i32, fname_len: i32| -> Result<i32> {
                if offset < 0 || length < 0 {
                    anyhow::bail!("Invalid offset or length");
                }
                let content_size = caller.data().processing_ctx.content_data.len();
                if (offset as usize + length as usize) > content_size {
                    anyhow::bail!("Slice out of bounds");
                }
                let memory = get_memory(&mut caller)?;
                let filename = read_string(&mut caller, memory, fname_ptr, fname_len)?;
                caller.data_mut().processing_ctx.subcontent.push(SubContentEmission {
                    data: SubContentData::Slice {
                        offset: offset as usize,
                        length: length as usize,
                    },
                    filename,
                });
                Ok(0)
            },
        )?;

        Ok(())
    }

    pub fn process_content(
        &mut self,
        content_uuid: uuid::Uuid,
        content_data: Arc<Vec<u8>>,
    ) -> Result<ProcessingContext> {
        // Write content data to virtual filesystem at /data.bin
        let data_file_path = self.vfs_root.join("data.bin");
        let mut file = std::fs::File::create(&data_file_path)?;
        file.write_all(&content_data)?;
        drop(file);  // Ensure file is closed before WASM accesses it

        // Set up new context
        let ctx = ProcessingContext::new(content_uuid, content_data);
        self.store.data_mut().processing_ctx = ctx;

        // Replenish fuel
        if let Some(fuel) = self.fuel_limit {
            self.store.set_fuel(fuel)?;
        }

        // Call process function
        let process_func = self.instance
            .get_typed_func::<(), i32>(&mut self.store, "process")?;

        let result = process_func.call(&mut self.store, ());

        // Check result
        match result {
            Ok(0) => {
                // Success - extract context
                let ctx = &mut self.store.data_mut().processing_ctx;
                let extracted = ProcessingContext {
                    content_uuid: ctx.content_uuid,
                    content_data: ctx.content_data.clone(),
                    subcontent: std::mem::take(&mut ctx.subcontent),
                    metadata: std::mem::take(&mut ctx.metadata),
                    table_schemas: std::mem::take(&mut ctx.table_schemas),
                };
                Ok(extracted)
            }
            Ok(code) => {
                anyhow::bail!("Module '{}' returned error code: {}", self.name, code)
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("fuel") || error_msg.contains("out of fuel") {
                    anyhow::bail!("Module '{}' exceeded fuel limit (CPU limit)", self.name)
                } else if error_msg.contains("stack overflow") {
                    anyhow::bail!("Module '{}' stack overflow", self.name)
                } else if error_msg.contains("memory") {
                    anyhow::bail!("Module '{}' memory limit exceeded", self.name)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata_store(&self) -> &MetadataStore {
        &self.metadata_store
    }
}

impl Drop for ModuleInstance {
    fn drop(&mut self) {
        // Clean up virtual filesystem directory
        if self.vfs_root.exists() {
            if let Err(e) = std::fs::remove_dir_all(&self.vfs_root) {
                tracing::warn!("Failed to clean up VFS directory {:?}: {}", self.vfs_root, e);
            }
        }
    }
}
