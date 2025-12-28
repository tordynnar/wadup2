use std::path::PathBuf;
use std::process::Command;
use std::fs;

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(".."); // Go up from wadup-cli
    path.push(".."); // Go up from crates
    path
}

fn wadup_binary() -> PathBuf {
    let mut path = workspace_root();
    path.push("target");
    path.push("release");
    path.push("wadup");
    path
}

fn get_wasm_target(_example_name: &str) -> &'static str {
    // All modules now use WASI filesystem to access /data.bin
    "wasm32-wasip1"
}

fn build_wasm_module(example_name: &str) -> PathBuf {
    let mut manifest_path = workspace_root();
    manifest_path.push("examples");
    manifest_path.push(example_name);
    manifest_path.push("Cargo.toml");

    let target = get_wasm_target(example_name);

    let mut cmd = Command::new("cargo");
    cmd.args(&["build", "--manifest-path", manifest_path.to_str().unwrap(), "--target", target, "--release"]);

    // Set WASI_SDK_PATH for WASI targets
    if target.contains("wasi") {
        cmd.env("WASI_SDK_PATH", "/tmp/wasi-sdk-24.0-arm64-macos");
    }

    // Special settings for sqlite-parser to disable threading
    if example_name == "sqlite-parser" {
        cmd.env("LIBSQLITE3_FLAGS", "-DSQLITE_THREADSAFE=0");
    }

    let status = cmd.status()
        .expect(&format!("Failed to build {} module", example_name));

    assert!(status.success(), "{} module build failed", example_name);

    let mut path = workspace_root();
    path.push("examples");
    path.push(example_name);
    path.push("target");
    path.push(target);
    path.push("release");
    path.push(&format!("{}.wasm", example_name.replace("-", "_")));

    assert!(path.exists(), "WASM module not found at {:?}", path);
    path
}

// Build shared Python WASI if not already built
fn ensure_python_wasi_built() {
    let mut python_build_script = workspace_root();
    python_build_script.push("scripts");
    python_build_script.push("build-python-wasi.sh");

    // Only build if not already present
    let mut python_lib = workspace_root();
    python_lib.push("build/python-wasi/lib/libpython3.13.a");

    if !python_lib.exists() {
        println!("Building shared Python WASI (this may take 5-10 minutes)...");
        let status = Command::new(python_build_script)
            .status()
            .expect("Failed to run build-python-wasi.sh");

        assert!(status.success(), "Python WASI build failed");
    }
}

fn setup_modules_dir(modules: &[&str]) -> tempfile::TempDir {
    let modules_dir = tempfile::tempdir().unwrap();

    for module in modules {
        let wasm_path = build_wasm_module(module);
        let dest = modules_dir.path().join(format!("{}.wasm", module.replace("-", "_")));
        fs::copy(&wasm_path, &dest).unwrap();
    }

    modules_dir
}

#[test]
fn test_sqlite_parser() {
    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Setup modules directory
    let modules_dir = setup_modules_dir(&["sqlite-parser"]);

    // Setup input directory
    let input_dir = tempfile::tempdir().unwrap();
    let db_path = input_dir.path().join("sample.db");
    let mut fixture_path = workspace_root();
    fixture_path.push("tests/fixtures/sample.db");
    fs::copy(&fixture_path, &db_path).unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup
    let status = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run wadup");

    assert!(status.success(), "wadup execution failed");

    // Verify results
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Check that db_table_stats table exists
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='db_table_stats'").unwrap();
    let exists: bool = stmt.exists([]).unwrap();
    assert!(exists, "db_table_stats table not created");

    // Check that we have some stats
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM db_table_stats", [], |row| row.get(0)).unwrap();
    assert!(count > 0, "No statistics recorded");
}

#[test]
fn test_zip_extractor_and_byte_counter() {
    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Setup modules directory with both zip-extractor and byte-counter
    let modules_dir = setup_modules_dir(&["zip-extractor", "byte-counter"]);

    // Setup input directory
    let input_dir = tempfile::tempdir().unwrap();
    let zip_path = input_dir.path().join("test.zip");
    let mut fixture_path = workspace_root();
    fixture_path.push("tests/fixtures/test.zip");
    fs::copy(&fixture_path, &zip_path).unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup
    let status = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run wadup");

    assert!(status.success(), "wadup execution failed");

    // Verify results
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Check that file_sizes table exists
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='file_sizes'").unwrap();
    let exists: bool = stmt.exists([]).unwrap();
    assert!(exists, "file_sizes table not created");

    // Check that we have sizes for the ZIP file and its contents
    // We should have:
    // 1. The original ZIP file
    // 2. file1.txt (extracted from ZIP)
    // 3. file2.txt (extracted from ZIP)
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM file_sizes", [], |row| row.get(0)).unwrap();
    assert!(count >= 3, "Expected at least 3 file size entries, got {}", count);

    // Check that we have content entries for extracted files
    let extracted_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM __wadup_content WHERE filename LIKE '%.txt'",
        [],
        |row| row.get(0)
    ).unwrap();
    assert!(extracted_count >= 2, "Expected at least 2 extracted .txt files, got {}", extracted_count);
}

