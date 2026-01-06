/// Helper for accessing content data in WADUP modules.
///
/// Content is exposed as a file at `/data.bin` in the WASM module's virtual filesystem.
/// Modules can access it using standard file I/O operations.
pub struct Content;

impl Content {
    /// Returns the path to the content file in the virtual filesystem.
    ///
    /// The content being processed is always available at this path.
    /// Modules can open and read this file using standard `std::fs` operations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::Read;
    /// use wadup_guest::Content;
    ///
    /// let mut file = File::open(Content::path()).unwrap();
    /// let mut data = Vec::new();
    /// file.read_to_end(&mut data).unwrap();
    /// ```
    pub fn path() -> &'static str {
        "/data.bin"
    }
}
