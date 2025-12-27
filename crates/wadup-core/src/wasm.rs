use wasmtime::*;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use crate::bindings_context::ProcessingContext;
use crate::metadata::MetadataStore;
use crate::memory_fs::MemoryFilesystem;
use crate::wasi_impl::WasiCtx;

#[derive(Clone)]
pub struct ResourceLimits {
    pub fuel: Option<u64>,
    pub max_memory: Option<usize>,
    pub max_stack: Option<usize>,
}

// Wrapper to combine ProcessingContext with WASI support
pub struct StoreData {
    pub processing_ctx: ProcessingContext,
    pub wasi_ctx: WasiCtx,
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
            crate::shared_buffer::SharedBuffer::from_vec(Vec::new()),
        );

        // Create in-memory filesystem
        let filesystem = Arc::new(MemoryFilesystem::new());

        // Create /tmp directory
        filesystem.create_dir_all("/tmp")?;

        // Create empty /data.bin file
        filesystem.create_file("/data.bin", Vec::new())?;

        // Create WASI context with our in-memory filesystem
        let wasi_ctx = WasiCtx::new(filesystem);

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
        let _limiter_box = limits.max_memory.map(|max_memory| {
            Box::new(ResourceLimiterImpl { max_memory })
        });

        let mut linker = Linker::new(engine);

        // Add WASI Preview1 functions
        Self::add_wasi_functions(&mut linker)?;

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

    fn add_wasi_functions(linker: &mut Linker<StoreData>) -> Result<()> {
        use crate::wasi_impl::Errno;

        // Helper to get memory
        fn get_memory<T>(caller: &mut Caller<T>) -> Result<Memory> {
            caller.get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| anyhow::anyhow!("No memory export found"))
        }

        // Helper to read string from guest memory
        fn read_string<T>(caller: &Caller<T>, memory: Memory, ptr: i32, len: i32) -> Result<String> {
            if ptr < 0 || len < 0 {
                anyhow::bail!("Invalid pointer or length");
            }
            let mut buffer = vec![0u8; len as usize];
            memory.read(caller, ptr as usize, &mut buffer)?;
            Ok(String::from_utf8(buffer)?)
        }

        // fd_write - Write to file descriptor
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_write",
            |mut caller: Caller<StoreData>, fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;

                // Read iovec array
                let mut bufs = Vec::new();
                for i in 0..iovs_len {
                    let iov_ptr = iovs_ptr + (i * 8);
                    let mut iov_buf = [0u8; 8];
                    memory.read(&caller, iov_ptr as usize, &mut iov_buf)?;

                    let buf_ptr = u32::from_le_bytes([iov_buf[0], iov_buf[1], iov_buf[2], iov_buf[3]]);
                    let buf_len = u32::from_le_bytes([iov_buf[4], iov_buf[5], iov_buf[6], iov_buf[7]]);

                    let mut buf = vec![0u8; buf_len as usize];
                    memory.read(&caller, buf_ptr as usize, &mut buf)?;
                    bufs.push(buf);
                }

                let buf_refs: Vec<&[u8]> = bufs.iter().map(|b| b.as_slice()).collect();
                let mut nwritten = 0;
                let errno = caller.data().wasi_ctx.fd_write(fd as u32, &buf_refs, &mut nwritten);

                // Write result
                memory.write(&mut caller, nwritten_ptr as usize, &(nwritten as i32).to_le_bytes())?;

                Ok(errno as i32)
            },
        )?;

        // fd_read - Read from file descriptor
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_read",
            |mut caller: Caller<StoreData>, fd: i32, iovs_ptr: i32, iovs_len: i32, nread_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;

                // Read iovec array and prepare buffers
                let mut iov_info = Vec::new();
                for i in 0..iovs_len {
                    let iov_ptr = iovs_ptr + (i * 8);
                    let mut iov_buf = [0u8; 8];
                    memory.read(&caller, iov_ptr as usize, &mut iov_buf)?;

                    let buf_ptr = u32::from_le_bytes([iov_buf[0], iov_buf[1], iov_buf[2], iov_buf[3]]);
                    let buf_len = u32::from_le_bytes([iov_buf[4], iov_buf[5], iov_buf[6], iov_buf[7]]);
                    iov_info.push((buf_ptr, buf_len));
                }

                let mut total_read = 0;
                let mut temp_bufs: Vec<Vec<u8>> = iov_info.iter().map(|(_, len)| vec![0u8; *len as usize]).collect();
                let mut buf_refs: Vec<&mut [u8]> = temp_bufs.iter_mut().map(|b| b.as_mut_slice()).collect();

                let errno = caller.data().wasi_ctx.fd_read(fd as u32, &mut buf_refs, &mut total_read);

                // Write buffers back to guest memory
                let mut offset = 0;
                for (i, (buf_ptr, buf_len)) in iov_info.iter().enumerate() {
                    let to_write = (total_read - offset).min(*buf_len as usize);
                    if to_write > 0 {
                        memory.write(&mut caller, *buf_ptr as usize, &temp_bufs[i][..to_write])?;
                        offset += to_write;
                    }
                }

                memory.write(&mut caller, nread_ptr as usize, &(total_read as i32).to_le_bytes())?;

                Ok(errno as i32)
            },
        )?;

        // fd_seek - Seek in file
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_seek",
            |mut caller: Caller<StoreData>, fd: i32, offset: i64, whence: i32, newoffset_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let mut newoffset = 0u64;
                let errno = caller.data().wasi_ctx.fd_seek(fd as u32, offset, whence as u8, &mut newoffset);
                memory.write(&mut caller, newoffset_ptr as usize, &newoffset.to_le_bytes())?;
                Ok(errno as i32)
            },
        )?;

        // fd_close - Close file descriptor
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_close",
            |caller: Caller<StoreData>, fd: i32| -> Result<i32> {
                let errno = caller.data().wasi_ctx.fd_close(fd as u32);
                Ok(errno as i32)
            },
        )?;

        // fd_filestat_get - Get file metadata
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_filestat_get",
            |mut caller: Caller<StoreData>, fd: i32, filestat_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let mut filestat = [0u8; 64];
                let errno = caller.data().wasi_ctx.fd_filestat_get(fd as u32, &mut filestat);
                memory.write(&mut caller, filestat_ptr as usize, &filestat)?;
                Ok(errno as i32)
            },
        )?;

        // fd_prestat_get - Get preopen info
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_prestat_get",
            |mut caller: Caller<StoreData>, fd: i32, prestat_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let mut prestat = [0u8; 8];
                let errno = caller.data().wasi_ctx.fd_prestat_get(fd as u32, &mut prestat);
                memory.write(&mut caller, prestat_ptr as usize, &prestat)?;
                Ok(errno as i32)
            },
        )?;

        // fd_prestat_dir_name - Get preopen directory name
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_prestat_dir_name",
            |mut caller: Caller<StoreData>, fd: i32, path_ptr: i32, path_len: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let mut path_buf = vec![0u8; path_len as usize];
                let errno = caller.data().wasi_ctx.fd_prestat_dir_name(fd as u32, &mut path_buf);
                memory.write(&mut caller, path_ptr as usize, &path_buf)?;
                Ok(errno as i32)
            },
        )?;

        // path_open - Open a file or directory
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_open",
            |mut caller: Caller<StoreData>, dirfd: i32, dirflags: i32, path_ptr: i32, path_len: i32,
             oflags: i32, _fs_rights_base: i64, _fs_rights_inheriting: i64, fdflags: i32, fd_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let path = read_string(&caller, memory, path_ptr, path_len)?;
                let mut fd_out = 0u32;
                let errno = caller.data().wasi_ctx.path_open(
                    dirfd as u32,
                    dirflags as u32,
                    &path,
                    oflags as u16,
                    0,
                    0,
                    fdflags as u16,
                    &mut fd_out,
                );
                memory.write(&mut caller, fd_ptr as usize, &(fd_out as i32).to_le_bytes())?;
                Ok(errno as i32)
            },
        )?;

        // path_filestat_get - Get file metadata by path
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_filestat_get",
            |mut caller: Caller<StoreData>, dirfd: i32, flags: i32, path_ptr: i32, path_len: i32, filestat_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let path = read_string(&caller, memory, path_ptr, path_len)?;
                let mut filestat = [0u8; 64];
                let errno = caller.data().wasi_ctx.path_filestat_get(dirfd as u32, flags as u32, &path, &mut filestat);
                memory.write(&mut caller, filestat_ptr as usize, &filestat)?;
                Ok(errno as i32)
            },
        )?;

        // fd_readdir - Read directory entries
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_readdir",
            |mut caller: Caller<StoreData>, fd: i32, buf_ptr: i32, buf_len: i32, cookie: i64, bufused_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let mut buf = vec![0u8; buf_len as usize];
                let mut bufused = 0usize;
                let errno = caller.data().wasi_ctx.fd_readdir(fd as u32, &mut buf, cookie as u64, &mut bufused);
                memory.write(&mut caller, buf_ptr as usize, &buf[..bufused])?;
                memory.write(&mut caller, bufused_ptr as usize, &(bufused as i32).to_le_bytes())?;
                Ok(errno as i32)
            },
        )?;

        // proc_exit - Exit process
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "proc_exit",
            |_caller: Caller<StoreData>, code: i32| -> Result<()> {
                anyhow::bail!("proc_exit called with code {}", code)
            },
        )?;

        // environ_sizes_get - Get environment variable sizes
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "environ_sizes_get",
            |mut caller: Caller<StoreData>, count_ptr: i32, size_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                memory.write(&mut caller, count_ptr as usize, &0i32.to_le_bytes())?;
                memory.write(&mut caller, size_ptr as usize, &0i32.to_le_bytes())?;
                Ok(Errno::Success as i32)
            },
        )?;

        // environ_get - Get environment variables
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "environ_get",
            |_caller: Caller<StoreData>, _environ_ptr: i32, _environ_buf_ptr: i32| -> Result<i32> {
                Ok(Errno::Success as i32)
            },
        )?;

        // clock_time_get - Get current time
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "clock_time_get",
            |mut caller: Caller<StoreData>, _clock_id: i32, _precision: i64, time_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as i64;
                memory.write(&mut caller, time_ptr as usize, &time.to_le_bytes())?;
                Ok(Errno::Success as i32)
            },
        )?;

        // random_get - Get random bytes
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "random_get",
            |mut caller: Caller<StoreData>, buf_ptr: i32, buf_len: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let buf = vec![0u8; buf_len as usize]; // For now, zeros (should use rand crate)
                memory.write(&mut caller, buf_ptr as usize, &buf)?;
                Ok(Errno::Success as i32)
            },
        )?;

        // fd_tell - Get current file position (equivalent to fd_seek with whence=1, offset=0)
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_tell",
            |mut caller: Caller<StoreData>, fd: i32, offset_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let mut newoffset = 0u64;
                let errno = caller.data().wasi_ctx.fd_seek(fd as u32, 0, 1, &mut newoffset);
                memory.write(&mut caller, offset_ptr as usize, &newoffset.to_le_bytes())?;
                Ok(errno as i32)
            },
        )?;

        // fd_fdstat_get - Get file descriptor flags
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_fdstat_get",
            |mut caller: Caller<StoreData>, fd: i32, fdstat_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                // fdstat structure: filetype(1) + flags(2) + rights_base(8) + rights_inheriting(8) = 24 bytes
                let mut fdstat = [0u8; 24];
                // Set filetype based on FD (3=directory for fd 3, 4=regular for others)
                fdstat[0] = if fd == 3 { 3 } else { 4 }; // directory or regular file
                // flags (fdflags) - 0 for now
                // rights_base - all rights (0xFFFFFFFFFFFFFFFF)
                fdstat[4..12].copy_from_slice(&0xFFFFFFFFFFFFFFFFu64.to_le_bytes());
                // rights_inheriting - all rights
                fdstat[12..20].copy_from_slice(&0xFFFFFFFFFFFFFFFFu64.to_le_bytes());
                memory.write(&mut caller, fdstat_ptr as usize, &fdstat)?;
                Ok(Errno::Success as i32)
            },
        )?;

        // fd_fdstat_set_flags - Set file descriptor flags
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_fdstat_set_flags",
            |_caller: Caller<StoreData>, _fd: i32, _flags: i32| -> Result<i32> {
                // For now, just return success (we don't actually track these flags)
                Ok(Errno::Success as i32)
            },
        )?;

        // fd_filestat_set_size - Set file size (truncate/extend)
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_filestat_set_size",
            |_caller: Caller<StoreData>, _fd: i32, _size: i64| -> Result<i32> {
                // For now, just return success (our in-memory files auto-resize on write)
                Ok(Errno::Success as i32)
            },
        )?;

        // fd_sync - Sync file to storage
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_sync",
            |_caller: Caller<StoreData>, _fd: i32| -> Result<i32> {
                // No-op for in-memory filesystem
                Ok(Errno::Success as i32)
            },
        )?;

        // fd_datasync - Sync file data to storage
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_datasync",
            |_caller: Caller<StoreData>, _fd: i32| -> Result<i32> {
                // No-op for in-memory filesystem
                Ok(Errno::Success as i32)
            },
        )?;

        // path_create_directory - Create directory
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_create_directory",
            |mut caller: Caller<StoreData>, _dirfd: i32, path_ptr: i32, path_len: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                let path = read_string(&caller, memory, path_ptr, path_len)?;
                match caller.data().wasi_ctx.filesystem.create_dir_all(&path) {
                    Ok(_) => Ok(Errno::Success as i32),
                    Err(_) => Ok(Errno::Io as i32),
                }
            },
        )?;

        // path_unlink_file - Remove file
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_unlink_file",
            |_caller: Caller<StoreData>, _dirfd: i32, _path_ptr: i32, _path_len: i32| -> Result<i32> {
                // For now, not supported
                Ok(Errno::Nosys as i32)
            },
        )?;

        // path_remove_directory - Remove directory
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_remove_directory",
            |_caller: Caller<StoreData>, _dirfd: i32, _path_ptr: i32, _path_len: i32| -> Result<i32> {
                // For now, not supported
                Ok(Errno::Nosys as i32)
            },
        )?;

        // path_filestat_set_times - Set file timestamps
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_filestat_set_times",
            |_caller: Caller<StoreData>, _dirfd: i32, _flags: i32, _path_ptr: i32, _path_len: i32, _atim: i64, _mtim: i64, _fst_flags: i32| -> Result<i32> {
                // For now, just return success (we don't track timestamps)
                Ok(Errno::Success as i32)
            },
        )?;

        // fd_filestat_set_times - Set file timestamps by FD
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_filestat_set_times",
            |_caller: Caller<StoreData>, _fd: i32, _atim: i64, _mtim: i64, _fst_flags: i32| -> Result<i32> {
                // For now, just return success (we don't track timestamps)
                Ok(Errno::Success as i32)
            },
        )?;

        // path_readlink - Read symlink
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_readlink",
            |_caller: Caller<StoreData>, _dirfd: i32, _path_ptr: i32, _path_len: i32, _buf_ptr: i32, _buf_len: i32, _bufused_ptr: i32| -> Result<i32> {
                // Symlinks not supported
                Ok(Errno::Nosys as i32)
            },
        )?;

        // path_rename - Rename file
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_rename",
            |_caller: Caller<StoreData>, _old_dirfd: i32, _old_path_ptr: i32, _old_path_len: i32, _new_dirfd: i32, _new_path_ptr: i32, _new_path_len: i32| -> Result<i32> {
                // Not supported for now
                Ok(Errno::Nosys as i32)
            },
        )?;

        // path_symlink - Create symlink
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "path_symlink",
            |_caller: Caller<StoreData>, _old_path_ptr: i32, _old_path_len: i32, _dirfd: i32, _new_path_ptr: i32, _new_path_len: i32| -> Result<i32> {
                // Symlinks not supported
                Ok(Errno::Nosys as i32)
            },
        )?;

        // fd_advise - Advise on file usage pattern
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_advise",
            |_caller: Caller<StoreData>, _fd: i32, _offset: i64, _len: i64, _advice: i32| -> Result<i32> {
                // No-op for in-memory filesystem
                Ok(Errno::Success as i32)
            },
        )?;

        // fd_allocate - Allocate space for file
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "fd_allocate",
            |_caller: Caller<StoreData>, _fd: i32, _offset: i64, _len: i64| -> Result<i32> {
                // No-op for in-memory filesystem (files auto-grow)
                Ok(Errno::Success as i32)
            },
        )?;

        // sched_yield - Yield to scheduler
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "sched_yield",
            |_caller: Caller<StoreData>| -> Result<i32> {
                Ok(Errno::Success as i32)
            },
        )?;

        // args_sizes_get - Get command line argument sizes
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "args_sizes_get",
            |mut caller: Caller<StoreData>, count_ptr: i32, size_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                memory.write(&mut caller, count_ptr as usize, &0i32.to_le_bytes())?;
                memory.write(&mut caller, size_ptr as usize, &0i32.to_le_bytes())?;
                Ok(Errno::Success as i32)
            },
        )?;

        // args_get - Get command line arguments
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "args_get",
            |_caller: Caller<StoreData>, _argv_ptr: i32, _argv_buf_ptr: i32| -> Result<i32> {
                Ok(Errno::Success as i32)
            },
        )?;

        // poll_oneoff - Poll for events
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "poll_oneoff",
            |mut caller: Caller<StoreData>, _in_ptr: i32, _out_ptr: i32, _nsubscriptions: i32, nevents_ptr: i32| -> Result<i32> {
                let memory = get_memory(&mut caller)?;
                // Return that no events occurred
                memory.write(&mut caller, nevents_ptr as usize, &0i32.to_le_bytes())?;
                Ok(Errno::Success as i32)
            },
        )?;

        // sock_recv - Receive from socket
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "sock_recv",
            |_caller: Caller<StoreData>, _fd: i32, _ri_data_ptr: i32, _ri_data_len: i32, _ri_flags: i32, _ro_datalen_ptr: i32, _ro_flags_ptr: i32| -> Result<i32> {
                Ok(Errno::Nosys as i32)
            },
        )?;

        // sock_send - Send to socket
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "sock_send",
            |_caller: Caller<StoreData>, _fd: i32, _si_data_ptr: i32, _si_data_len: i32, _si_flags: i32, _so_datalen_ptr: i32| -> Result<i32> {
                Ok(Errno::Nosys as i32)
            },
        )?;

        // sock_shutdown - Shutdown socket
        linker.func_wrap(
            "wasi_snapshot_preview1",
            "sock_shutdown",
            |_caller: Caller<StoreData>, _fd: i32, _how: i32| -> Result<i32> {
                Ok(Errno::Nosys as i32)
            },
        )?;

        Ok(())
    }

    fn add_host_functions(linker: &mut Linker<StoreData>) -> Result<()> {
        use crate::bindings_context::{MetadataRow, SubContentEmission, SubContentData};
        use crate::bindings_types::{Column, Value, TableSchema};

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
        content_data: crate::shared_buffer::SharedBuffer,
    ) -> Result<ProcessingContext> {
        // Update /data.bin in the in-memory filesystem (zero-copy)
        let filesystem = &self.store.data().wasi_ctx.filesystem;
        filesystem.set_data_bin(content_data.to_bytes())?;

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