#[test]
fn test_combined_sqlite_and_zip() {
    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Setup modules directory with all three modules
    let modules_dir = setup_modules_dir(&["sqlite-parser", "zip-extractor", "byte-counter"]);

    // Setup input directory with both files
    let input_dir = tempfile::tempdir().unwrap();
    let mut db_fixture = workspace_root();
    db_fixture.push("tests/fixtures/sample.db");
    let mut zip_fixture = workspace_root();
    zip_fixture.push("tests/fixtures/test.zip");
    fs::copy(&db_fixture, input_dir.path().join("sample.db")).unwrap();
    fs::copy(&zip_fixture, input_dir.path().join("test.zip")).unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup
    let status = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run wadup");

    assert!(status.success(), "wadup execution failed");

    // Verify results
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Check that both metadata tables exist
    let db_table_stats_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='db_table_stats'",
        [],
        |row| Ok(row.get::<_, i64>(0)? > 0)
    ).unwrap();
    assert!(db_table_stats_exists, "db_table_stats table not created");

    let file_sizes_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='file_sizes'",
        [],
        |row| Ok(row.get::<_, i64>(0)? > 0)
    ).unwrap();
    assert!(file_sizes_exists, "file_sizes table not created");

    // Check that we processed both input files
    let content_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM __wadup_content WHERE parent_uuid IS NULL",
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(content_count, 2, "Expected 2 top-level content entries");

    // Check that ZIP was extracted (should have child content)
    let extracted_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM __wadup_content WHERE parent_uuid IS NOT NULL",
        [],
        |row| row.get(0)
    ).unwrap();
    assert!(extracted_count >= 2, "Expected at least 2 extracted files from ZIP");
}

// Helper function to build Python WASM module (doesn't use Cargo)
fn build_python_module(module_name: &str) -> PathBuf {
    // Ensure shared Python WASI is built first
    ensure_python_wasi_built();

    let mut python_example = workspace_root();
    python_example.push(format!("examples/{}", module_name));

    // Build module using make
    let build_status = Command::new("make")
        .current_dir(&python_example)
        .status()
        .expect(&format!("Failed to run make for {}", module_name));

    assert!(build_status.success(), "{} module build failed", module_name);

    // Return path to WASM file
    let mut wasm_path = python_example;
    wasm_path.push(format!("target/{}.wasm", module_name.replace("-", "_")));

    assert!(wasm_path.exists(), "Python WASM module not found at {:?}", wasm_path);
    wasm_path
}

// Helper function to build Go WASM module (doesn't use Cargo)
fn build_go_module(module_name: &str) -> PathBuf {
    let mut go_example = workspace_root();
    go_example.push(format!("examples/{}", module_name));

    // Build module using make
    let build_status = Command::new("make")
        .current_dir(&go_example)
        .status()
        .expect(&format!("Failed to run make for {}", module_name));

    assert!(build_status.success(), "{} module build failed", module_name);

    // Return path to WASM file
    let mut wasm_path = go_example;
    wasm_path.push(format!("target/{}.wasm", module_name.replace("-", "_")));

    assert!(wasm_path.exists(), "Go WASM module not found at {:?}", wasm_path);
    wasm_path
}

// Helper function to build C# WASM module (uses dotnet + Wasi.Sdk)
fn build_csharp_module(module_name: &str) -> PathBuf {
    let mut csharp_example = workspace_root();
    csharp_example.push(format!("examples/{}", module_name));

    // Build module using make
    let build_status = Command::new("make")
        .current_dir(&csharp_example)
        .status()
        .expect(&format!("Failed to run make for {}", module_name));

    assert!(build_status.success(), "{} module build failed", module_name);

    // Return path to WASM file
    let mut wasm_path = csharp_example;
    wasm_path.push(format!("target/{}.wasm", module_name.replace("-", "_")));

    assert!(wasm_path.exists(), "C# WASM module not found at {:?}", wasm_path);
    wasm_path
}

