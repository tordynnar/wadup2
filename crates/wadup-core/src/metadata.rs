use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use serde::Serialize;
use chrono::{DateTime, Utc};
use crate::bindings_types::{TableSchema, Value};

/// Content metadata document
#[derive(Debug, Clone, Serialize)]
pub struct ContentDoc {
    pub doc_type: &'static str,
    pub content_uuid: String,
    pub filename: String,
    pub parent_uuid: Option<String>,
    pub processed_at: DateTime<Utc>,
    pub status: String,
    pub error_message: Option<String>,
}

/// Module stdout/stderr output document
#[derive(Debug, Clone, Serialize)]
pub struct ModuleOutputDoc {
    pub doc_type: &'static str,
    pub content_uuid: String,
    pub module_name: String,
    pub processed_at: DateTime<Utc>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

/// Table row document with flattened column values
/// Fixed fields use underscore prefix to avoid conflicts with column names
#[derive(Debug, Clone, Serialize)]
pub struct RowDoc {
    pub doc_type: &'static str,
    pub content_uuid: String,
    #[serde(rename = "_module")]
    pub module_name: String,
    #[serde(rename = "_table")]
    pub table_name: String,
    pub processed_at: DateTime<Utc>,
    /// Column values flattened as key-value pairs
    #[serde(flatten)]
    pub columns: HashMap<String, String>,
}

/// Tracking state for content being processed
struct ContentState {
    filename: String,
    parent_uuid: Option<String>,
    current_module: Option<String>,
}

pub struct MetadataStore {
    es_url: String,
    es_index: String,
    client: reqwest::blocking::Client,
    /// Content state tracking, keyed by content UUID
    content_state: Arc<Mutex<HashMap<String, ContentState>>>,
    /// Table schemas, keyed by table name -> column names
    table_schemas: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl MetadataStore {
    pub fn new(es_url: &str, es_index: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        // Health check - verify Elasticsearch is available
        let health_url = format!("{}/_cluster/health", es_url);
        match client.get(&health_url).send() {
            Ok(response) => {
                if !response.status().is_success() {
                    anyhow::bail!(
                        "Elasticsearch health check failed: HTTP {}",
                        response.status()
                    );
                }
            }
            Err(e) => {
                anyhow::bail!(
                    "Failed to connect to Elasticsearch at {}: {}",
                    es_url,
                    e
                );
            }
        }

        Ok(Self {
            es_url: es_url.to_string(),
            es_index: es_index.to_string(),
            client,
            content_state: Arc::new(Mutex::new(HashMap::new())),
            table_schemas: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create a dummy MetadataStore for test mode (no Elasticsearch connection).
    pub fn new_dummy() -> Self {
        Self {
            es_url: String::new(),
            es_index: String::new(),
            client: reqwest::blocking::Client::new(),
            content_state: Arc::new(Mutex::new(HashMap::new())),
            table_schemas: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start tracking a new content item
    pub fn start_content(
        &self,
        uuid: &str,
        filename: &str,
        parent_uuid: Option<&str>,
    ) -> Result<()> {
        let mut state = self.content_state.lock().unwrap();
        state.insert(uuid.to_string(), ContentState {
            filename: filename.to_string(),
            parent_uuid: parent_uuid.map(|s| s.to_string()),
            current_module: None,
        });
        Ok(())
    }

    /// Set the current module context for subsequent operations
    pub fn set_current_module(&self, uuid: &str, module_name: &str) -> Result<()> {
        let mut state = self.content_state.lock().unwrap();
        if let Some(content) = state.get_mut(uuid) {
            content.current_module = Some(module_name.to_string());
        }
        Ok(())
    }

    /// Define a table schema - stores column names for flattening row values
    pub fn define_table(&self, schema: TableSchema) -> Result<()> {
        let column_names: Vec<String> = schema.columns.iter()
            .map(|c| c.name.clone())
            .collect();
        let mut schemas = self.table_schemas.lock().unwrap();
        schemas.insert(schema.name, column_names);
        Ok(())
    }

    /// Insert a row - POSTs a RowDoc immediately with flattened column values
    pub fn insert_row(&self, table: &str, uuid: &str, values: &[Value]) -> Result<()> {
        let module_name = {
            let state = self.content_state.lock().unwrap();
            state.get(uuid)
                .and_then(|s| s.current_module.clone())
                .ok_or_else(|| anyhow::anyhow!("No current module set for content {}", uuid))?
        };

        // Get column names from schema
        let column_names = {
            let schemas = self.table_schemas.lock().unwrap();
            schemas.get(table).cloned()
                .ok_or_else(|| anyhow::anyhow!("No schema defined for table {}", table))?
        };

        // Build flattened column map
        let mut columns = HashMap::new();
        for (i, value) in values.iter().enumerate() {
            if let Some(col_name) = column_names.get(i) {
                let string_value = match value {
                    Value::Int64(i) => i.to_string(),
                    Value::Float64(f) => f.to_string(),
                    Value::String(s) => s.clone(),
                };
                columns.insert(col_name.clone(), string_value);
            }
        }

        let doc = RowDoc {
            doc_type: "row",
            content_uuid: uuid.to_string(),
            module_name,
            table_name: table.to_string(),
            processed_at: Utc::now(),
            columns,
        };

        // POST without explicit ID - let ES generate one
        self.post_document_auto_id(&doc)?;

        Ok(())
    }

    /// Record module stdout/stderr - POSTs a ModuleOutputDoc immediately
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

        let doc = ModuleOutputDoc {
            doc_type: "module_output",
            content_uuid: content_uuid.to_string(),
            module_name: module_name.to_string(),
            processed_at: Utc::now(),
            stdout: stdout.map(|s| s.to_string()),
            stderr: stderr.map(|s| s.to_string()),
            stdout_truncated,
            stderr_truncated,
        };

        // Use content_uuid + module_name as ID
        let doc_id = format!("{}_{}", content_uuid, module_name);
        self.post_document_with_id(&doc, &doc_id)?;

        Ok(())
    }

    /// Finalize a successful content - POSTs the ContentDoc
    pub fn finalize_content_success(&self, uuid: &str) -> Result<()> {
        let (filename, parent_uuid) = {
            let mut state = self.content_state.lock().unwrap();
            if let Some(content) = state.remove(uuid) {
                (content.filename, content.parent_uuid)
            } else {
                return Ok(());
            }
        };

        let doc = ContentDoc {
            doc_type: "content",
            content_uuid: uuid.to_string(),
            filename,
            parent_uuid,
            processed_at: Utc::now(),
            status: "success".to_string(),
            error_message: None,
        };

        self.post_document_with_id(&doc, uuid)?;
        Ok(())
    }

    /// Finalize a failed content - POSTs the ContentDoc with error
    pub fn finalize_content_failure(&self, uuid: &str, error: &str) -> Result<()> {
        let (filename, parent_uuid) = {
            let mut state = self.content_state.lock().unwrap();
            if let Some(content) = state.remove(uuid) {
                (content.filename, content.parent_uuid)
            } else {
                // Content not started, create minimal doc
                ("unknown".to_string(), None)
            }
        };

        let doc = ContentDoc {
            doc_type: "content",
            content_uuid: uuid.to_string(),
            filename,
            parent_uuid,
            processed_at: Utc::now(),
            status: "failed".to_string(),
            error_message: Some(error.to_string()),
        };

        self.post_document_with_id(&doc, uuid)?;
        Ok(())
    }

    /// POST a document with auto-generated ID
    fn post_document_auto_id<T: Serialize>(&self, doc: &T) -> Result<()> {
        let url = format!("{}/{}/_doc", self.es_url, self.es_index);

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(doc)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Failed to index document: HTTP {} - {}", status, body);
        }

        Ok(())
    }

    /// POST a document with explicit ID
    fn post_document_with_id<T: Serialize>(&self, doc: &T, id: &str) -> Result<()> {
        let url = format!("{}/{}/_doc/{}", self.es_url, self.es_index, id);

        let response = self.client
            .put(&url)
            .header("Content-Type", "application/json")
            .json(doc)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Failed to index document {}: HTTP {} - {}", id, status, body);
        }

        Ok(())
    }

    // Legacy compatibility methods

    pub fn record_content_success(
        &self,
        uuid: &str,
        filename: &str,
        parent_uuid: Option<&str>,
    ) -> Result<()> {
        let state = self.content_state.lock().unwrap();
        if !state.contains_key(uuid) {
            drop(state);
            self.start_content(uuid, filename, parent_uuid)?;
        }
        Ok(())
    }

    pub fn record_content_failure(
        &self,
        uuid: &str,
        filename: &str,
        parent_uuid: Option<&str>,
        error: &str,
    ) -> Result<()> {
        {
            let state = self.content_state.lock().unwrap();
            if !state.contains_key(uuid) {
                drop(state);
                self.start_content(uuid, filename, parent_uuid)?;
            }
        }
        self.finalize_content_failure(uuid, error)
    }
}

impl Clone for MetadataStore {
    fn clone(&self) -> Self {
        Self {
            es_url: self.es_url.clone(),
            es_index: self.es_index.clone(),
            client: self.client.clone(),
            content_state: Arc::clone(&self.content_state),
            table_schemas: Arc::clone(&self.table_schemas),
        }
    }
}
