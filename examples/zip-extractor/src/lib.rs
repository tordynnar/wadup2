use wadup_guest::*;
use std::io::Read;

#[no_mangle]
pub extern "C" fn process() -> i32 {
    if let Err(_) = run() {
        return 1;
    }
    0
}

fn run() -> Result<(), String> {
    let reader = Content::reader();

    // Try to parse as ZIP
    let mut archive = match zip::ZipArchive::new(reader) {
        Ok(archive) => archive,
        Err(_) => {
            // Not a ZIP file, skip processing
            return Ok(());
        }
    };

    // Extract each file in the ZIP as sub-content
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry {}: {}", i, e))?;

        // Skip directories
        if file.is_dir() {
            continue;
        }

        let filename = file.name().to_string();

        // Read the file contents
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| format!("Failed to read ZIP file '{}': {}", filename, e))?;

        // Emit as sub-content
        SubContent::emit_bytes(&contents, &filename)?;
    }

    Ok(())
}