#[test]
fn test_python_sqlite_parser() {
    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(workspace_root())
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Build Python module
    let python_wasm = build_python_module("python-sqlite-parser");

    // Setup modules directory
    let modules_dir = tempfile::tempdir().unwrap();
    let dest = modules_dir.path().join("python_sqlite_parser.wasm");
    fs::copy(&python_wasm, &dest).unwrap();

    // Setup input directory
    let input_dir = tempfile::tempdir().unwrap();
    let db_path = input_dir.path().join("sample.db");
    let mut fixture_path = workspace_root();
    fixture_path.push("tests/fixtures/sample.db");
    fs::copy(&fixture_path, &db_path).unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup
    let status = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run wadup");

    assert!(status.success(), "wadup execution failed");

    // Verify results match Rust sqlite-parser
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Check table exists
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='db_table_stats'"
    ).unwrap();
    assert!(stmt.exists([]).unwrap(), "db_table_stats table not created");

    // Check data
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM db_table_stats",
        [],
        |row| row.get(0)
    ).unwrap();
    assert!(count > 0, "No statistics recorded");

    // Verify content matches Rust version
    let python_stats: Vec<(String, i64)> = conn.prepare(
        "SELECT table_name, row_count FROM db_table_stats ORDER BY table_name"
    ).unwrap()
    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    assert!(python_stats.len() >= 2, "Expected at least 2 tables");
    assert!(python_stats.iter().any(|(name, _)| name == "users"),
            "Missing 'users' table");
}

#[test]
fn test_go_sqlite_parser() {
    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(workspace_root())
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Build Go module
    let go_wasm = build_go_module("go-sqlite-parser");

    // Setup modules directory
    let modules_dir = tempfile::tempdir().unwrap();
    let dest = modules_dir.path().join("go_sqlite_parser.wasm");
    fs::copy(&go_wasm, &dest).unwrap();

    // Setup input directory
    let input_dir = tempfile::tempdir().unwrap();
    let db_path = input_dir.path().join("sample.db");
    let mut fixture_path = workspace_root();
    fixture_path.push("tests/fixtures/sample.db");
    fs::copy(&fixture_path, &db_path).unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup
    let status = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run wadup");

    assert!(status.success(), "wadup execution failed");

    // Verify results match Rust/Python sqlite-parser
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Check table exists
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='db_table_stats'"
    ).unwrap();
    assert!(stmt.exists([]).unwrap(), "db_table_stats table not created");

    // Check data
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM db_table_stats",
        [],
        |row| row.get(0)
    ).unwrap();
    assert!(count > 0, "No statistics recorded");

    // Verify content matches Rust/Python versions
    let go_stats: Vec<(String, i64)> = conn.prepare(
        "SELECT table_name, row_count FROM db_table_stats ORDER BY table_name"
    ).unwrap()
    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    assert!(go_stats.len() >= 2, "Expected at least 2 tables");
    assert!(go_stats.iter().any(|(name, _)| name == "users"),
            "Missing 'users' table");
}

#[test]
fn test_python_module_reuse() {
    // This test verifies that Python modules are loaded once and reused across
    // multiple files, rather than being re-initialized for each file.
    // The python-counter module maintains a global counter that increments
    // on each call. If the module is properly reused, we should see 1, 2, 3...
    // If it's being reloaded, we'd see 1, 1, 1...

    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(workspace_root())
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Build Python counter module
    let python_wasm = build_python_module("python-counter");

    // Setup modules directory
    let modules_dir = tempfile::tempdir().unwrap();
    let dest = modules_dir.path().join("python_counter.wasm");
    fs::copy(&python_wasm, &dest).unwrap();

    // Setup input directory with 3 test files
    let input_dir = tempfile::tempdir().unwrap();
    fs::write(input_dir.path().join("file1.txt"), "test1").unwrap();
    fs::write(input_dir.path().join("file2.txt"), "test2").unwrap();
    fs::write(input_dir.path().join("file3.txt"), "test3").unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup with single thread to ensure sequential processing
    let status = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
            "--threads", "1",  // Single thread for deterministic ordering
        ])
        .status()
        .expect("Failed to run wadup");

    assert!(status.success(), "wadup execution failed");

    // Verify results - counter should increment
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Get all counter values ordered by ROWID
    let counter_values: Vec<i64> = conn.prepare(
        "SELECT call_number FROM call_counter ORDER BY ROWID"
    ).unwrap()
    .query_map([], |row| row.get(0))
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    // Verify we have exactly 3 values
    assert_eq!(counter_values.len(), 3, "Expected 3 counter values, got {}", counter_values.len());

    // Verify the counter incremented (module was reused)
    assert_eq!(counter_values[0], 1, "First call should be 1");
    assert_eq!(counter_values[1], 2, "Second call should be 2 (module reused)");
    assert_eq!(counter_values[2], 3, "Third call should be 3 (module reused)");

    println!("✓ Module reuse verified: counter values are {}, {}, {}",
             counter_values[0], counter_values[1], counter_values[2]);
}

