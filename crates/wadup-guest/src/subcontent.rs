use crate::ffi;

pub struct SubContent;

impl SubContent {
    pub fn emit_bytes(data: &[u8], filename: &str) -> Result<(), String> {
        unsafe {
            let result = ffi::emit_subcontent_bytes(
                data.as_ptr(),
                data.len(),
                filename.as_ptr(),
                filename.len(),
            );

            if result < 0 {
                return Err(format!("Failed to emit sub-content '{}'", filename));
            }
        }

        Ok(())
    }

    pub fn emit_slice(offset: usize, length: usize, filename: &str) -> Result<(), String> {
        unsafe {
            let result = ffi::emit_subcontent_slice(
                offset,
                length,
                filename.as_ptr(),
                filename.len(),
            );

            if result < 0 {
                return Err(format!("Failed to emit sub-content slice '{}'", filename));
            }
        }

        Ok(())
    }
}
