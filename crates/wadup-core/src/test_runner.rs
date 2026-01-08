//! Test runner for executing a single WASM module against a sample file.
//!
//! This module provides the `run_test` function used by the `wadup test` CLI command.
//! It outputs JSON in a format compatible with the WADUP Web backend.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use wasmtime::*;

use crate::memory_fs::MemoryFilesystem;
use crate::precompile::load_module_with_cache;
use crate::test_output::{SubcontentOutput, TestOutput};
use crate::wasm::ResourceLimits;

/// Maximum bytes for subcontent hex display (4KB).
const MAX_SUBCONTENT_HEX_BYTES: usize = 4096;

/// Run a single WASM module against a sample file and return JSON-compatible output.
///
/// # Arguments
/// * `module_path` - Path to the .wasm module file
/// * `sample_path` - Path to the sample file to process
/// * `filename` - Original filename (passed via WADUP_FILENAME env var)
/// * `limits` - Optional resource limits (fuel, memory, stack)
pub fn run_test(
    module_path: &Path,
    sample_path: &Path,
    filename: &str,
    limits: ResourceLimits,
) -> TestOutput {
    match run_test_inner(module_path, sample_path, filename, limits) {
        Ok(output) => output,
        Err(e) => TestOutput::failure(
            format!("Test execution error: {}", e),
            -1,
            String::new(),
            String::new(),
            None,
        ),
    }
}

fn run_test_inner(
    module_path: &Path,
    sample_path: &Path,
    filename: &str,
    limits: ResourceLimits,
) -> Result<TestOutput> {
    // Read the sample file
    let sample_data = std::fs::read(sample_path)
        .map_err(|e| anyhow::anyhow!("Failed to read sample file: {}", e))?;

    // Create engine with appropriate configuration
    let mut config = Config::new();
    config.wasm_multi_memory(true);
    config.async_support(false);

    if limits.fuel.is_some() {
        config.consume_fuel(true);
    }

    if let Some(max_stack) = limits.max_stack {
        config.max_wasm_stack(max_stack);
    }

    let engine = Engine::new(&config)?;

    // Load the module (uses cache if available)
    let module = load_module_with_cache(&engine, module_path)?;

    // Validate module exports
    let has_process = module.exports().any(|export| export.name() == "process");
    if !has_process {
        return Ok(TestOutput::failure(
            "Module does not export 'process' function",
            -1,
            String::new(),
            String::new(),
            None,
        ));
    }

    // Create in-memory filesystem
    let filesystem = Arc::new(MemoryFilesystem::new());
    filesystem.create_dir_all("/tmp")?;
    filesystem.create_dir_all("/metadata")?;
    filesystem.create_dir_all("/subcontent")?;

    // Write sample data to /data.bin
    filesystem.set_data_bin(bytes::Bytes::from(sample_data))?;

    // Create WASI context with environment variables
    let mut env_vars = HashMap::new();
    env_vars.insert("WADUP_FILENAME".to_string(), filename.to_string());
    let wasi_ctx = TestWasiCtx::new(filesystem.clone(), env_vars);

    // Create store data
    let store_data = TestStoreData {
        wasi_ctx,
        resource_limiter: limits.max_memory.map(|max| TestResourceLimiter { max_memory: max }),
    };

    let mut store = Store::new(&engine, store_data);

    // Set fuel limit if specified
    if let Some(fuel) = limits.fuel {
        store.set_fuel(fuel)?;
    }

    // Set up resource limiter if specified
    if store.data().resource_limiter.is_some() {
        store.limiter(|data| data.resource_limiter.as_mut().unwrap());
    }

    // Create linker with WASI functions
    let mut linker = Linker::new(&engine);
    add_wasi_functions(&mut linker)?;

    // Instantiate module
    let instance = match linker.instantiate(&mut store, &module) {
        Ok(inst) => inst,
        Err(e) => {
            return Ok(TestOutput::failure(
                format!("Failed to instantiate module: {}", e),
                -1,
                String::new(),
                String::new(),
                None,
            ));
        }
    };

    // Handle Go/TinyGo initialization
    // First try _initialize (reactor mode - TinyGo with -buildmode=c-shared)
    let has_initialize = instance.get_typed_func::<(), ()>(&mut store, "_initialize").is_ok();

    if has_initialize {
        if let Ok(init_func) = instance.get_typed_func::<(), ()>(&mut store, "_initialize") {
            if let Err(e) = init_func.call(&mut store, ()) {
                let (stdout, _) = store.data().wasi_ctx.take_stdout();
                let (stderr, _) = store.data().wasi_ctx.take_stderr();
                return Ok(TestOutput::failure(
                    format!("Runtime initialization (_initialize) failed: {}", e),
                    -1,
                    stdout,
                    stderr,
                    None,
                ));
            }
        }
    } else {
        // Try _start (command mode) - only if there's also a process function
        if let Ok(start_func) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
            // Call _start and handle exit trap
            match start_func.call(&mut store, ()) {
                Ok(()) => {}
                Err(e) => {
                    // Check if it's an ExitTrap with code 0 (normal initialization)
                    if let Some(exit_trap) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                        if exit_trap.0 != 0 {
                            let (stdout, _) = store.data().wasi_ctx.take_stdout();
                            let (stderr, _) = store.data().wasi_ctx.take_stderr();
                            return Ok(TestOutput::failure(
                                format!("Runtime initialization (_start) failed with exit code {}", exit_trap.0),
                                exit_trap.0,
                                stdout,
                                stderr,
                                None,
                            ));
                        }
                    }
                    // Other errors during _start are ignored (some init errors are OK)
                }
            }
        }
    }

    // Call the process function
    let exit_code;
    let error_msg;

    if let Ok(process_func) = instance.get_typed_func::<(), i32>(&mut store, "process") {
        match process_func.call(&mut store, ()) {
            Ok(code) => {
                exit_code = code;
                error_msg = if code != 0 {
                    Some(format!("Module returned error code: {}", code))
                } else {
                    None
                };
            }
            Err(e) => {
                exit_code = -1;
                error_msg = Some(classify_error(&e));
            }
        }
    } else if let Ok(process_func) = instance.get_typed_func::<(), ()>(&mut store, "process") {
        match process_func.call(&mut store, ()) {
            Ok(()) => {
                exit_code = 0;
                error_msg = None;
            }
            Err(e) => {
                exit_code = -1;
                error_msg = Some(classify_error(&e));
            }
        }
    } else {
        let (stdout, _) = store.data().wasi_ctx.take_stdout();
        let (stderr, _) = store.data().wasi_ctx.take_stderr();
        return Ok(TestOutput::failure(
            "Module 'process' function has unsupported signature",
            -1,
            stdout,
            stderr,
            None,
        ));
    }

    // Capture stdout/stderr
    let (stdout, _stdout_truncated) = store.data().wasi_ctx.take_stdout();
    let (stderr, _stderr_truncated) = store.data().wasi_ctx.take_stderr();

    // Read metadata from /metadata/*.json files
    let metadata = read_metadata_files(&filesystem);

    // Read subcontent from /subcontent/
    let subcontent = read_subcontent_files(&filesystem);

    // Build output
    if exit_code == 0 && error_msg.is_none() {
        Ok(TestOutput::success(stdout, stderr, metadata, subcontent))
    } else {
        Ok(TestOutput {
            success: false,
            error: error_msg,
            stdout,
            stderr,
            exit_code,
            metadata,
            subcontent,
        })
    }
}

