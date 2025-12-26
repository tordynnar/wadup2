use crate::ffi;
use uuid::Uuid;
use std::io::{self, Read, Seek, SeekFrom};

pub struct Content;

impl Content {
    pub fn size() -> usize {
        unsafe { ffi::get_content_size() }
    }

    pub fn read(offset: usize, length: usize) -> Result<Vec<u8>, String> {
        let mut buffer = vec![0u8; length];

        unsafe {
            let result = ffi::read_content(offset, length, buffer.as_mut_ptr());

            if result < 0 {
                return Err("Failed to read content".to_string());
            }
        }

        Ok(buffer)
    }

    pub fn read_all() -> Result<Vec<u8>, String> {
        Self::read(0, Self::size())
    }

    pub fn uuid() -> Result<Uuid, String> {
        let mut buffer = [0u8; 16];

        unsafe {
            let result = ffi::get_content_uuid(buffer.as_mut_ptr());

            if result < 0 {
                return Err("Failed to get content UUID".to_string());
            }
        }

        Ok(Uuid::from_bytes(buffer))
    }

    /// Read content as UTF-8 string
    pub fn read_string() -> Result<String, String> {
        let bytes = Self::read_all()?;
        String::from_utf8(bytes).map_err(|e| format!("Content is not valid UTF-8: {}", e))
    }

    /// Create a reader for the content that implements Read and Seek
    pub fn reader() -> ContentReader {
        ContentReader::new()
    }
}

/// A reader for content that implements Read and Seek traits
pub struct ContentReader {
    position: usize,
    size: usize,
}

impl ContentReader {
    pub fn new() -> Self {
        Self {
            position: 0,
            size: Content::size(),
        }
    }
}

impl Default for ContentReader {
    fn default() -> Self {
        Self::new()
    }
}

impl Read for ContentReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.size {
            return Ok(0); // EOF
        }

        let remaining = self.size - self.position;
        let to_read = buf.len().min(remaining);

        if to_read == 0 {
            return Ok(0);
        }

        let data = Content::read(self.position, to_read)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        buf[..to_read].copy_from_slice(&data);
        self.position += to_read;

        Ok(to_read)
    }
}

impl Seek for ContentReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.size as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cannot seek before start of content",
            ));
        }

        // Allow seeking past the end (this is standard behavior for Seek)
        self.position = new_pos as usize;

        Ok(self.position as u64)
    }
}
