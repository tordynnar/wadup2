use wadup_guest::*;
use std::io::{Read, Seek, SeekFrom};
use rusqlite::Connection;

#[no_mangle]
pub extern "C" fn process() -> i32 {
    if let Err(_) = run() {
        return 1;
    }
    0
}

fn run() -> Result<(), String> {
    let mut reader = Content::reader();

    // Check if this is a SQLite database
    if !is_sqlite_database(&mut reader)? {
        return Ok(());
    }

    // Read entire database into memory
    reader.seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek: {}", e))?;

    let mut db_bytes = Vec::new();
    reader.read_to_end(&mut db_bytes)
        .map_err(|e| format!("Failed to read database: {}", e))?;

    // Query the database using rusqlite
    let stats = query_database(&db_bytes)?;

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

    Ok(())
}

fn is_sqlite_database(reader: &mut ContentReader) -> Result<bool, String> {
    reader.seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek: {}", e))?;

    let mut header = [0u8; 16];
    reader.read_exact(&mut header)
        .map_err(|_| "File too small to be SQLite database".to_string())?;

    Ok(&header == b"SQLite format 3\0")
}

fn query_database(db_bytes: &[u8]) -> Result<Vec<(String, i64)>, String> {
    // For WASM with WASI, we can write the bytes to a temporary file and open it
    use std::io::Write;

    // Write to a temp file
    let temp_path = "/tmp/temp_db.sqlite";
    let mut file = std::fs::File::create(temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    file.write_all(db_bytes)
        .map_err(|e| format!("Failed to write database to temp file: {}", e))?;

    drop(file); // Close the file

    // Open the database
    let conn = Connection::open(temp_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let result = execute_queries(&conn);

    // Clean up
    let _ = std::fs::remove_file(temp_path);

    result
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
