use uuid::Uuid;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use anyhow::Result;
use crate::shared_buffer::SharedBuffer;

#[derive(Debug, Clone)]
pub struct Content {
    pub uuid: Uuid,
    pub data: ContentData,
    pub filename: String,
    pub parent_uuid: Option<Uuid>,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub enum ContentData {
    Owned(SharedBuffer),
    Borrowed {
        parent_uuid: Uuid,
        offset: usize,
        length: usize,
    },
}

impl Content {
    pub fn new_root(buffer: SharedBuffer, filename: String) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            data: ContentData::Owned(buffer),
            filename,
            parent_uuid: None,
            depth: 0,
        }
    }

    pub fn new_subcontent(
        parent: &Content,
        data: ContentData,
        filename: String,
        max_depth: usize,
    ) -> Result<Self> {
        if parent.depth >= max_depth {
            anyhow::bail!("Max recursion depth exceeded (limit: {})", max_depth);
        }

        Ok(Self {
            uuid: Uuid::new_v4(),
            data,
            filename,
            parent_uuid: Some(parent.uuid),
            depth: parent.depth + 1,
        })
    }
}

pub struct ContentStore {
    store: Arc<RwLock<HashMap<Uuid, SharedBuffer>>>,
}

impl ContentStore {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, uuid: Uuid, buffer: SharedBuffer) {
        self.store.write().unwrap().insert(uuid, buffer);
    }

    pub fn get(&self, uuid: &Uuid) -> Option<SharedBuffer> {
        self.store.read().unwrap().get(uuid).cloned()
    }

    /// Resolve content to a SharedBuffer
    ///
    /// For owned content, returns a cheap clone of the buffer.
    /// For borrowed content, creates a zero-copy slice of the parent buffer.
    pub fn resolve(&self, content: &Content) -> Option<SharedBuffer> {
        match &content.data {
            ContentData::Owned(buffer) => Some(buffer.clone()),
            ContentData::Borrowed { parent_uuid, offset, length } => {
                let parent_buffer = self.get(parent_uuid)?;
                // Zero-copy slice via Bytes::slice()
                Some(parent_buffer.slice(*offset..*offset + *length))
            }
        }
    }
}

impl Clone for ContentStore {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}