/// Classify a WASM execution error into a user-friendly message.
fn classify_error(e: &anyhow::Error) -> String {
    let error_msg = e.to_string();
    if error_msg.contains("fuel") || error_msg.contains("out of fuel") {
        "Module exceeded fuel limit (CPU limit)".to_string()
    } else if error_msg.contains("stack overflow") {
        "Module stack overflow".to_string()
    } else if error_msg.contains("memory") {
        "Module memory limit exceeded".to_string()
    } else {
        format!("Module execution error: {}", error_msg)
    }
}

/// Read all /metadata/*.json files and merge them.
fn read_metadata_files(filesystem: &Arc<MemoryFilesystem>) -> Option<serde_json::Value> {
    let metadata_dir = match filesystem.get_dir("/metadata") {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    let entries = metadata_dir.list();
    let mut json_files: Vec<(String, serde_json::Value)> = Vec::new();

    for (name, is_dir) in entries {
        if is_dir || !name.ends_with(".json") {
            continue;
        }

        let path = format!("/metadata/{}", name);
        if let Ok(contents) = filesystem.read_file(&path) {
            if let Ok(content_str) = String::from_utf8(contents) {
                if !content_str.trim().is_empty() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content_str) {
                        json_files.push((name, json));
                    }
                }
            }
        }
    }

    if json_files.is_empty() {
        return None;
    }

    // Sort by filename for consistent ordering
    json_files.sort_by(|a, b| a.0.cmp(&b.0));

    // If single file, return it directly; otherwise return array
    if json_files.len() == 1 {
        Some(json_files.into_iter().next().unwrap().1)
    } else {
        Some(serde_json::Value::Array(
            json_files.into_iter().map(|(_, v)| v).collect(),
        ))
    }
}

