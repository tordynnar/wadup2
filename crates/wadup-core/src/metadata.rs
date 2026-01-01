use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use crate::bindings_types::{TableSchema, DataType, Value};

pub struct MetadataStore {
    conn: Arc<Mutex<Connection>>,
    schemas: Arc<Mutex<HashMap<String, TableSchema>>>,
}

impl MetadataStore {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;

        Self::init_tables(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            schemas: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn init_tables(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS __wadup_content (
                uuid TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                parent_uuid TEXT,
                processed_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT
            )",
            [],
        )?;

        // Table for capturing stdout/stderr from each module per content
        conn.execute(
            "CREATE TABLE IF NOT EXISTS __wadup_module_output (
                content_uuid TEXT NOT NULL,
                module_name TEXT NOT NULL,
                stdout TEXT,
                stderr TEXT,
                stdout_truncated INTEGER NOT NULL DEFAULT 0,
                stderr_truncated INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (content_uuid, module_name),
                FOREIGN KEY (content_uuid) REFERENCES __wadup_content(uuid)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn define_table(&self, schema: TableSchema) -> Result<()> {
        let mut schemas = self.schemas.lock().unwrap();

        // Check if table already defined
        if let Some(existing) = schemas.get(&schema.name) {
            self.validate_schema_match(existing, &schema)?;
            return Ok(()); // Already exists, schema matches
        }

        // Create table in SQLite
        let conn = self.conn.lock().unwrap();
        self.create_table(&conn, &schema)?;

        // Store schema
        schemas.insert(schema.name.clone(), schema);

        Ok(())
    }

    fn validate_schema_match(&self, existing: &TableSchema, new: &TableSchema) -> Result<()> {
        if existing.columns.len() != new.columns.len() {
            anyhow::bail!(
                "Schema mismatch for table '{}': different column count ({} vs {})",
                existing.name,
                existing.columns.len(),
                new.columns.len()
            );
        }

        for (existing_col, new_col) in existing.columns.iter().zip(&new.columns) {
            if existing_col.name != new_col.name {
                anyhow::bail!(
                    "Schema mismatch for table '{}': column name '{}' vs '{}'",
                    existing.name,
                    existing_col.name,
                    new_col.name
                );
            }
            if existing_col.data_type != new_col.data_type {
                anyhow::bail!(
                    "Schema mismatch for table '{}': column '{}' type mismatch",
                    existing.name,
                    existing_col.name
                );
            }
        }

        Ok(())
    }

    fn create_table(&self, conn: &Connection, schema: &TableSchema) -> Result<()> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (", schema.name);
        sql.push_str("content_uuid TEXT NOT NULL, ");

        for col in &schema.columns {
            let sql_type = match col.data_type {
                DataType::Int64 => "INTEGER",
                DataType::Float64 => "REAL",
                DataType::String => "TEXT",
            };
            sql.push_str(&format!("{} {}, ", col.name, sql_type));
        }

        sql.push_str("FOREIGN KEY(content_uuid) REFERENCES __wadup_content(uuid)");
        sql.push(')');

        conn.execute(&sql, [])?;

        Ok(())
    }

    pub fn insert_row(&self, table: &str, uuid: &str, values: &[Value]) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let placeholders: Vec<String> = (0..values.len() + 1).map(|_| "?".to_string()).collect();
        let sql = format!("INSERT INTO {} VALUES ({})", table, placeholders.join(", "));

        // Build rusqlite params
        let mut rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        rusqlite_params.push(Box::new(uuid.to_string()));

        for value in values {
            match value {
                Value::Int64(v) => rusqlite_params.push(Box::new(*v)),
                Value::Float64(v) => rusqlite_params.push(Box::new(*v)),
                Value::String(v) => rusqlite_params.push(Box::new(v.clone())),
            }
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = rusqlite_params.iter()
            .map(|p| p.as_ref())
            .collect();

        conn.execute(&sql, param_refs.as_slice())?;

        Ok(())
    }

    pub fn record_content_success(
        &self,
        uuid: &str,
        filename: &str,
        parent_uuid: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO __wadup_content
             (uuid, filename, parent_uuid, processed_at, status, error_message)
             VALUES (?1, ?2, ?3, ?4, 'success', NULL)",
            params![uuid, filename, parent_uuid, timestamp],
        )?;

        Ok(())
    }

    pub fn record_content_failure(
        &self,
        uuid: &str,
        filename: &str,
        parent_uuid: Option<&str>,
        error: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO __wadup_content
             (uuid, filename, parent_uuid, processed_at, status, error_message)
             VALUES (?1, ?2, ?3, ?4, 'failed', ?5)",
            params![uuid, filename, parent_uuid, timestamp, error],
        )?;

        Ok(())
    }

    /// Record stdout/stderr output from a module for a specific content.
    /// Only records if at least one of stdout or stderr is non-empty.
    pub fn record_module_output(
        &self,
        content_uuid: &str,
        module_name: &str,
        stdout: Option<&str>,
        stderr: Option<&str>,
        stdout_truncated: bool,
        stderr_truncated: bool,
    ) -> Result<()> {
        // Skip if nothing to record
        if stdout.is_none() && stderr.is_none() {
            return Ok(());
        }

        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO __wadup_module_output
             (content_uuid, module_name, stdout, stderr, stdout_truncated, stderr_truncated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                content_uuid,
                module_name,
                stdout,
                stderr,
                stdout_truncated as i32,
                stderr_truncated as i32
            ],
        )?;

        Ok(())
    }
}

impl Clone for MetadataStore {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            schemas: Arc::clone(&self.schemas),
        }
    }
}
