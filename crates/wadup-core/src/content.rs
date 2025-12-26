use uuid::Uuid;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use anyhow::Result;

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
    Owned(Arc<Vec<u8>>),
    Borrowed {
        parent_uuid: Uuid,
        offset: usize,
        length: usize,
    },
}

impl Content {
    pub fn new_root(data: Vec<u8>, filename: String) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            data: ContentData::Owned(Arc::new(data)),
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
    store: Arc<RwLock<HashMap<Uuid, Arc<Vec<u8>>>>>,
}

impl ContentStore {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, uuid: Uuid, data: Arc<Vec<u8>>) {
        self.store.write().unwrap().insert(uuid, data);
    }

    pub fn get(&self, uuid: &Uuid) -> Option<Arc<Vec<u8>>> {
        self.store.read().unwrap().get(uuid).cloned()
    }

    pub fn resolve(&self, content: &Content) -> Option<Arc<Vec<u8>>> {
        match &content.data {
            ContentData::Owned(data) => Some(data.clone()),
            ContentData::Borrowed { parent_uuid, offset, length } => {
                let parent_data = self.get(parent_uuid)?;
                let slice = parent_data[*offset..*offset+*length].to_vec();
                Some(Arc::new(slice))
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
