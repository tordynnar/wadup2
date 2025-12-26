use anyhow::Result;
use wasmtime::{Caller, Memory};
use crate::context::{ProcessingContext, SubContentEmission, SubContentData, MetadataRow};
use crate::types::{Column, Value};

// Helper function to get memory export
fn get_memory(caller: &mut Caller<ProcessingContext>) -> Result<Memory> {
    caller.get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("No memory export found"))
}

// Helper function to read string from WASM memory
fn read_string(
    caller: &mut Caller<ProcessingContext>,
    memory: Memory,
    ptr: i32,
    len: i32,
) -> Result<String> {
    if ptr < 0 || len < 0 {
        anyhow::bail!("Invalid pointer or length");
    }

    let mut buffer = vec![0u8; len as usize];
    memory.read(caller, ptr as usize, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

// Define a table with columns
pub fn define_table(
    mut caller: Caller<ProcessingContext>,
    name_ptr: i32,
    name_len: i32,
    columns_ptr: i32,
    columns_len: i32,
) -> Result<i32> {
    let memory = get_memory(&mut caller)?;
    let table_name = read_string(&mut caller, memory, name_ptr, name_len)?;
    let columns_json = read_string(&mut caller, memory, columns_ptr, columns_len)?;

    let columns: Vec<Column> = serde_json::from_str(&columns_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse columns: {}", e))?;

    // Store table schema in context so it can be created in the metadata store later
    use crate::types::TableSchema;
    caller.data_mut().table_schemas.push(TableSchema {
        name: table_name,
        columns,
    });

    Ok(0)
}

// Insert a row into a table
pub fn insert_row(
    mut caller: Caller<ProcessingContext>,
    table_name_ptr: i32,
    table_name_len: i32,
    row_data_ptr: i32,
    row_data_len: i32,
) -> Result<i32> {
    let memory = get_memory(&mut caller)?;
    let table_name = read_string(&mut caller, memory, table_name_ptr, table_name_len)?;
    let row_json = read_string(&mut caller, memory, row_data_ptr, row_data_len)?;

    let values: Vec<Value> = serde_json::from_str(&row_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse row data: {}", e))?;

    caller.data_mut().metadata.push(MetadataRow {
        table_name,
        values,
    });

    Ok(0)
}

// Emit sub-content from bytes
pub fn emit_subcontent_bytes(
    mut caller: Caller<ProcessingContext>,
    data_ptr: i32,
    data_len: i32,
    filename_ptr: i32,
    filename_len: i32,
) -> Result<i32> {
    if data_ptr < 0 || data_len < 0 {
        anyhow::bail!("Invalid data pointer or length");
    }

    let memory = get_memory(&mut caller)?;

    let mut data = vec![0u8; data_len as usize];
    memory.read(&caller, data_ptr as usize, &mut data)?;

    let filename = read_string(&mut caller, memory, filename_ptr, filename_len)?;

    caller.data_mut().subcontent.push(SubContentEmission {
        data: SubContentData::Bytes(data),
        filename,
    });

    Ok(0)
}

// Emit sub-content as a slice of current content
pub fn emit_subcontent_slice(
    mut caller: Caller<ProcessingContext>,
    offset: i32,
    length: i32,
    filename_ptr: i32,
    filename_len: i32,
) -> Result<i32> {
    if offset < 0 || length < 0 {
        anyhow::bail!("Invalid offset or length");
    }

    let content_size = caller.data().content_data.len();
    if (offset as usize + length as usize) > content_size {
        anyhow::bail!("Slice out of bounds");
    }

    let memory = get_memory(&mut caller)?;
    let filename = read_string(&mut caller, memory, filename_ptr, filename_len)?;

    caller.data_mut().subcontent.push(SubContentEmission {
        data: SubContentData::Slice {
            offset: offset as usize,
            length: length as usize,
        },
        filename,
    });

    Ok(0)
}

// Get the size of the current content
pub fn get_content_size(caller: Caller<ProcessingContext>) -> i32 {
    caller.data().content_data.len() as i32
}

// Read content into WASM memory
pub fn read_content(
    mut caller: Caller<ProcessingContext>,
    offset: i32,
    length: i32,
    dest_ptr: i32,
) -> Result<i32> {
    if offset < 0 || length < 0 || dest_ptr < 0 {
        anyhow::bail!("Invalid parameters");
    }

    let offset = offset as usize;
    let length = length as usize;

    // Clone the Arc to avoid borrowing issues
    let content = caller.data().content_data.clone();

    if offset + length > content.len() {
        anyhow::bail!("Read out of bounds");
    }

    let memory = get_memory(&mut caller)?;
    memory.write(&mut caller, dest_ptr as usize, &content[offset..offset+length])?;

    Ok(0)
}

// Get the current content UUID
pub fn get_content_uuid(
    mut caller: Caller<ProcessingContext>,
    dest_ptr: i32,
) -> Result<i32> {
    if dest_ptr < 0 {
        anyhow::bail!("Invalid pointer");
    }

    // Copy UUID bytes to avoid borrowing issues
    let uuid_bytes = *caller.data().content_uuid.as_bytes();
    let memory = get_memory(&mut caller)?;
    memory.write(&mut caller, dest_ptr as usize, &uuid_bytes)?;

    Ok(0)
}
