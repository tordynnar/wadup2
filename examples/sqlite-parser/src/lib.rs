use wadup_guest::*;
use std::io::Read;
use rusqlite::Connection;

#[no_mangle]
pub extern "C" fn process() -> i32 {
    if let Err(_) = run() {
        return 1;
    }
    0
}

fn run() -> Result<(), String> {
    // Check if this is a SQLite database by reading the header
    if !is_sqlite_database()? {
        return Ok(());
    }

    // Open the database directly from the virtual filesystem
    let conn = Connection::open(Content::path())
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // Query the database for table statistics
    let stats = execute_queries(&conn)?;

    // Define our metadata table
    let table = TableBuilder::new("db_table_stats")
        .column("table_name", DataType::String)
        .column("row_count", DataType::Int64)
        .build()?;

    // Insert statistics for each table
    for (table_name, row_count) in stats {
        table.insert(&[
            Value::String(table_name),
            Value::Int64(row_count),
        ])?;
    }

    // Flush metadata to file for WADUP to process
    flush()?;

    Ok(())
}

fn is_sqlite_database() -> Result<bool, String> {
    let mut file = std::fs::File::open(Content::path())
        .map_err(|e| format!("Failed to open content file: {}", e))?;

    let mut header = [0u8; 16];
    file.read_exact(&mut header)
        .map_err(|_| "File too small to be SQLite database".to_string())?;

    Ok(&header == b"SQLite format 3\0")
}

fn execute_queries(conn: &Connection) -> Result<Vec<(String, i64)>, String> {
    let mut stats = Vec::new();

    // Query for all user tables (excluding sqlite_* tables)
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    ).map_err(|e| format!("Failed to prepare table query: {}", e))?;

    let table_names: Vec<String> = stmt.query_map([], |row| row.get(0))
        .map_err(|e| format!("Failed to query tables: {}", e))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| format!("Failed to collect table names: {}", e))?;

    // For each table, count the rows
    for table_name in table_names {
        let count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM \"{}\"", table_name),
            [],
            |row| row.get(0)
        ).map_err(|e| format!("Failed to count rows in table {}: {}", table_name, e))?;

        stats.push((table_name, count));
    }

    Ok(stats)
}
