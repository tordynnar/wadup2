use uuid::Uuid;
use crate::bindings_types::{Value, TableSchema};
use crate::shared_buffer::SharedBuffer;

pub struct ProcessingContext {
    pub content_uuid: Uuid,
    pub content_data: SharedBuffer,
    pub subcontent: Vec<SubContentEmission>,
    pub metadata: Vec<MetadataRow>,
    pub table_schemas: Vec<TableSchema>,
    /// Captured stdout from module (None if empty)
    pub stdout: Option<String>,
    /// Captured stderr from module (None if empty)
    pub stderr: Option<String>,
    /// Whether stdout was truncated due to size limit
    pub stdout_truncated: bool,
    /// Whether stderr was truncated due to size limit
    pub stderr_truncated: bool,
}

impl ProcessingContext {
    pub fn new(content_uuid: Uuid, content_data: SharedBuffer) -> Self {
        Self {
            content_uuid,
            content_data,
            subcontent: Vec::new(),
            metadata: Vec::new(),
            table_schemas: Vec::new(),
            stdout: None,
            stderr: None,
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    pub fn clear(&mut self) {
        self.subcontent.clear();
        self.metadata.clear();
        self.table_schemas.clear();
        self.stdout = None;
        self.stderr = None;
        self.stdout_truncated = false;
        self.stderr_truncated = false;
    }
}

pub struct SubContentEmission {
    pub data: SubContentData,
    pub filename: String,
}

pub enum SubContentData {
    /// Owned bytes data (zero-copy: wraps bytes::Bytes directly)
    Bytes(bytes::Bytes),
    /// Slice of parent content (zero-copy reference)
    Slice { offset: usize, length: usize },
}

pub struct MetadataRow {
    pub table_name: String,
    pub values: Vec<Value>,
}
