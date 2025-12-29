//! File-based metadata writer for WADUP.
//!
//! Accumulates table definitions and rows in memory, then writes them
//! to `/metadata/output_N.json` files that WADUP processes on close.

use crate::types::{Column, Value};
use serde::Serialize;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;

/// Internal table definition for serialization.
#[derive(Serialize)]
struct TableDef {
    name: String,
    columns: Vec<Column>,
}

/// Internal row definition for serialization.
#[derive(Serialize)]
struct RowDef {
    table_name: String,
    values: Vec<Value>,
}

/// Metadata file structure matching WADUP's expected format.
#[derive(Serialize)]
struct MetadataFile {
    tables: Vec<TableDef>,
    rows: Vec<RowDef>,
}

thread_local! {
    static TABLES: RefCell<Vec<TableDef>> = RefCell::new(Vec::new());
    static ROWS: RefCell<Vec<RowDef>> = RefCell::new(Vec::new());
    static FILE_COUNTER: RefCell<usize> = RefCell::new(0);
}

/// Add a table definition to the accumulated metadata.
pub fn add_table(name: String, columns: Vec<Column>) {
    TABLES.with(|tables| {
        tables.borrow_mut().push(TableDef { name, columns });
    });
}

/// Add a row to the accumulated metadata.
pub fn add_row(table_name: String, values: Vec<Value>) {
    ROWS.with(|rows| {
        rows.borrow_mut().push(RowDef { table_name, values });
    });
}

/// Flush all accumulated metadata to a file.
///
/// Writes to `/metadata/output_N.json` where N is an incrementing counter.
/// The file is closed after writing, which triggers WADUP to read and process it.
///
/// Returns `Ok(())` if successful or if there's nothing to flush.
pub fn flush() -> Result<(), String> {
    let (tables, rows) = TABLES.with(|t| {
        ROWS.with(|r| {
            let tables = std::mem::take(&mut *t.borrow_mut());
            let rows = std::mem::take(&mut *r.borrow_mut());
            (tables, rows)
        })
    });

    // Nothing to flush
    if tables.is_empty() && rows.is_empty() {
        return Ok(());
    }

    let counter = FILE_COUNTER.with(|c| {
        let val = *c.borrow();
        *c.borrow_mut() = val + 1;
        val
    });

    let filename = format!("/metadata/output_{}.json", counter);

    let metadata = MetadataFile { tables, rows };
    let json = serde_json::to_string(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    let mut file = File::create(&filename)
        .map_err(|e| format!("Failed to create metadata file '{}': {}", filename, e))?;

    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write metadata file '{}': {}", filename, e))?;

    // File is closed when dropped, triggering WADUP to process it
    Ok(())
}
