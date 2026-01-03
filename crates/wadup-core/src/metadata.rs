use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::bindings_types::{TableSchema, Value};

/// Output from a single module processing a piece of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    /// Tables emitted by this module, keyed by table name
    /// Each table contains rows, where each row is a list of values
    pub tables: HashMap<String, Vec<Vec<serde_json::Value>>>,
}

impl Default for ModuleOutput {
    fn default() -> Self {
        Self {
            stdout: None,
            stderr: None,
            stdout_truncated: false,
            stderr_truncated: false,
            tables: HashMap::new(),
        }
    }
}

/// Document representing a single piece of content and all its processing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDocument {
    pub uuid: String,
    pub filename: String,
    pub parent_uuid: Option<String>,
    pub processed_at: DateTime<Utc>,
    pub status: String,
    pub error_message: Option<String>,
    /// Module outputs keyed by module name
    pub modules: HashMap<String, ModuleOutput>,
}

impl ContentDocument {
    fn new(uuid: String, filename: String, parent_uuid: Option<String>) -> Self {
        Self {
            uuid,
            filename,
            parent_uuid,
            processed_at: Utc::now(),
            status: "processing".to_string(),
            error_message: None,
            modules: HashMap::new(),
        }
    }
}

/// Internal state for accumulating document data
struct DocumentState {
    document: ContentDocument,
    current_module: Option<String>,
}

pub struct MetadataStore {
    es_url: String,
    es_index: String,
    client: reqwest::blocking::Client,
    /// Documents being accumulated, keyed by content UUID
    documents: Arc<Mutex<HashMap<String, DocumentState>>>,
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
            documents: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Start accumulating data for a new content item
    pub fn start_content(
        &self,
        uuid: &str,
        filename: &str,
        parent_uuid: Option<&str>,
    ) -> Result<()> {
        let mut documents = self.documents.lock().unwrap();
        let state = DocumentState {
            document: ContentDocument::new(
                uuid.to_string(),
                filename.to_string(),
                parent_uuid.map(|s| s.to_string()),
            ),
            current_module: None,
        };
        documents.insert(uuid.to_string(), state);
        Ok(())
    }

    /// Set the current module context for subsequent insert_row calls
    pub fn set_current_module(&self, uuid: &str, module_name: &str) -> Result<()> {
        let mut documents = self.documents.lock().unwrap();
        if let Some(state) = documents.get_mut(uuid) {
            state.current_module = Some(module_name.to_string());
            // Ensure module output exists
            if !state.document.modules.contains_key(module_name) {
                state.document.modules.insert(
                    module_name.to_string(),
                    ModuleOutput::default(),
                );
            }
        }
        Ok(())
    }

    /// Define a table schema (no-op for Elasticsearch - schema-less)
    pub fn define_table(&self, _schema: TableSchema) -> Result<()> {
        // Elasticsearch is schema-less, no action needed
        Ok(())
    }

    /// Insert a row into a table for the current content/module
    pub fn insert_row(&self, table: &str, uuid: &str, values: &[Value]) -> Result<()> {
        let mut documents = self.documents.lock().unwrap();

        if let Some(state) = documents.get_mut(uuid) {
            let module_name = state.current_module.clone()
                .ok_or_else(|| anyhow::anyhow!("No current module set for content {}", uuid))?;

            // Convert values to JSON - always use strings to avoid ES mapping conflicts
            let json_values: Vec<serde_json::Value> = values
                .iter()
                .map(|v| match v {
                    Value::Int64(i) => serde_json::Value::String(i.to_string()),
                    Value::Float64(f) => serde_json::Value::String(f.to_string()),
                    Value::String(s) => serde_json::Value::String(s.clone()),
                })
                .collect();

            // Get or create module output
            let module_output = state.document.modules
                .entry(module_name)
                .or_insert_with(ModuleOutput::default);

            // Get or create table
            let table_rows = module_output.tables
                .entry(table.to_string())
                .or_insert_with(Vec::new);

            table_rows.push(json_values);
        }

        Ok(())
    }

    /// Record module stdout/stderr output
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

        let mut documents = self.documents.lock().unwrap();

        if let Some(state) = documents.get_mut(content_uuid) {
            let module_output = state.document.modules
                .entry(module_name.to_string())
                .or_insert_with(ModuleOutput::default);

            module_output.stdout = stdout.map(|s| s.to_string());
            module_output.stderr = stderr.map(|s| s.to_string());
            module_output.stdout_truncated = stdout_truncated;
            module_output.stderr_truncated = stderr_truncated;
        }

        Ok(())
    }

    /// Finalize and POST a successful content document to Elasticsearch
    pub fn finalize_content_success(&self, uuid: &str) -> Result<()> {
        let document = {
            let mut documents = self.documents.lock().unwrap();
            if let Some(mut state) = documents.remove(uuid) {
                state.document.status = "success".to_string();
                state.document.processed_at = Utc::now();
                Some(state.document)
            } else {
                None
            }
        };

        if let Some(doc) = document {
            self.post_document(&doc)?;
        }

        Ok(())
    }

    /// Finalize and POST a failed content document to Elasticsearch
    pub fn finalize_content_failure(&self, uuid: &str, error: &str) -> Result<()> {
        let document = {
            let mut documents = self.documents.lock().unwrap();
            if let Some(mut state) = documents.remove(uuid) {
                state.document.status = "failed".to_string();
                state.document.error_message = Some(error.to_string());
                state.document.processed_at = Utc::now();
                Some(state.document)
            } else {
                None
            }
        };

        if let Some(doc) = document {
            self.post_document(&doc)?;
        }

        Ok(())
    }

    /// POST a document to Elasticsearch
    fn post_document(&self, document: &ContentDocument) -> Result<()> {
        let url = format!("{}/{}/_doc/{}", self.es_url, self.es_index, document.uuid);

        let response = self.client
            .put(&url)
            .header("Content-Type", "application/json")
            .json(document)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!(
                "Failed to index document {}: HTTP {} - {}",
                document.uuid,
                status,
                body
            );
        }

        tracing::debug!("Indexed document {} to Elasticsearch", document.uuid);
        Ok(())
    }

    /// Legacy methods for backward compatibility during refactoring
    /// These will be removed once processor.rs is updated

    pub fn record_content_success(
        &self,
        uuid: &str,
        filename: &str,
        parent_uuid: Option<&str>,
    ) -> Result<()> {
        // Start content if not already started
        {
            let documents = self.documents.lock().unwrap();
            if !documents.contains_key(uuid) {
                drop(documents);
                self.start_content(uuid, filename, parent_uuid)?;
            }
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
        // Start content if not already started
        {
            let documents = self.documents.lock().unwrap();
            if !documents.contains_key(uuid) {
                drop(documents);
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
            documents: Arc::clone(&self.documents),
        }
    }
}
