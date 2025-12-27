use uuid::Uuid;
use crate::bindings_types::{Value, TableSchema};
use crate::shared_buffer::SharedBuffer;

pub struct ProcessingContext {
    pub content_uuid: Uuid,
    pub content_data: SharedBuffer,
    pub subcontent: Vec<SubContentEmission>,
    pub metadata: Vec<MetadataRow>,
    pub table_schemas: Vec<TableSchema>,
}

impl ProcessingContext {
    pub fn new(content_uuid: Uuid, content_data: SharedBuffer) -> Self {
        Self {
            content_uuid,
            content_data,
            subcontent: Vec::new(),
            metadata: Vec::new(),
            table_schemas: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.subcontent.clear();
        self.metadata.clear();
        self.table_schemas.clear();
    }
}

pub struct SubContentEmission {
    pub data: SubContentData,
    pub filename: String,
}

pub enum SubContentData {
    Bytes(Vec<u8>),
    Slice { offset: usize, length: usize },
}

pub struct MetadataRow {
    pub table_name: String,
    pub values: Vec<Value>,
}