/// Read all /subcontent/ files and convert to SubcontentOutput.
fn read_subcontent_files(filesystem: &Arc<MemoryFilesystem>) -> Option<Vec<SubcontentOutput>> {
    let subcontent_dir = match filesystem.get_dir("/subcontent") {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    let entries = subcontent_dir.list();
    let mut outputs: Vec<SubcontentOutput> = Vec::new();

    // Find all data_N.bin files
    for (name, is_dir) in &entries {
        if *is_dir || !name.starts_with("data_") || !name.ends_with(".bin") {
            continue;
        }

        // Extract index from filename
        let idx_str = name.trim_start_matches("data_").trim_end_matches(".bin");
        let idx: usize = match idx_str.parse() {
            Ok(i) => i,
            Err(_) => continue,
        };

        // Read the data file
        let data_path = format!("/subcontent/{}", name);
        let data = match filesystem.read_file(&data_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let size = data.len();
        let truncated = size > MAX_SUBCONTENT_HEX_BYTES;
        let data_for_hex = if truncated {
            &data[..MAX_SUBCONTENT_HEX_BYTES]
        } else {
            &data[..]
        };
        let data_hex = hex::encode(data_for_hex);

        // Read corresponding metadata if exists
        let meta_path = format!("/subcontent/metadata_{}.json", idx);
        let (filename_opt, metadata_opt) = if let Ok(meta_contents) = filesystem.read_file(&meta_path) {
            if let Ok(meta_str) = String::from_utf8(meta_contents) {
                if let Ok(meta_json) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                    let filename = meta_json.get("filename").and_then(|v| v.as_str()).map(|s| s.to_string());
                    (filename, Some(meta_json))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        outputs.push(SubcontentOutput {
            index: idx,
            filename: filename_opt,
            data_hex,
            size,
            truncated,
            metadata: metadata_opt,
        });
    }

    if outputs.is_empty() {
        return None;
    }

    // Sort by index
    outputs.sort_by_key(|o| o.index);

    Some(outputs)
}

// ============================================================================
// Test-specific WASI implementation
// ============================================================================

/// Store data for test execution (simplified - no MetadataStore needed).
struct TestStoreData {
    wasi_ctx: TestWasiCtx,
    resource_limiter: Option<TestResourceLimiter>,
}

struct TestResourceLimiter {
    max_memory: usize,
}

impl ResourceLimiter for TestResourceLimiter {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
        Ok(desired <= self.max_memory)
    }

    fn table_growing(&mut self, _current: usize, _desired: usize, _maximum: Option<usize>) -> Result<bool> {
        Ok(true)
    }
}

/// Maximum bytes to capture from stdout/stderr (1 MB).
const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

/// Simplified WASI context for test execution with environment variable support.
struct TestWasiCtx {
    filesystem: Arc<MemoryFilesystem>,
    file_table: HashMap<u32, TestFileHandle>,
    next_fd: u32,
    env_vars: HashMap<String, String>,
    stdout_capture: Vec<u8>,
    stderr_capture: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

enum TestFileHandle {
    File(crate::memory_fs::MemoryFile, Option<String>),
    Directory(crate::memory_fs::MemoryDirectory, usize),
    Stdin,
    Stdout,
    Stderr,
}

impl TestWasiCtx {
    fn new(filesystem: Arc<MemoryFilesystem>, env_vars: HashMap<String, String>) -> Self {
        let mut file_table = HashMap::new();
        file_table.insert(0, TestFileHandle::Stdin);
        file_table.insert(1, TestFileHandle::Stdout);
        file_table.insert(2, TestFileHandle::Stderr);
        file_table.insert(3, TestFileHandle::Directory(filesystem.root().clone(), 0));

        Self {
            filesystem,
            file_table,
            next_fd: 4,
            env_vars,
            stdout_capture: Vec::new(),
            stderr_capture: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    fn take_stdout(&self) -> (String, bool) {
        let text = String::from_utf8_lossy(&self.stdout_capture).to_string();
        (text, self.stdout_truncated)
    }

    fn take_stderr(&self) -> (String, bool) {
        let text = String::from_utf8_lossy(&self.stderr_capture).to_string();
        (text, self.stderr_truncated)
    }
}

/// Add WASI functions to the linker for test execution.
fn add_wasi_functions(linker: &mut Linker<TestStoreData>) -> Result<()> {
    use crate::wasi_impl::Errno;
    use std::io::{Read, Seek, SeekFrom, Write};

    fn get_memory<T>(caller: &mut Caller<T>) -> Result<Memory> {
        caller
            .get_export("memory")
            .and_then(|e| e.into_memory())
            .ok_or_else(|| anyhow::anyhow!("No memory export found"))
    }

    fn read_string<T>(caller: &Caller<T>, memory: Memory, ptr: i32, len: i32) -> Result<String> {
        if ptr < 0 || len < 0 {
            anyhow::bail!("Invalid pointer or length");
        }
        let mut buffer = vec![0u8; len as usize];
        memory.read(caller, ptr as usize, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    // fd_write
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_write",
        |mut caller: Caller<TestStoreData>, fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;

            // Collect all data to write
            let mut all_data = Vec::new();
            for i in 0..iovs_len {
                let iov_ptr = iovs_ptr + (i * 8);
                let mut iov_buf = [0u8; 8];
                memory.read(&caller, iov_ptr as usize, &mut iov_buf)?;

                let buf_ptr = u32::from_le_bytes([iov_buf[0], iov_buf[1], iov_buf[2], iov_buf[3]]);
                let buf_len = u32::from_le_bytes([iov_buf[4], iov_buf[5], iov_buf[6], iov_buf[7]]);

                let mut buf = vec![0u8; buf_len as usize];
                memory.read(&caller, buf_ptr as usize, &mut buf)?;
                all_data.extend_from_slice(&buf);
            }

            let nwritten = all_data.len();
            let store_data = caller.data_mut();

            match fd {
                1 => {
                    // stdout
                    if store_data.wasi_ctx.stdout_capture.len() < MAX_CAPTURE_BYTES {
                        let remaining = MAX_CAPTURE_BYTES - store_data.wasi_ctx.stdout_capture.len();
                        if all_data.len() > remaining {
                            store_data.wasi_ctx.stdout_capture.extend_from_slice(&all_data[..remaining]);
                            store_data.wasi_ctx.stdout_truncated = true;
                        } else {
                            store_data.wasi_ctx.stdout_capture.extend_from_slice(&all_data);
                        }
                    }
                }
                2 => {
                    // stderr
                    if store_data.wasi_ctx.stderr_capture.len() < MAX_CAPTURE_BYTES {
                        let remaining = MAX_CAPTURE_BYTES - store_data.wasi_ctx.stderr_capture.len();
                        if all_data.len() > remaining {
                            store_data.wasi_ctx.stderr_capture.extend_from_slice(&all_data[..remaining]);
                            store_data.wasi_ctx.stderr_truncated = true;
                        } else {
                            store_data.wasi_ctx.stderr_capture.extend_from_slice(&all_data);
                        }
                    }
                }
                _ => {
                    // Regular file
                    if let Some(TestFileHandle::File(ref mut file, _)) = store_data.wasi_ctx.file_table.get_mut(&(fd as u32)) {
                        let _ = file.write_all(&all_data);
                    } else {
                        return Ok(Errno::Badf as i32);
                    }
                }
            }

            memory.write(&mut caller, nwritten_ptr as usize, &(nwritten as i32).to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_read
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_read",
        |mut caller: Caller<TestStoreData>, fd: i32, iovs_ptr: i32, iovs_len: i32, nread_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;

            // Read iovec info first
            let mut iov_info = Vec::new();
            for i in 0..iovs_len {
                let iov_ptr = iovs_ptr + (i * 8);
                let mut iov_buf = [0u8; 8];
                memory.read(&caller, iov_ptr as usize, &mut iov_buf)?;

                let buf_ptr = u32::from_le_bytes([iov_buf[0], iov_buf[1], iov_buf[2], iov_buf[3]]);
                let buf_len = u32::from_le_bytes([iov_buf[4], iov_buf[5], iov_buf[6], iov_buf[7]]);
                iov_info.push((buf_ptr, buf_len));
            }

            let store_data = caller.data_mut();

            let mut total_read = 0usize;
            let mut read_data = Vec::new();

            match store_data.wasi_ctx.file_table.get_mut(&(fd as u32)) {
                Some(TestFileHandle::File(ref mut file, _)) => {
                    let total_len: usize = iov_info.iter().map(|(_, len)| *len as usize).sum();
                    read_data.resize(total_len, 0);
                    total_read = file.read(&mut read_data).unwrap_or(0);
                    read_data.truncate(total_read);
                }
                Some(TestFileHandle::Stdin) => {
                    // No stdin for test
                }
                _ => {
                    return Ok(Errno::Badf as i32);
                }
            }

            // Write data back to guest memory
            let mut offset = 0;
            for (buf_ptr, buf_len) in iov_info {
                let to_write = (total_read - offset).min(buf_len as usize);
                if to_write > 0 && offset < read_data.len() {
                    memory.write(&mut caller, buf_ptr as usize, &read_data[offset..offset + to_write])?;
                    offset += to_write;
                }
            }

            memory.write(&mut caller, nread_ptr as usize, &(total_read as i32).to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_seek
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_seek",
        |mut caller: Caller<TestStoreData>, fd: i32, offset: i64, whence: i32, newoffset_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let store_data = caller.data_mut();

            let seek_from = match whence {
                0 => SeekFrom::Start(offset as u64),
                1 => SeekFrom::Current(offset),
                2 => SeekFrom::End(offset),
                _ => return Ok(Errno::Inval as i32),
            };

            let new_pos = match store_data.wasi_ctx.file_table.get_mut(&(fd as u32)) {
                Some(TestFileHandle::File(ref mut file, _)) => {
                    file.seek(seek_from).unwrap_or(0)
                }
                _ => return Ok(Errno::Badf as i32),
            };

            memory.write(&mut caller, newoffset_ptr as usize, &new_pos.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_close
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_close",
        |mut caller: Caller<TestStoreData>, fd: i32| -> Result<i32> {
            let store_data = caller.data_mut();
            if store_data.wasi_ctx.file_table.remove(&(fd as u32)).is_some() {
                Ok(Errno::Success as i32)
            } else {
                Ok(Errno::Badf as i32)
            }
        },
    )?;

    // fd_prestat_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_get",
        |mut caller: Caller<TestStoreData>, fd: i32, prestat_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let store_data = caller.data();

            if fd == 3 {
                // Root directory preopen
                let mut prestat = [0u8; 8];
                prestat[0] = 0; // type = directory
                prestat[4..8].copy_from_slice(&1u32.to_le_bytes()); // name length = 1 (for "/")
                memory.write(&mut caller, prestat_ptr as usize, &prestat)?;
                Ok(Errno::Success as i32)
            } else {
                Ok(Errno::Badf as i32)
            }
        },
    )?;

    // fd_prestat_dir_name
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_dir_name",
        |mut caller: Caller<TestStoreData>, fd: i32, path_ptr: i32, path_len: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;

            if fd == 3 && path_len >= 1 {
                memory.write(&mut caller, path_ptr as usize, b"/")?;
                Ok(Errno::Success as i32)
            } else {
                Ok(Errno::Badf as i32)
            }
        },
    )?;

    // path_open
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_open",
        |mut caller: Caller<TestStoreData>,
         dirfd: i32,
         _dirflags: i32,
         path_ptr: i32,
         path_len: i32,
         oflags: i32,
         _fs_rights_base: i64,
         _fs_rights_inheriting: i64,
         _fdflags: i32,
         fd_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let path = read_string(&caller, memory, path_ptr, path_len)?;

            let store_data = caller.data_mut();

            // Resolve path relative to dirfd
            let full_path = if dirfd == 3 {
                format!("/{}", path.trim_start_matches('/'))
            } else {
                return Ok(Errno::Badf as i32);
            };

            let o_creat = (oflags & 1) != 0;
            let o_trunc = (oflags & 8) != 0;

            // Try to open existing file or create new one
            let file = if let Ok(f) = store_data.wasi_ctx.filesystem.open_file(&full_path) {
                if o_trunc {
                    let _ = f.truncate();
                }
                f
            } else if o_creat {
                store_data.wasi_ctx.filesystem.create_file(&full_path, Vec::new())?;
                store_data.wasi_ctx.filesystem.open_file(&full_path)?
            } else {
                return Ok(Errno::Noent as i32);
            };

            let new_fd = store_data.wasi_ctx.next_fd;
            store_data.wasi_ctx.next_fd += 1;
            store_data.wasi_ctx.file_table.insert(new_fd, TestFileHandle::File(file, Some(full_path)));

            memory.write(&mut caller, fd_ptr as usize, &new_fd.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_filestat_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_filestat_get",
        |mut caller: Caller<TestStoreData>, fd: i32, filestat_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let store_data = caller.data();

            let mut filestat = [0u8; 64];

            match store_data.wasi_ctx.file_table.get(&(fd as u32)) {
                Some(TestFileHandle::File(file, _)) => {
                    let size = file.len() as u64;
                    filestat[16] = 4; // filetype = regular file (offset 16)
                    filestat[32..40].copy_from_slice(&size.to_le_bytes()); // size (offset 32)
                }
                Some(TestFileHandle::Directory(_, _)) => {
                    filestat[16] = 3; // filetype = directory
                }
                _ => return Ok(Errno::Badf as i32),
            }

            memory.write(&mut caller, filestat_ptr as usize, &filestat)?;
            Ok(Errno::Success as i32)
        },
    )?;

    // environ_sizes_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_sizes_get",
        |mut caller: Caller<TestStoreData>, count_ptr: i32, size_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let store_data = caller.data();

            let count = store_data.wasi_ctx.env_vars.len() as i32;
            let mut total_size = 0i32;
            for (k, v) in &store_data.wasi_ctx.env_vars {
                total_size += (k.len() + 1 + v.len() + 1) as i32; // "KEY=VALUE\0"
            }

            memory.write(&mut caller, count_ptr as usize, &count.to_le_bytes())?;
            memory.write(&mut caller, size_ptr as usize, &total_size.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // environ_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_get",
        |mut caller: Caller<TestStoreData>, environ_ptr: i32, environ_buf_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;

            // Collect env vars to avoid borrow checker issues
            let env_vars: Vec<(String, String)> = caller.data().wasi_ctx.env_vars
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            let mut buf_offset = environ_buf_ptr as usize;
            let mut ptr_offset = environ_ptr as usize;

            for (k, v) in env_vars {
                // Write pointer to this env var
                memory.write(&mut caller, ptr_offset, &(buf_offset as u32).to_le_bytes())?;
                ptr_offset += 4;

                // Write "KEY=VALUE\0"
                let env_str = format!("{}={}\0", k, v);
                memory.write(&mut caller, buf_offset, env_str.as_bytes())?;
                buf_offset += env_str.len();
            }

            Ok(Errno::Success as i32)
        },
    )?;

    // args_sizes_get - return 0 args
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_sizes_get",
        |mut caller: Caller<TestStoreData>, count_ptr: i32, size_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            memory.write(&mut caller, count_ptr as usize, &0i32.to_le_bytes())?;
            memory.write(&mut caller, size_ptr as usize, &0i32.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // args_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_get",
        |_caller: Caller<TestStoreData>, _argv_ptr: i32, _argv_buf_ptr: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // clock_time_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_time_get",
        |mut caller: Caller<TestStoreData>, _clock_id: i32, _precision: i64, time_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            memory.write(&mut caller, time_ptr as usize, &now.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // clock_res_get - Get clock resolution
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_res_get",
        |mut caller: Caller<TestStoreData>, _clock_id: i32, resolution_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            // Return 1 nanosecond resolution
            let resolution: u64 = 1;
            memory.write(&mut caller, resolution_ptr as usize, &resolution.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // proc_exit
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "proc_exit",
        |_caller: Caller<TestStoreData>, code: i32| -> Result<()> {
            Err(wasmtime_wasi::I32Exit(code).into())
        },
    )?;

    // random_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "random_get",
        |mut caller: Caller<TestStoreData>, buf_ptr: i32, buf_len: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let random_bytes: Vec<u8> = (0..buf_len).map(|_| rand::random()).collect();
            memory.write(&mut caller, buf_ptr as usize, &random_bytes)?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_fdstat_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_get",
        |mut caller: Caller<TestStoreData>, fd: i32, fdstat_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let store_data = caller.data();

            let mut fdstat = [0u8; 24];

            match store_data.wasi_ctx.file_table.get(&(fd as u32)) {
                Some(TestFileHandle::File(_, _)) => {
                    fdstat[0] = 4; // filetype = regular file
                    // fs_rights_base - full permissions
                    fdstat[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
                }
                Some(TestFileHandle::Directory(_, _)) => {
                    fdstat[0] = 3; // filetype = directory
                    fdstat[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
                }
                Some(TestFileHandle::Stdout) | Some(TestFileHandle::Stderr) => {
                    fdstat[0] = 2; // filetype = character device
                    fdstat[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
                }
                Some(TestFileHandle::Stdin) => {
                    fdstat[0] = 2;
                    fdstat[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
                }
                None => return Ok(Errno::Badf as i32),
            }

            memory.write(&mut caller, fdstat_ptr as usize, &fdstat)?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_fdstat_set_flags (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_set_flags",
        |_caller: Caller<TestStoreData>, _fd: i32, _flags: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_filestat_set_size - Set file size (truncate/extend)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_filestat_set_size",
        |_caller: Caller<TestStoreData>, _fd: i32, _size: i64| -> Result<i32> {
            // For now, just return success (our in-memory files auto-resize on write)
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_sync - Sync file to storage
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_sync",
        |_caller: Caller<TestStoreData>, _fd: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_datasync - Sync file data to storage
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_datasync",
        |_caller: Caller<TestStoreData>, _fd: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_tell - Get current position
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_tell",
        |mut caller: Caller<TestStoreData>, fd: i32, offset_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let store_data = caller.data_mut();

            let pos = match store_data.wasi_ctx.file_table.get_mut(&(fd as u32)) {
                Some(TestFileHandle::File(ref mut file, _)) => {
                    file.seek(SeekFrom::Current(0)).unwrap_or(0)
                }
                _ => return Ok(Errno::Badf as i32),
            };

            memory.write(&mut caller, offset_ptr as usize, &pos.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_allocate - Allocate space (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_allocate",
        |_caller: Caller<TestStoreData>, _fd: i32, _offset: i64, _len: i64| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_advise - File advisory (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_advise",
        |_caller: Caller<TestStoreData>, _fd: i32, _offset: i64, _len: i64, _advice: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // path_rename - Rename file (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_rename",
        |_caller: Caller<TestStoreData>, _old_fd: i32, _old_path_ptr: i32, _old_path_len: i32, _new_fd: i32, _new_path_ptr: i32, _new_path_len: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // path_symlink - Create symlink (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_symlink",
        |_caller: Caller<TestStoreData>, _old_path_ptr: i32, _old_path_len: i32, _fd: i32, _new_path_ptr: i32, _new_path_len: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    // path_link - Create hard link (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_link",
        |_caller: Caller<TestStoreData>, _old_fd: i32, _old_flags: i32, _old_path_ptr: i32, _old_path_len: i32, _new_fd: i32, _new_path_ptr: i32, _new_path_len: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    // path_readlink - Read symlink (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_readlink",
        |_caller: Caller<TestStoreData>, _fd: i32, _path_ptr: i32, _path_len: i32, _buf_ptr: i32, _buf_len: i32, _bufused_ptr: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    // fd_renumber - Renumber fd (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_renumber",
        |_caller: Caller<TestStoreData>, _fd: i32, _to: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    // sock_* functions (stubs - not supported)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sock_accept",
        |_caller: Caller<TestStoreData>, _fd: i32, _flags: i32, _fd_ptr: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sock_recv",
        |_caller: Caller<TestStoreData>, _fd: i32, _ri_data_ptr: i32, _ri_data_len: i32, _ri_flags: i32, _ro_datalen_ptr: i32, _ro_flags_ptr: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sock_send",
        |_caller: Caller<TestStoreData>, _fd: i32, _si_data_ptr: i32, _si_data_len: i32, _si_flags: i32, _so_datalen_ptr: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sock_shutdown",
        |_caller: Caller<TestStoreData>, _fd: i32, _how: i32| -> Result<i32> {
            Ok(Errno::Nosys as i32)
        },
    )?;

    // path_filestat_set_times - Set file timestamps (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_filestat_set_times",
        |_caller: Caller<TestStoreData>, _fd: i32, _flags: i32, _path_ptr: i32, _path_len: i32, _atim: i64, _mtim: i64, _fst_flags: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_filestat_set_times - Set file timestamps via fd (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_filestat_set_times",
        |_caller: Caller<TestStoreData>, _fd: i32, _atim: i64, _mtim: i64, _fst_flags: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // path_filestat_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_filestat_get",
        |mut caller: Caller<TestStoreData>, dirfd: i32, _flags: i32, path_ptr: i32, path_len: i32, filestat_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let path = read_string(&caller, memory, path_ptr, path_len)?;

            let store_data = caller.data();
            let full_path = if dirfd == 3 {
                format!("/{}", path.trim_start_matches('/'))
            } else {
                return Ok(Errno::Badf as i32);
            };

            let mut filestat = [0u8; 64];

            if let Ok(file) = store_data.wasi_ctx.filesystem.open_file(&full_path) {
                let size = file.len() as u64;
                filestat[16] = 4; // filetype = regular file
                filestat[32..40].copy_from_slice(&size.to_le_bytes());
            } else if store_data.wasi_ctx.filesystem.get_dir(&full_path).is_ok() {
                filestat[16] = 3; // filetype = directory
            } else {
                return Ok(Errno::Noent as i32);
            }

            memory.write(&mut caller, filestat_ptr as usize, &filestat)?;
            Ok(Errno::Success as i32)
        },
    )?;

    // path_create_directory
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_create_directory",
        |mut caller: Caller<TestStoreData>, dirfd: i32, path_ptr: i32, path_len: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let path = read_string(&caller, memory, path_ptr, path_len)?;

            let store_data = caller.data_mut();
            let full_path = if dirfd == 3 {
                format!("/{}", path.trim_start_matches('/'))
            } else {
                return Ok(Errno::Badf as i32);
            };

            match store_data.wasi_ctx.filesystem.create_dir_all(&full_path) {
                Ok(_) => Ok(Errno::Success as i32),
                Err(_) => Ok(Errno::Io as i32),
            }
        },
    )?;

    // fd_readdir
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_readdir",
        |mut caller: Caller<TestStoreData>, fd: i32, buf_ptr: i32, buf_len: i32, cookie: i64, bufused_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let store_data = caller.data_mut();

            let dir = match store_data.wasi_ctx.file_table.get(&(fd as u32)) {
                Some(TestFileHandle::Directory(d, _)) => d.clone(),
                _ => return Ok(Errno::Badf as i32),
            };

            let entries = dir.list();
            let mut buf_offset = 0usize;
            let buf_size = buf_len as usize;

            for (i, (name, is_dir)) in entries.iter().enumerate() {
                if (i as i64) < cookie {
                    continue;
                }

                // dirent structure: d_next(8) + d_ino(8) + d_namlen(4) + d_type(1) + name
                let dirent_size = 24 + name.len();
                if buf_offset + dirent_size > buf_size {
                    break;
                }

                let mut dirent = vec![0u8; 24];
                dirent[0..8].copy_from_slice(&((i + 1) as u64).to_le_bytes()); // d_next
                dirent[8..16].copy_from_slice(&0u64.to_le_bytes()); // d_ino
                dirent[16..20].copy_from_slice(&(name.len() as u32).to_le_bytes()); // d_namlen
                dirent[20] = if *is_dir { 3 } else { 4 }; // d_type

                memory.write(&mut caller, (buf_ptr as usize) + buf_offset, &dirent)?;
                buf_offset += 24;
                memory.write(&mut caller, (buf_ptr as usize) + buf_offset, name.as_bytes())?;
                buf_offset += name.len();
            }

            memory.write(&mut caller, bufused_ptr as usize, &(buf_offset as u32).to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // path_remove_directory (stub)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_remove_directory",
        |_caller: Caller<TestStoreData>, _dirfd: i32, _path_ptr: i32, _path_len: i32| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // path_unlink_file
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_unlink_file",
        |mut caller: Caller<TestStoreData>, dirfd: i32, path_ptr: i32, path_len: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            let path = read_string(&caller, memory, path_ptr, path_len)?;

            let _store_data = caller.data_mut();
            // For simplicity, just return success
            Ok(Errno::Success as i32)
        },
    )?;

    // sched_yield
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sched_yield",
        |_caller: Caller<TestStoreData>| -> Result<i32> {
            Ok(Errno::Success as i32)
        },
    )?;

    // poll_oneoff (stub - return immediately)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "poll_oneoff",
        |mut caller: Caller<TestStoreData>, _in_ptr: i32, out_ptr: i32, nsubscriptions: i32, nevents_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;
            // Return 0 events
            memory.write(&mut caller, nevents_ptr as usize, &0i32.to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_pwrite
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_pwrite",
        |mut caller: Caller<TestStoreData>, fd: i32, iovs_ptr: i32, iovs_len: i32, offset: i64, nwritten_ptr: i32| -> Result<i32> {
            // Simplified: just do a regular write (ignoring offset for now)
            let memory = get_memory(&mut caller)?;

            let mut all_data = Vec::new();
            for i in 0..iovs_len {
                let iov_ptr = iovs_ptr + (i * 8);
                let mut iov_buf = [0u8; 8];
                memory.read(&caller, iov_ptr as usize, &mut iov_buf)?;

                let buf_ptr = u32::from_le_bytes([iov_buf[0], iov_buf[1], iov_buf[2], iov_buf[3]]);
                let buf_len = u32::from_le_bytes([iov_buf[4], iov_buf[5], iov_buf[6], iov_buf[7]]);

                let mut buf = vec![0u8; buf_len as usize];
                memory.read(&caller, buf_ptr as usize, &mut buf)?;
                all_data.extend_from_slice(&buf);
            }

            let nwritten = all_data.len();
            let store_data = caller.data_mut();

            if let Some(TestFileHandle::File(ref mut file, _)) = store_data.wasi_ctx.file_table.get_mut(&(fd as u32)) {
                let _ = file.seek(SeekFrom::Start(offset as u64));
                let _ = file.write_all(&all_data);
            }

            memory.write(&mut caller, nwritten_ptr as usize, &(nwritten as i32).to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // fd_pread
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_pread",
        |mut caller: Caller<TestStoreData>, fd: i32, iovs_ptr: i32, iovs_len: i32, offset: i64, nread_ptr: i32| -> Result<i32> {
            let memory = get_memory(&mut caller)?;

            let mut iov_info = Vec::new();
            for i in 0..iovs_len {
                let iov_ptr = iovs_ptr + (i * 8);
                let mut iov_buf = [0u8; 8];
                memory.read(&caller, iov_ptr as usize, &mut iov_buf)?;

                let buf_ptr = u32::from_le_bytes([iov_buf[0], iov_buf[1], iov_buf[2], iov_buf[3]]);
                let buf_len = u32::from_le_bytes([iov_buf[4], iov_buf[5], iov_buf[6], iov_buf[7]]);
                iov_info.push((buf_ptr, buf_len));
            }

            let store_data = caller.data_mut();

            let mut total_read = 0usize;
            let mut read_data = Vec::new();

            if let Some(TestFileHandle::File(ref mut file, _)) = store_data.wasi_ctx.file_table.get_mut(&(fd as u32)) {
                let _ = file.seek(SeekFrom::Start(offset as u64));
                let total_len: usize = iov_info.iter().map(|(_, len)| *len as usize).sum();
                read_data.resize(total_len, 0);
                total_read = file.read(&mut read_data).unwrap_or(0);
                read_data.truncate(total_read);
            }

            let mut write_offset = 0;
            for (buf_ptr, buf_len) in iov_info {
                let to_write = (total_read - write_offset).min(buf_len as usize);
                if to_write > 0 && write_offset < read_data.len() {
                    memory.write(&mut caller, buf_ptr as usize, &read_data[write_offset..write_offset + to_write])?;
                    write_offset += to_write;
                }
            }

            memory.write(&mut caller, nread_ptr as usize, &(total_read as i32).to_le_bytes())?;
            Ok(Errno::Success as i32)
        },
    )?;

    // Add soft-float intrinsics (same as main wasm.rs)
    // __floatunditf
    linker.func_wrap(
        "env",
        "__floatunditf",
        |mut caller: Caller<TestStoreData>, outptr: i32, _value: i64| {
            let memory = caller.get_export("memory").and_then(|e| e.into_memory());
            if let Some(mem) = memory {
                let _ = mem.write(&mut caller, outptr as usize, &[0u8; 16]);
            }
        },
    )?;

    // __floatditf
    linker.func_wrap(
        "env",
        "__floatditf",
        |mut caller: Caller<TestStoreData>, outptr: i32, _value: i64| {
            let memory = caller.get_export("memory").and_then(|e| e.into_memory());
            if let Some(mem) = memory {
                let _ = mem.write(&mut caller, outptr as usize, &[0u8; 16]);
            }
        },
    )?;

    // __trunctfdf2
    linker.func_wrap(
        "env",
        "__trunctfdf2",
        |_caller: Caller<TestStoreData>, _low: i64, _high: i64| -> f64 {
            0.0
        },
    )?;

    // __extenddftf2
    linker.func_wrap(
        "env",
        "__extenddftf2",
        |mut caller: Caller<TestStoreData>, outptr: i32, _value: f64| {
            let memory = caller.get_export("memory").and_then(|e| e.into_memory());
            if let Some(mem) = memory {
                let _ = mem.write(&mut caller, outptr as usize, &[0u8; 16]);
            }
        },
    )?;

    // Comparison functions
    for name in ["__letf2", "__getf2", "__unordtf2", "__eqtf2", "__netf2", "__lttf2", "__gttf2"] {
        linker.func_wrap(
            "env",
            name,
            |_caller: Caller<TestStoreData>, _a_low: i64, _a_high: i64, _b_low: i64, _b_high: i64| -> i32 {
                0
            },
        )?;
    }

    // Arithmetic functions
    for name in ["__multf3", "__addtf3", "__subtf3", "__divtf3"] {
        linker.func_wrap(
            "env",
            name,
            |_caller: Caller<TestStoreData>, _a_low: i64, _a_high: i64, _b_low: i64, _b_high: i64| -> (i64, i64) {
                (0i64, 0i64)
            },
        )?;
    }

    // Conversion functions
    linker.func_wrap(
        "env",
        "__fixtfdi",
        |_caller: Caller<TestStoreData>, _low: i64, _high: i64| -> i64 {
            0i64
        },
    )?;

    linker.func_wrap(
        "env",
        "__fixunstfdi",
        |_caller: Caller<TestStoreData>, _low: i64, _high: i64| -> i64 {
            0i64
        },
    )?;

    Ok(())
}
