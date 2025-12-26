use wadup_guest::*;
use std::io::{Read, Seek, SeekFrom};

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
        // Not a SQLite database, skip processing
        return Ok(());
    }

    // Parse the database and count tables/rows
    let stats = parse_sqlite_stats(&mut reader)?;

    // Define our metadata table
    let table = TableBuilder::new("db_table_stats")
        .column("table_name", DataType::String)
        .column("row_count", DataType::Int64)
        .build()?;

    // Insert statistics for each table
    for (table_name, row_count) in stats {
        table.insert(&[
            Value::String(table_name),
            Value::Int64(row_count as i64),
        ])?;
    }

    Ok(())
}

fn is_sqlite_database(reader: &mut ContentReader) -> Result<bool, String> {
    // SQLite databases start with "SQLite format 3\0"
    reader.seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek: {}", e))?;

    let mut header = [0u8; 16];
    reader.read_exact(&mut header)
        .map_err(|e| format!("Failed to read header: {}", e))?;

    Ok(&header == b"SQLite format 3\0")
}

fn parse_sqlite_stats(reader: &mut ContentReader) -> Result<Vec<(String, usize)>, String> {
    // For a real implementation, we would need a full SQLite parser
    // For this test, we'll use a simplified approach:
    // - Read the entire database into memory
    // - Use basic pattern matching to find table definitions
    // - Estimate row counts based on page analysis

    // For now, return mock data to demonstrate the concept
    // In a production version, you would use a proper SQLite parser library

    // Read all content
    reader.seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek: {}", e))?;

    let mut data = Vec::new();
    reader.read_to_end(&mut data)
        .map_err(|e| format!("Failed to read content: {}", e))?;

    // Simple heuristic: look for "CREATE TABLE" statements in the data
    let content_str = String::from_utf8_lossy(&data);
    let mut tables = Vec::new();

    // This is a very simplified parser that looks for table names
    // In reality, you'd want to properly parse the sqlite_master table
    for line in content_str.lines() {
        if line.contains("CREATE TABLE") {
            // Extract table name (very simplified)
            if let Some(start) = line.find("TABLE ") {
                let rest = &line[start + 6..];
                if let Some(name_end) = rest.find(|c: char| c.is_whitespace() || c == '(') {
                    let table_name = rest[..name_end].trim().to_string();
                    // Mock row count for demonstration
                    tables.push((table_name, 10));
                }
            }
        }
    }

    // If we couldn't find any tables, return a placeholder
    if tables.is_empty() {
        tables.push(("unknown".to_string(), 0));
    }

    Ok(tables)
}
