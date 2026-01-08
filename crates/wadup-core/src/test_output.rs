//! Test output types for the `wadup test` subcommand.
//!
//! These types define the JSON output format that matches the Python test_runner.py
//! for compatibility with WADUP Web.

use serde::Serialize;

/// Output from running a single module test.
#[derive(Debug, Serialize)]
pub struct TestOutput {
    /// Whether the test was successful (exit_code == 0 and no errors).
    pub success: bool,

    /// Error message if the test failed.
    pub error: Option<String>,

    /// Captured stdout from the module.
    pub stdout: String,

    /// Captured stderr from the module.
    pub stderr: String,

    /// Exit code from the module (0 = success).
    pub exit_code: i32,

    /// Metadata output from /metadata/*.json files.
    /// Can be a single object or array of objects depending on file count.
    pub metadata: Option<serde_json::Value>,

    /// Extracted subcontent files from /subcontent/.
    pub subcontent: Option<Vec<SubcontentOutput>>,
}

/// A single extracted subcontent file.
#[derive(Debug, Serialize)]
pub struct SubcontentOutput {
    /// Index extracted from filename (data_N.bin).
    pub index: usize,

    /// Original filename from metadata_N.json.
    pub filename: Option<String>,

    /// Binary data as hex string (truncated to 4KB).
    pub data_hex: String,

    /// Actual file size in bytes.
    pub size: usize,

    /// Whether the data was truncated (file > 4KB).
    pub truncated: bool,

    /// Full metadata from metadata_N.json.
    pub metadata: Option<serde_json::Value>,
}

impl TestOutput {
    /// Create a successful test output.
    pub fn success(
        stdout: String,
        stderr: String,
        metadata: Option<serde_json::Value>,
        subcontent: Option<Vec<SubcontentOutput>>,
    ) -> Self {
        Self {
            success: true,
            error: None,
            stdout,
            stderr,
            exit_code: 0,
            metadata,
            subcontent,
        }
    }

    /// Create a failed test output.
    pub fn failure(
        error: impl Into<String>,
        exit_code: i32,
        stdout: String,
        stderr: String,
        subcontent: Option<Vec<SubcontentOutput>>,
    ) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
            stdout,
            stderr,
            exit_code,
            metadata: None,
            subcontent,
        }
    }
}
