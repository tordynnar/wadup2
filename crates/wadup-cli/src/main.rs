use clap::Parser;
use std::path::PathBuf;
use anyhow::Result;
use wadup_core::*;

#[derive(Parser)]
#[command(name = "wadup")]
#[command(about = "Web Assembly Data Unified Processing")]
#[command(version)]
struct Cli {
    #[arg(long, help = "Directory containing WASM modules")]
    modules: PathBuf,

    #[arg(long, help = "Directory containing input files")]
    input: PathBuf,

    #[arg(long, help = "Output SQLite database path")]
    output: PathBuf,

    #[arg(long, default_value = "4", help = "Number of worker threads")]
    threads: usize,

    #[arg(long, help = "Fuel limit (CPU) per module per content (e.g., 10000000). If not set, no CPU limit.")]
    fuel: Option<u64>,

    #[arg(long, help = "Maximum memory in bytes per module instance (e.g., 67108864 for 64MB). If not set, uses wasmtime defaults.")]
    max_memory: Option<usize>,

    #[arg(long, help = "Maximum stack size in bytes per module instance (e.g., 1048576 for 1MB). If not set, uses wasmtime defaults.")]
    max_stack: Option<usize>,

    #[arg(long, default_value = "100", help = "Maximum recursion depth for sub-content (number of nesting levels allowed)")]
    max_recursion_depth: usize,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .init();

    tracing::info!("WADUP - Web Assembly Data Unified Processing");
    tracing::info!("============================================");

    // Validate inputs
    if !cli.modules.exists() || !cli.modules.is_dir() {
        anyhow::bail!("Modules directory does not exist: {:?}", cli.modules);
    }

    if !cli.input.exists() || !cli.input.is_dir() {
        anyhow::bail!("Input directory does not exist: {:?}", cli.input);
    }

    if cli.threads == 0 {
        anyhow::bail!("Number of threads must be at least 1");
    }

    // Configure resource limits
    let limits = ResourceLimits {
        fuel: cli.fuel,
        max_memory: cli.max_memory,
        max_stack: cli.max_stack,
    };

    tracing::info!("Configuration:");
    tracing::info!("  Modules directory: {:?}", cli.modules);
    tracing::info!("  Input directory: {:?}", cli.input);
    tracing::info!("  Output database: {:?}", cli.output);
    tracing::info!("  Worker threads: {}", cli.threads);
    tracing::info!("  Max recursion depth: {}", cli.max_recursion_depth);

    if let Some(fuel) = limits.fuel {
        tracing::info!("  Fuel limit: {}", fuel);
    } else {
        tracing::info!("  Fuel limit: None (no CPU limit)");
    }

    if let Some(mem) = limits.max_memory {
        tracing::info!("  Memory limit: {} bytes ({} MB)", mem, mem / 1024 / 1024);
    } else {
        tracing::info!("  Memory limit: None (wasmtime defaults)");
    }

    if let Some(stack) = limits.max_stack {
        tracing::info!("  Stack limit: {} bytes ({} KB)", stack, stack / 1024);
    } else {
        tracing::info!("  Stack limit: None (wasmtime defaults)");
    }

    // Load WASM modules
    tracing::info!("Loading WASM modules...");
    let mut runtime = WasmRuntime::new(limits)?;
    runtime.load_modules(&cli.modules)?;

    // Create metadata store
    tracing::info!("Initializing metadata store...");
    let metadata_store = MetadataStore::new(cli.output.to_str().unwrap())?;

    // Load input files
    tracing::info!("Loading input files...");
    let contents = load_files(&cli.input)?;
    tracing::info!("Found {} input files", contents.len());

    // Create processor
    let processor = ContentProcessor::new(
        runtime,
        metadata_store,
        cli.max_recursion_depth,
    );

    // Process content
    tracing::info!("Starting processing...");
    processor.process(contents, cli.threads)?;

    tracing::info!("============================================");
    tracing::info!("Processing complete! Results written to: {:?}", cli.output);

    Ok(())
}

fn load_files(input_dir: &PathBuf) -> Result<Vec<Content>> {
    let mut contents = Vec::new();

    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            tracing::debug!("Loading file: {}", filename);
            // Use memory mapping for zero-copy file loading
            let buffer = wadup_core::shared_buffer::SharedBuffer::from_file(&path)?;
            let content = Content::new_root(buffer, filename);

            contents.push(content);
        }
    }

    Ok(contents)
}
