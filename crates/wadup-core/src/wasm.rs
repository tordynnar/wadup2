use wasmtime::*;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use wadup_bindings::ProcessingContext;
use crate::metadata::MetadataStore;

#[derive(Clone)]
pub struct ResourceLimits {
    pub fuel: Option<u64>,
    pub max_memory: Option<usize>,
    pub max_stack: Option<usize>,
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
    store: Store<ProcessingContext>,
    instance: Instance,
    name: String,
    fuel_limit: Option<u64>,
    metadata_store: MetadataStore,
    _limiter: Option<Box<ResourceLimiterImpl>>,
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

        let mut store = Store::new(engine, dummy_ctx);

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
        })
    }

    fn add_host_functions(linker: &mut Linker<ProcessingContext>) -> Result<()> {
        use wadup_bindings::host;

        linker.func_wrap(
            "env",
            "define_table",
            |caller: Caller<ProcessingContext>, name_ptr: i32, name_len: i32, cols_ptr: i32, cols_len: i32| -> Result<i32> {
                host::define_table(caller, name_ptr, name_len, cols_ptr, cols_len)
            },
        )?;

        linker.func_wrap(
            "env",
            "insert_row",
            |caller: Caller<ProcessingContext>, table_ptr: i32, table_len: i32, row_ptr: i32, row_len: i32| -> Result<i32> {
                host::insert_row(caller, table_ptr, table_len, row_ptr, row_len)
            },
        )?;

        linker.func_wrap(
            "env",
            "emit_subcontent_bytes",
            |caller: Caller<ProcessingContext>, data_ptr: i32, data_len: i32, fname_ptr: i32, fname_len: i32| -> Result<i32> {
                host::emit_subcontent_bytes(caller, data_ptr, data_len, fname_ptr, fname_len)
            },
        )?;

        linker.func_wrap(
            "env",
            "emit_subcontent_slice",
            |caller: Caller<ProcessingContext>, offset: i32, length: i32, fname_ptr: i32, fname_len: i32| -> Result<i32> {
                host::emit_subcontent_slice(caller, offset, length, fname_ptr, fname_len)
            },
        )?;

        linker.func_wrap(
            "env",
            "get_content_size",
            |caller: Caller<ProcessingContext>| -> i32 {
                host::get_content_size(caller)
            },
        )?;

        linker.func_wrap(
            "env",
            "read_content",
            |caller: Caller<ProcessingContext>, offset: i32, length: i32, dest_ptr: i32| -> Result<i32> {
                host::read_content(caller, offset, length, dest_ptr)
            },
        )?;

        linker.func_wrap(
            "env",
            "get_content_uuid",
            |caller: Caller<ProcessingContext>, dest_ptr: i32| -> Result<i32> {
                host::get_content_uuid(caller, dest_ptr)
            },
        )?;

        Ok(())
    }

    pub fn process_content(
        &mut self,
        content_uuid: uuid::Uuid,
        content_data: Arc<Vec<u8>>,
    ) -> Result<ProcessingContext> {
        // Set up new context
        let ctx = ProcessingContext::new(content_uuid, content_data);
        *self.store.data_mut() = ctx;

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
                let ctx = self.store.data_mut();
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
