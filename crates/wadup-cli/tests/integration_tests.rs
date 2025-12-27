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
fn build_python_module() -> PathBuf {
    let mut python_example = workspace_root();
    python_example.push("examples/python-sqlite-parser");

    // Run build.sh script
    let build_status = Command::new("./build.sh")
        .current_dir(&python_example)
        .status()
        .expect("Failed to run build.sh for python-sqlite-parser");

    assert!(build_status.success(), "Python module build failed");

    // Return path to WASM file
    let mut wasm_path = python_example;
    wasm_path.push("target/python_sqlite_parser.wasm");

    assert!(wasm_path.exists(), "Python WASM module not found at {:?}", wasm_path);
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
    let python_wasm = build_python_module();

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
