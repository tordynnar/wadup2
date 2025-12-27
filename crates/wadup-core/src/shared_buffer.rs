use bytes::Bytes;
use memmap2::Mmap;
use std::path::Path;
use anyhow::Result;

/// Unified abstraction over memory-mapped and in-memory data
///
/// This type provides zero-copy slicing and efficient sharing of content data.
/// Files are memory-mapped then immediately converted to Bytes for consistent
/// zero-copy operations throughout the processing pipeline.
#[derive(Clone, Debug)]
pub struct SharedBuffer {
    data: Bytes,
}

impl SharedBuffer {
    /// Create from file via memory mapping
    ///
    /// The file is memory-mapped and immediately converted to Bytes.
    /// This involves one copy from the memory-mapped region to Bytes,
    /// but enables all subsequent operations to be zero-copy.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        // Convert to Bytes (one copy, but enables zero-copy slicing downstream)
        let data = Bytes::copy_from_slice(&mmap[..]);
        Ok(Self { data })
    }

    /// Create from Vec<u8> (takes ownership)
    ///
    /// Converts the Vec to Bytes without copying (just wraps it).
    pub fn from_vec(vec: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(vec),
        }
    }

    /// Create from existing Bytes
    ///
    /// This is a cheap clone operation (just increments reference count).
    pub fn from_bytes(bytes: Bytes) -> Self {
        Self { data: bytes }
    }

    /// Get slice as &[u8]
    ///
    /// Provides a view into the data without copying.
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..]
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Create zero-copy slice
    ///
    /// Returns a new SharedBuffer that references a slice of this buffer's data.
    /// No copying occurs - both buffers share the same underlying memory via
    /// reference counting.
    pub fn slice(&self, range: std::ops::Range<usize>) -> Self {
        Self {
            data: self.data.slice(range),
        }
    }

    /// Convert to Bytes
    ///
    /// This is a cheap clone operation (just increments reference count).
    pub fn to_bytes(&self) -> Bytes {
        self.data.clone()
    }

    /// Get a clone of the underlying Bytes
    ///
    /// This is a cheap operation (just increments reference count).
    pub fn clone_bytes(&self) -> Bytes {
        self.data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_from_vec() {
        let data = vec![1, 2, 3, 4, 5];
        let buffer = SharedBuffer::from_vec(data);
        assert_eq!(buffer.as_slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(buffer.len(), 5);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_from_file() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"Hello, World!")?;
        temp_file.flush()?;

        let buffer = SharedBuffer::from_file(temp_file.path())?;
        assert_eq!(buffer.as_slice(), b"Hello, World!");
        assert_eq!(buffer.len(), 13);

        Ok(())
    }

    #[test]
    fn test_zero_copy_slice() {
        let buffer = SharedBuffer::from_vec(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        // Create a slice
        let slice1 = buffer.slice(2..7);
        assert_eq!(slice1.as_slice(), &[2, 3, 4, 5, 6]);

        // Create a slice of a slice (zero-copy)
        let slice2 = slice1.slice(1..4);
        assert_eq!(slice2.as_slice(), &[3, 4, 5]);

        // Original buffer unchanged
        assert_eq!(buffer.as_slice(), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_clone_is_cheap() {
        let buffer1 = SharedBuffer::from_vec(vec![1, 2, 3, 4, 5]);
        let buffer2 = buffer1.clone();

        // Both reference the same data
        assert_eq!(buffer1.as_slice(), buffer2.as_slice());

        // Modifying one doesn't affect the other (they share immutable data)
        let slice = buffer1.slice(1..3);
        assert_eq!(slice.as_slice(), &[2, 3]);
        assert_eq!(buffer2.as_slice(), &[1, 2, 3, 4, 5]);
    }
}
