use std::collections::HashMap;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::{Bytes, BytesMut};

/// File data storage - either read-only or read-write
#[derive(Clone)]
pub enum MemoryFileData {
    /// Read-only view (zero-copy reference to content)
    ReadOnly(Bytes),
    /// Read-write buffer (for WASM-generated content)
    ReadWrite(Arc<RwLock<BytesMut>>),
}

/// In-memory file with Read/Write/Seek support
#[derive(Clone)]
pub struct MemoryFile {
    data: MemoryFileData,
    position: Arc<RwLock<usize>>,
}

impl MemoryFile {
    pub fn new() -> Self {
        Self {
            data: MemoryFileData::ReadWrite(Arc::new(RwLock::new(BytesMut::new()))),
            position: Arc::new(RwLock::new(0)),
        }
    }

    /// Create read-only file from Bytes (zero-copy)
    pub fn with_readonly_data(data: Bytes) -> Self {
        Self {
            data: MemoryFileData::ReadOnly(data),
            position: Arc::new(RwLock::new(0)),
        }
    }

    /// Create read-write file from Vec (takes ownership)
    pub fn with_data(data: Vec<u8>) -> Self {
        Self {
            data: MemoryFileData::ReadWrite(Arc::new(RwLock::new(BytesMut::from(&data[..])))),
            position: Arc::new(RwLock::new(0)),
        }
    }

    pub fn len(&self) -> usize {
        match &self.data {
            MemoryFileData::ReadOnly(bytes) => bytes.len(),
            MemoryFileData::ReadWrite(buf) => buf.read().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Read for MemoryFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut pos = self.position.write();

        match &self.data {
            MemoryFileData::ReadOnly(bytes) => {
                if *pos >= bytes.len() {
                    return Ok(0);
                }

                let available = bytes.len() - *pos;
                let to_read = buf.len().min(available);
                buf[..to_read].copy_from_slice(&bytes[*pos..*pos + to_read]);
                *pos += to_read;

                Ok(to_read)
            }
            MemoryFileData::ReadWrite(data) => {
                let data_guard = data.read();

                if *pos >= data_guard.len() {
                    return Ok(0);
                }

                let available = data_guard.len() - *pos;
                let to_read = buf.len().min(available);
                buf[..to_read].copy_from_slice(&data_guard[*pos..*pos + to_read]);
                *pos += to_read;

                Ok(to_read)
            }
        }
    }
}

impl Write for MemoryFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &self.data {
            MemoryFileData::ReadOnly(_) => {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Cannot write to read-only file",
                ))
            }
            MemoryFileData::ReadWrite(data) => {
                let mut data_guard = data.write();
                let mut pos = self.position.write();

                // Extend if writing past end
                if *pos + buf.len() > data_guard.len() {
                    data_guard.resize(*pos + buf.len(), 0);
                }

                data_guard[*pos..*pos + buf.len()].copy_from_slice(buf);
                *pos += buf.len();

                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for MemoryFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let data_len = match &self.data {
            MemoryFileData::ReadOnly(bytes) => bytes.len(),
            MemoryFileData::ReadWrite(data) => data.read().len(),
        };
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

    /// Create or replace /data.bin with zero-copy view
    ///
    /// This method provides a zero-copy way to update the /data.bin file
    /// used by WASM modules. The Bytes data is stored directly without copying.
    pub fn set_data_bin(&self, data: Bytes) -> io::Result<()> {
        // Remove existing /data.bin if it exists
        let _ = self.root.remove("data.bin");

        // Create new read-only file
        let mut entries = self.root.entries.write();
        entries.insert("data.bin".to_string(), Entry::File(MemoryFile::with_readonly_data(data)));
        Ok(())
    }

    /// Get directory at path
    pub fn get_dir(&self, path: &str) -> io::Result<MemoryDirectory> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Ok(self.root.clone());
        }

        let parts: Vec<&str> = path.split('/').collect();
        let mut current_dir = self.root.clone();

        for &part in &parts {
            current_dir = current_dir.get_dir(part)?;
        }

        Ok(current_dir)
    }

    /// Read entire file contents as Vec<u8>
    pub fn read_file(&self, path: &str) -> io::Result<Vec<u8>> {
        let mut file = self.open_file(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        Ok(contents)
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
