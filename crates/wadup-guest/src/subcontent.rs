//! File-based sub-content emission for WADUP.
//!
//! Emits sub-content for recursive processing by WADUP using files:
//! - `/subcontent/data_N.bin` - raw data bytes
//! - `/subcontent/metadata_N.json` - metadata (filename, optional offset/length)

use serde::Serialize;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;

thread_local! {
    static FILE_COUNTER: RefCell<usize> = RefCell::new(0);
}

fn next_counter() -> usize {
    FILE_COUNTER.with(|c| {
        let val = *c.borrow();
        *c.borrow_mut() = val + 1;
        val
    })
}

/// Metadata for sub-content with bytes.
#[derive(Serialize)]
struct SubContentMetadata {
    filename: String,
}

/// Metadata for sub-content slice (references input content).
#[derive(Serialize)]
struct SubContentSliceMetadata {
    filename: String,
    offset: usize,
    length: usize,
}

pub struct SubContent;

impl SubContent {
    /// Emit sub-content bytes for recursive processing.
    ///
    /// Writes data to `/subcontent/data_N.bin` and metadata to `/subcontent/metadata_N.json`.
    /// WADUP processes the sub-content when the metadata file is closed.
    pub fn emit_bytes(data: &[u8], filename: &str) -> Result<(), String> {
        let n = next_counter();
        let data_path = format!("/subcontent/data_{}.bin", n);
        let metadata_path = format!("/subcontent/metadata_{}.json", n);

        // Write data file first
        let mut data_file = File::create(&data_path)
            .map_err(|e| format!("Failed to create subcontent data file '{}': {}", data_path, e))?;
        data_file.write_all(data)
            .map_err(|e| format!("Failed to write subcontent data file '{}': {}", data_path, e))?;
        drop(data_file); // Close data file

        // Write metadata file (triggers processing when closed)
        let metadata = SubContentMetadata {
            filename: filename.to_string(),
        };
        let json = serde_json::to_string(&metadata)
            .map_err(|e| format!("Failed to serialize subcontent metadata: {}", e))?;

        let mut meta_file = File::create(&metadata_path)
            .map_err(|e| format!("Failed to create subcontent metadata file '{}': {}", metadata_path, e))?;
        meta_file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write subcontent metadata file '{}': {}", metadata_path, e))?;
        // File closed on drop, triggering WADUP processing

        Ok(())
    }

    /// Emit a slice of the input content as sub-content (zero-copy).
    ///
    /// The slice references a range of the original `/data.bin` content without copying.
    /// Only writes metadata to `/subcontent/metadata_N.json`.
    pub fn emit_slice(offset: usize, length: usize, filename: &str) -> Result<(), String> {
        let n = next_counter();
        let metadata_path = format!("/subcontent/metadata_{}.json", n);

        let metadata = SubContentSliceMetadata {
            filename: filename.to_string(),
            offset,
            length,
        };
        let json = serde_json::to_string(&metadata)
            .map_err(|e| format!("Failed to serialize subcontent slice metadata: {}", e))?;

        let mut meta_file = File::create(&metadata_path)
            .map_err(|e| format!("Failed to create subcontent metadata file '{}': {}", metadata_path, e))?;
        meta_file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write subcontent metadata file '{}': {}", metadata_path, e))?;
        // File closed on drop, triggering WADUP processing

        Ok(())
    }
}
