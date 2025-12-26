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

    // Get content size
    let size = Content::size() as i64;

    // Insert the size
    table.insert(&[Value::Int64(size)])?;

    Ok(())
}