#[test]
fn test_python_c_extensions() {
    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(workspace_root())
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Build Python module test module
    let python_wasm = build_python_module("python-module-test");

    // Setup modules directory
    let modules_dir = tempfile::tempdir().unwrap();
    let dest = modules_dir.path().join("python_module_test.wasm");
    fs::copy(&python_wasm, &dest).unwrap();

    // Setup input directory with a single dummy file
    let input_dir = tempfile::tempdir().unwrap();
    fs::write(input_dir.path().join("test.txt"), "test").unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup
    let status = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run wadup");

    assert!(status.success(), "wadup execution failed");

    // Verify results - all C extensions should import successfully
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Check that the table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='c_extension_imports'",
        [],
        |row| row.get::<_, i64>(0)
    ).unwrap() > 0;

    assert!(table_exists, "c_extension_imports table not created");

    // Get all import results
    let mut stmt = conn.prepare(
        "SELECT module_name, import_successful, error_message FROM c_extension_imports ORDER BY module_name"
    ).unwrap();

    let results: Vec<(String, i64, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Expected C extension modules that are available in Python WASI build
    let expected_modules = vec![
        "array", "binascii", "bz2", "cmath", "hashlib", "io", "itertools",
        "lzma", "math", "struct", "time", "unicodedata", "zlib"
    ];

    // Verify we have results for all expected modules
    assert_eq!(results.len(), expected_modules.len(),
               "Expected {} modules, got {}", expected_modules.len(), results.len());

    // Verify each module imported successfully
    let mut failed_imports = Vec::new();
    for (module_name, import_successful, error_message) in &results {
        if *import_successful == 0 {
            failed_imports.push(format!("{}: {}", module_name, error_message));
        }
    }

    if !failed_imports.is_empty() {
        panic!("Failed to import the following C extension modules:\n{}",
               failed_imports.join("\n"));
    }

    println!("✓ All {} C extension modules imported successfully:", expected_modules.len());
    for (module_name, _, _) in &results {
        println!("  - {}", module_name);
    }
}

