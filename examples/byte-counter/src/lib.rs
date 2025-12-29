use wadup_guest::*;

#[no_mangle]
pub extern "C" fn process() -> i32 {
    if let Err(e) = run() {
        // In a real implementation, we'd log this somehow
        // For now, just return error code
        return 1;
    }
    0
}

fn run() -> Result<(), String> {
    // Define our table
    let table = TableBuilder::new("file_sizes")
        .column("size_bytes", DataType::Int64)
        .build()?;

    // Get content size from the virtual filesystem
    let metadata = std::fs::metadata(Content::path())
        .map_err(|e| format!("Failed to get content metadata: {}", e))?;
    let size = metadata.len() as i64;

    // Insert the size
    table.insert(&[Value::Int64(size)])?;

    // Flush metadata to file for WADUP to process
    flush()?;

    Ok(())
}
