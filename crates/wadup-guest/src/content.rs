use crate::ffi;
use uuid::Uuid;

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
}
