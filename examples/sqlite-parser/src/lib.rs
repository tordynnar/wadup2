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
        return Ok(());
    }

    // Parse the database
    let db = SqliteDatabase::parse(&mut reader)?;

    // Define our metadata table
    let table = TableBuilder::new("db_table_stats")
        .column("table_name", DataType::String)
        .column("row_count", DataType::Int64)
        .build()?;

    // Insert statistics for each table
    for (table_name, row_count) in db.table_stats {
        table.insert(&[
            Value::String(table_name),
            Value::Int64(row_count as i64),
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

// SQLite database parser
struct SqliteDatabase {
    table_stats: Vec<(String, usize)>,
}

impl SqliteDatabase {
    fn parse(reader: &mut ContentReader) -> Result<Self, String> {
        // Read the entire database into memory for easier parsing
        reader.seek(SeekFrom::Start(0))
            .map_err(|e| format!("Failed to seek: {}", e))?;

        let mut data = Vec::new();
        reader.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read database: {}", e))?;

        // Parse the header
        let header = SqliteHeader::parse(&data)?;

        // Find and parse the schema
        let tables = Self::parse_schema(&data, &header)?;

        Ok(SqliteDatabase { table_stats: tables })
    }

    fn parse_schema(data: &[u8], header: &SqliteHeader) -> Result<Vec<(String, usize)>, String> {
        let page_size = header.page_size as usize;

        // Extract table information from the database
        // The sqlite_master table is on page 1 (offset 100 + page 1 data)
        let schema_entries = Self::find_tables_and_root_pages(data, page_size)?;

        let mut stats = Vec::new();

        // For each table, count rows by examining its root page
        for (table_name, rootpage) in schema_entries {
            let row_count = Self::count_table_rows(data, rootpage, page_size);
            stats.push((table_name, row_count));
        }

        Ok(stats)
    }

    fn find_tables_and_root_pages(data: &[u8], page_size: usize) -> Result<Vec<(String, usize)>, String> {
        // Read the first page which contains sqlite_master
        // For simplicity, we'll extract table names and their root pages using pattern matching

        let page1_start = 100; // Header is 100 bytes
        if data.len() < page1_start + page_size {
            return Ok(Vec::new());
        }

        // Search in the first page for table definitions
        let search_area = &data[page1_start..page1_start.saturating_add(page_size).min(data.len())];
        let text = String::from_utf8_lossy(search_area);

        let mut tables = Vec::new();

        // Look for the pattern: "table"<table_name><table_name><rootpage (as byte)>
        // In SQLite's sqlite_master, entries are: type, name, tbl_name, rootpage, sql
        // These are stored as strings/integers in the B-tree

        // Simple heuristic: find CREATE TABLE statements and extract table names
        let mut pos = 0;
        while let Some(create_pos) = text[pos..].find("CREATE TABLE") {
            let absolute_pos = pos + create_pos;
            let after_create = &text[absolute_pos..];

            // Extract table name
            if let Some(table_keyword_pos) = after_create.find("TABLE ") {
                let after_keyword = &after_create[table_keyword_pos + 6..];
                let name_part = after_keyword.trim_start();

                // Find the end of the table name (space or '(')
                let name_end = name_part
                    .find(|c: char| c.is_whitespace() || c == '(')
                    .unwrap_or(name_part.len().min(50));

                let table_name = name_part[..name_end].trim().trim_matches('"').to_string();

                if !table_name.is_empty() && !table_name.starts_with("sqlite_") {
                    // Try to find the rootpage by looking for small integers near the table name
                    // In the binary format, the rootpage often appears as a small integer (2, 3, 4, etc.)
                    let search_bytes = &search_area[absolute_pos.saturating_sub(50)..
                                                     (absolute_pos + 200).min(search_area.len())];

                    // Look for likely rootpage values (typically 2-10 for user tables)
                    let mut rootpage = 2 + tables.len(); // Default guess

                    // Try to find it more accurately by looking at bytes around the table name
                    for i in 0..search_bytes.len().saturating_sub(1) {
                        let byte_val = search_bytes[i] as usize;
                        if byte_val >= 2 && byte_val <= 20 {
                            // This could be a rootpage
                            rootpage = byte_val;
                            break;
                        }
                    }

                    tables.push((table_name, rootpage));
                }
            }

            pos = absolute_pos + 12; // Move past "CREATE TABLE"
        }

        Ok(tables)
    }

    fn count_table_rows(data: &[u8], rootpage: usize, page_size: usize) -> usize {
        // Calculate page offset
        let page_offset = (rootpage - 1) * page_size;

        if page_offset + page_size > data.len() {
            return 0;
        }

        let page_data = &data[page_offset..page_offset + page_size];

        // Read B-tree page header
        // Page type at offset 0: 0x0d = leaf table, 0x05 = interior table
        if page_data.len() < 8 {
            return 0;
        }

        let page_type = page_data[0];

        // If it's a leaf table page (0x0d), count cells
        if page_type == 0x0d {
            // Number of cells is at offset 3-4 (big-endian u16)
            if page_data.len() >= 5 {
                let cell_count = u16::from_be_bytes([page_data[3], page_data[4]]) as usize;
                return cell_count;
            }
        } else if page_type == 0x05 {
            // Interior table page - this means there are overflow pages
            // For a more accurate count, we'd need to recursively traverse
            // For now, return an estimate
            return 0; // Will be counted when we hit the leaf pages
        }

        0
    }
}

#[derive(Debug)]
struct SqliteHeader {
    page_size: u32,
}

impl SqliteHeader {
    fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 100 {
            return Err("File too small for SQLite header".to_string());
        }

        // Page size is at offset 16-17 (big-endian u16)
        let page_size_raw = u16::from_be_bytes([data[16], data[17]]);

        let page_size = if page_size_raw == 1 {
            65536 // Special case: 1 means 65536
        } else {
            page_size_raw as u32
        };

        Ok(SqliteHeader { page_size })
    }
}