#[test]
fn test_csharp_json_analyzer() {
    // Build the CLI
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(workspace_root())
        .status()
        .expect("Failed to build wadup CLI");
    assert!(status.success(), "CLI build failed");

    // Build C# module
    let csharp_wasm = build_csharp_module("csharp-json-analyzer");

    // Setup modules directory
    let modules_dir = tempfile::tempdir().unwrap();
    let dest = modules_dir.path().join("csharp_json_analyzer.wasm");
    fs::copy(&csharp_wasm, &dest).unwrap();

    // Setup input directory with a JSON file
    let input_dir = tempfile::tempdir().unwrap();
    let json_content = r#"{"name": "test", "values": [1, 2, 3], "nested": {"a": "b"}}"#;
    fs::write(input_dir.path().join("test.json"), json_content).unwrap();

    // Setup output database
    let output_dir = tempfile::tempdir().unwrap();
    let output_db = output_dir.path().join("output.db");

    // Run wadup and capture stderr to verify incremental metadata processing
    let output = Command::new(wadup_binary())
        .args(&[
            "--modules", modules_dir.path().to_str().unwrap(),
            "--input", input_dir.path().to_str().unwrap(),
            "--output", output_db.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wadup");

    assert!(output.status.success(), "wadup execution failed");

    // Check stderr for debug output showing metadata processed on fd_close
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("=== WADUP stderr output ===\n{}", stderr);

    // Verify metadata is processed on fd_close (before _start completes)
    // The pattern should be: "WADUP: Processing metadata on fd_close" appears multiple times
    // BEFORE the final "WADUP: _start completed" message
    let fd_close_count = stderr.matches("WADUP: Processing metadata on fd_close").count();
    let start_completed_count = stderr.matches("WADUP: _start completed").count();

    assert!(fd_close_count >= 5,
        "Expected at least 5 metadata files processed on fd_close, got {}. \
         This verifies incremental metadata processing.", fd_close_count);

    // Verify subcontent is processed on fd_close
    // The JSON has 2 string values: "test" (name) and "b" (nested.a)
    let subcontent_count = stderr.matches("WADUP: Processing subcontent on fd_close").count();
    assert!(subcontent_count >= 2,
        "Expected at least 2 subcontent files processed on fd_close, got {}. \
         This verifies file-based sub-content emission.", subcontent_count);

    // Verify the order: fd_close processing should happen BEFORE _start completes
    // Find the position of the first "_start completed" after all fd_close messages
    let last_fd_close_pos = stderr.rfind("WADUP: Processing metadata on fd_close")
        .expect("Should have fd_close processing messages");
    let first_start_after_processing = stderr[last_fd_close_pos..].find("WADUP: _start completed");
    assert!(first_start_after_processing.is_some(),
        "_start completed should appear after the last fd_close processing");

    // Verify results in database
    let conn = rusqlite::Connection::open(&output_db).unwrap();

    // Check that json_metadata table exists and has data
    let json_metadata_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM json_metadata",
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(json_metadata_count, 1, "Expected 1 row in json_metadata");

    // Check that json_keys table exists and has all 4 keys
    let json_keys_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM json_keys",
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(json_keys_count, 4, "Expected 4 rows in json_keys (name, values, nested, a)");

    // Get the metadata row
    let (max_depth, total_keys, total_arrays, total_objects, parser_used): (i64, i64, i64, i64, String) = conn.query_row(
        "SELECT max_depth, total_keys, total_arrays, total_objects, parser_used FROM json_metadata",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    ).unwrap();

    // Verify the analysis results
    assert!(max_depth >= 3, "Expected max_depth >= 3, got {}", max_depth);
    assert_eq!(total_keys, 4, "Expected total_keys = 4, got {}", total_keys);
    assert_eq!(total_arrays, 1, "Expected 1 array, got {}", total_arrays);
    assert_eq!(total_objects, 2, "Expected 2 objects, got {}", total_objects);
    assert_eq!(parser_used, "System.Text.Json", "Expected System.Text.Json parser");

    // Get the keys
    let mut keys: Vec<String> = conn.prepare("SELECT key_name FROM json_keys ORDER BY key_name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    keys.sort();
    assert_eq!(keys, vec!["a", "name", "nested", "values"],
        "Expected keys [a, name, nested, values], got {:?}", keys);

    // Verify sub-content was emitted and processed
    // The JSON has 2 string values that should be emitted as sub-content:
    // - "test" (the value of "name")
    // - "b" (the value of "nested.a")
    let subcontent_entries: i64 = conn.query_row(
        "SELECT COUNT(*) FROM __wadup_content WHERE parent_uuid IS NOT NULL",
        [],
        |row| row.get(0)
    ).unwrap();
    assert!(subcontent_entries >= 2,
        "Expected at least 2 sub-content entries (extracted string values), got {}", subcontent_entries);

    // Verify that sub-content files have .txt extension (which the JSON analyzer ignores)
    let txt_subcontent: Vec<String> = conn.prepare(
        "SELECT filename FROM __wadup_content WHERE parent_uuid IS NOT NULL AND filename LIKE '%.txt'"
    ).unwrap()
    .query_map([], |row| row.get(0))
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();
    assert!(txt_subcontent.len() >= 2,
        "Expected at least 2 .txt sub-content files, got {:?}", txt_subcontent);

    // Verify no infinite recursion: the .txt files should NOT have child sub-content
    // (since they're not JSON and the JSON analyzer returns early)
    let grandchild_content: i64 = conn.query_row(
        r#"SELECT COUNT(*) FROM __wadup_content c1
           JOIN __wadup_content c2 ON c2.parent_uuid = c1.uuid
           JOIN __wadup_content c3 ON c3.parent_uuid = c2.uuid"#,
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(grandchild_content, 0,
        "Expected no grandchild content (would indicate infinite recursion), got {}", grandchild_content);

    println!("✓ C# JSON analyzer verified:");
    println!("  - Metadata files processed on fd_close: {}", fd_close_count);
    println!("  - Subcontent files processed on fd_close: {}", subcontent_count);
    println!("  - _start completed count: {}", start_completed_count);
    println!("  - json_metadata rows: {}", json_metadata_count);
    println!("  - json_keys rows: {}", json_keys_count);
    println!("  - max_depth: {}", max_depth);
    println!("  - total_keys: {}", total_keys);
    println!("  - Keys: {:?}", keys);
    println!("  - Sub-content entries: {}", subcontent_entries);
    println!("  - .txt sub-content files: {:?}", txt_subcontent);
    println!("  - Grandchild content (recursion check): {}", grandchild_content);
    println!("✓ Incremental metadata processing verified!");
    println!("✓ File-based sub-content emission verified!");
    println!("✓ No infinite recursion verified!");
}
