//! WASM module precompilation and caching.
//!
//! This module provides functionality to cache compiled WASM modules for faster
//! subsequent loads. Cache files are stored alongside `.wasm` files with the
//! `_precompiled` suffix.
//!
//! Cache validity is determined by:
//! - Engine compatibility hash (ensures same wasmtime config)
//! - Source file modification time (detects source changes)

use anyhow::Result;
use std::fs::{self, File};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use wasmtime::{Engine, Module};

/// Compute a hash of the engine's precompile compatibility hash.
/// This changes when engine configuration changes in ways that affect code generation.
pub fn compute_engine_hash(engine: &Engine) -> u64 {
    let mut hasher = DefaultHasher::new();
    engine.precompile_compatibility_hash().hash(&mut hasher);
    hasher.finish()
}

/// Get the cache file path for a given WASM file.
/// Returns `{stem}_precompiled` in the same directory.
pub fn get_cache_path(wasm_path: &Path) -> PathBuf {
    let stem = wasm_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    wasm_path.with_file_name(format!("{}_precompiled", stem))
}

/// Get the modification time of a file as seconds since UNIX epoch.
pub fn get_file_mtime(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path)?;
    let mtime = metadata.modified()?;
    let duration = mtime
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Invalid mtime: {}", e))?;
    Ok(duration.as_secs())
}

/// Header stored at the beginning of cache files.
struct CacheHeader {
    engine_hash: u64,
    mtime: u64,
}

/// Read the header from a cache file.
fn read_cache_header(cache_path: &Path) -> Result<Option<CacheHeader>> {
    if !cache_path.exists() {
        return Ok(None);
    }

    let mut file = File::open(cache_path)?;
    let mut header = [0u8; 16];

    if file.read_exact(&mut header).is_err() {
        return Ok(None); // Corrupted/incomplete cache
    }

    let engine_hash = u64::from_le_bytes(header[0..8].try_into().unwrap());
    let mtime = u64::from_le_bytes(header[8..16].try_into().unwrap());

    Ok(Some(CacheHeader { engine_hash, mtime }))
}

/// Check if a cache file is valid for the current engine and source file.
pub fn is_cache_valid(cache_path: &Path, current_engine_hash: u64, current_mtime: u64) -> bool {
    match read_cache_header(cache_path) {
        Ok(Some(header)) => {
            header.engine_hash == current_engine_hash && header.mtime == current_mtime
        }
        _ => false,
    }
}

/// Write a precompiled module to the cache.
fn write_precompiled_cache(
    cache_path: &Path,
    engine_hash: u64,
    mtime: u64,
    serialized_module: &[u8],
) -> Result<()> {
    let mut file = File::create(cache_path)?;

    // Write header
    file.write_all(&engine_hash.to_le_bytes())?;
    file.write_all(&mtime.to_le_bytes())?;

    // Write serialized module
    file.write_all(serialized_module)?;

    Ok(())
}

/// Load a WASM module, using cache if available and valid.
///
/// If the cache is valid, deserializes the precompiled module.
/// If the cache is invalid or missing, compiles from source and writes cache.
pub fn load_module_with_cache(engine: &Engine, wasm_path: &Path) -> Result<Module> {
    let cache_path = get_cache_path(wasm_path);
    let engine_hash = compute_engine_hash(engine);
    let current_mtime = get_file_mtime(wasm_path)?;

    // Try loading from cache
    if is_cache_valid(&cache_path, engine_hash, current_mtime) {
        tracing::debug!("Loading precompiled module from cache: {:?}", cache_path);

        let cache_data = fs::read(&cache_path)?;
        if cache_data.len() > 16 {
            let serialized_data = &cache_data[16..]; // Skip header

            // SAFETY: We only deserialize data we serialized ourselves.
            // Cache validity is checked via engine hash and mtime.
            match unsafe { Module::deserialize(engine, serialized_data) } {
                Ok(module) => return Ok(module),
                Err(e) => {
                    tracing::warn!("Failed to deserialize cached module: {}", e);
                    // Fall through to recompile
                }
            }
        }
    }

    // Compile from source
    tracing::debug!("Compiling module from source: {:?}", wasm_path);
    let module = Module::from_file(engine, wasm_path)?;

    // Write to cache
    match module.serialize() {
        Ok(serialized) => {
            if let Err(e) =
                write_precompiled_cache(&cache_path, engine_hash, current_mtime, &serialized)
            {
                tracing::warn!("Failed to write precompiled cache: {}", e);
            } else {
                tracing::debug!("Wrote precompiled cache: {:?}", cache_path);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to serialize module: {}", e);
        }
    }

    Ok(module)
}
