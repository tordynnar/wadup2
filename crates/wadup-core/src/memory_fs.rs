use std::collections::HashMap;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::BytesMut;

/// In-memory file with Read/Write/Seek support
#[derive(Clone)]
pub struct MemoryFile {
    data: Arc<RwLock<BytesMut>>,
    position: Arc<RwLock<usize>>,
}

impl MemoryFile {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(BytesMut::new())),
            position: Arc::new(RwLock::new(0)),
        }
    }

    pub fn with_data(data: Vec<u8>) -> Self {
        Self {
            data: Arc::new(RwLock::new(BytesMut::from(&data[..]))),
            position: Arc::new(RwLock::new(0)),
        }
    }

    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }
}

impl Read for MemoryFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data = self.data.read();
        let mut pos = self.position.write();

        if *pos >= data.len() {
            return Ok(0);
        }

        let available = data.len() - *pos;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&data[*pos..*pos + to_read]);
        *pos += to_read;

        Ok(to_read)
    }
}

impl Write for MemoryFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut data = self.data.write();
        let mut pos = self.position.write();

        // Extend if writing past end
        if *pos + buf.len() > data.len() {
            data.resize(*pos + buf.len(), 0);
        }

        data[*pos..*pos + buf.len()].copy_from_slice(buf);
        *pos += buf.len();

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for MemoryFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let data_len = self.data.read().len();
        let mut position = self.position.write();

        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => *position as i64 + offset,
            SeekFrom::End(offset) => data_len as i64 + offset,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid seek position",
            ));
        }

        *position = new_pos as usize;
        Ok(new_pos as u64)
    }
}

/// In-memory directory entry
#[derive(Clone)]
pub enum Entry {
    File(MemoryFile),
    Directory(MemoryDirectory),
}

/// In-memory directory
#[derive(Clone)]
pub struct MemoryDirectory {
    entries: Arc<RwLock<HashMap<String, Entry>>>,
}

impl MemoryDirectory {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_file(&self, name: &str, data: Vec<u8>) -> io::Result<()> {
        let mut entries = self.entries.write();
        if entries.contains_key(name) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "File already exists",
            ));
        }
        entries.insert(name.to_string(), Entry::File(MemoryFile::with_data(data)));
        Ok(())
    }

    pub fn create_dir(&self, name: &str) -> io::Result<()> {
        let mut entries = self.entries.write();
        if entries.contains_key(name) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Directory already exists",
            ));
        }
        entries.insert(name.to_string(), Entry::Directory(MemoryDirectory::new()));
        Ok(())
    }

    pub fn get_file(&self, name: &str) -> io::Result<MemoryFile> {
        let entries = self.entries.read();
        match entries.get(name) {
            Some(Entry::File(file)) => Ok(file.clone()),
            Some(Entry::Directory(_)) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Path is a directory",
            )),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "File not found",
            )),
        }
    }

    pub fn get_dir(&self, name: &str) -> io::Result<MemoryDirectory> {
        let entries = self.entries.read();
        match entries.get(name) {
            Some(Entry::Directory(dir)) => Ok(dir.clone()),
            Some(Entry::File(_)) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Path is a file",
            )),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Directory not found",
            )),
        }
    }

    pub fn list(&self) -> Vec<(String, bool)> {
        let entries = self.entries.read();
        entries
            .iter()
            .map(|(name, entry)| {
                let is_dir = matches!(entry, Entry::Directory(_));
                (name.clone(), is_dir)
            })
            .collect()
    }

    pub fn remove(&self, name: &str) -> io::Result<()> {
        let mut entries = self.entries.write();
        entries.remove(name).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Entry not found")
        })?;
        Ok(())
    }
}

/// Root filesystem with path resolution
pub struct MemoryFilesystem {
    root: MemoryDirectory,
}

impl MemoryFilesystem {
    pub fn new() -> Self {
        Self {
            root: MemoryDirectory::new(),
        }
    }

    /// Resolve a path and return the parent directory and filename
    fn resolve_path(&self, path: &str) -> io::Result<(MemoryDirectory, String)> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid path",
            ));
        }

        let parts: Vec<&str> = path.split('/').collect();
        let filename = parts.last().unwrap().to_string();

        if parts.len() == 1 {
            return Ok((self.root.clone(), filename));
        }

        let mut current_dir = self.root.clone();
        for &part in &parts[..parts.len() - 1] {
            current_dir = current_dir.get_dir(part)?;
        }

        Ok((current_dir, filename))
    }

    pub fn create_file(&self, path: &str, data: Vec<u8>) -> io::Result<()> {
        let (parent_dir, filename) = self.resolve_path(path)?;
        parent_dir.create_file(&filename, data)
    }

    pub fn open_file(&self, path: &str) -> io::Result<MemoryFile> {
        let (parent_dir, filename) = self.resolve_path(path)?;
        parent_dir.get_file(&filename)
    }

    pub fn create_dir_all(&self, path: &str) -> io::Result<()> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = path.split('/').collect();
        let mut current_dir = self.root.clone();

        for &part in &parts {
            match current_dir.get_dir(part) {
                Ok(dir) => current_dir = dir,
                Err(_) => {
                    current_dir.create_dir(part)?;
                    current_dir = current_dir.get_dir(part)?;
                }
            }
        }

        Ok(())
    }

    pub fn root(&self) -> &MemoryDirectory {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_file() {
        let mut file = MemoryFile::with_data(b"Hello, World!".to_vec());

        let mut buf = [0u8; 5];
        file.read(&mut buf).unwrap();
        assert_eq!(&buf, b"Hello");

        file.seek(SeekFrom::Start(0)).unwrap();
        file.write(b"Hi").unwrap();

        file.seek(SeekFrom::Start(0)).unwrap();
        let mut result = Vec::new();
        file.read_to_end(&mut result).unwrap();
        assert_eq!(result, b"Hillo, World!");
    }

    #[test]
    fn test_memory_filesystem() {
        let fs = MemoryFilesystem::new();

        fs.create_dir_all("/tmp").unwrap();
        fs.create_file("/data.bin", b"test data".to_vec()).unwrap();
        fs.create_file("/tmp/output.txt", b"output".to_vec()).unwrap();

        let mut file = fs.open_file("/data.bin").unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"test data");
    }
}
