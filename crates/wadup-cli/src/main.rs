use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;
use wadup_core::*;

#[derive(Parser)]
#[command(name = "wadup")]
#[command(about = "Web Assembly Data Unified Processing")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true, help = "Verbose output")]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Precompile WASM modules for faster subsequent runs
    Compile {
        #[arg(long, help = "Directory containing WASM modules")]
        modules: PathBuf,

        #[arg(long, help = "Fuel limit (CPU) per module per content")]
        fuel: Option<u64>,

        #[arg(long, help = "Maximum memory in bytes per module instance")]
        max_memory: Option<usize>,

        #[arg(long, help = "Maximum stack size in bytes per module instance")]
        max_stack: Option<usize>,
    },

    /// Run WASM modules on input files
    Run {
        #[arg(long, help = "Directory containing WASM modules")]
        modules: PathBuf,

        #[arg(long, help = "Directory containing input files")]
        input: PathBuf,

        #[arg(long, default_value = "http://localhost:9200", help = "Elasticsearch URL")]
        es_url: String,

        #[arg(long, default_value = "wadup", help = "Elasticsearch index name")]
        es_index: String,

        #[arg(long, default_value = "4", help = "Number of worker threads")]
        threads: usize,

        #[arg(long, help = "Fuel limit (CPU) per module per content")]
        fuel: Option<u64>,

        #[arg(long, help = "Maximum memory in bytes per module instance")]
        max_memory: Option<usize>,

        #[arg(long, help = "Maximum stack size in bytes per module instance")]
        max_stack: Option<usize>,

        #[arg(long, default_value = "100", help = "Maximum recursion depth for sub-content")]
        max_recursion_depth: usize,
    },
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

    match cli.command {
        Commands::Compile { modules, fuel, max_memory, max_stack } => {
            run_compile(modules, fuel, max_memory, max_stack)
        }
        Commands::Run { modules, input, es_url, es_index, threads, fuel, max_memory, max_stack, max_recursion_depth } => {
            run_process(modules, input, es_url, es_index, threads, fuel, max_memory, max_stack, max_recursion_depth)
        }
    }
}

fn run_compile(
    modules: PathBuf,
    fuel: Option<u64>,
    max_memory: Option<usize>,
    max_stack: Option<usize>,
) -> Result<()> {
    tracing::info!("WADUP - Precompiling WASM Modules");
    tracing::info!("============================================");

    // Validate inputs
    if !modules.exists() || !modules.is_dir() {
        anyhow::bail!("Modules directory does not exist: {:?}", modules);
    }

    // Configure resource limits (affects engine hash)
    let limits = ResourceLimits {
        fuel,
        max_memory,
        max_stack,
    };

    tracing::info!("Configuration:");
    tracing::info!("  Modules directory: {:?}", modules);

    if let Some(fuel) = limits.fuel {
        tracing::info!("  Fuel limit: {}", fuel);
    }
    if let Some(mem) = limits.max_memory {
        tracing::info!("  Memory limit: {} bytes", mem);
    }
    if let Some(stack) = limits.max_stack {
        tracing::info!("  Stack limit: {} bytes", stack);
    }

    // Create runtime and load modules (this triggers precompilation)
    tracing::info!("Precompiling WASM modules...");
    let mut runtime = WasmRuntime::new(limits)?;
    runtime.load_modules(&modules)?;

    tracing::info!("============================================");
    tracing::info!("Precompilation complete!");

    Ok(())
}

fn run_process(
    modules: PathBuf,
    input: PathBuf,
    es_url: String,
    es_index: String,
    threads: usize,
    fuel: Option<u64>,
    max_memory: Option<usize>,
    max_stack: Option<usize>,
    max_recursion_depth: usize,
) -> Result<()> {
    tracing::info!("WADUP - Web Assembly Data Unified Processing");
    tracing::info!("============================================");

    // Validate inputs
    if !modules.exists() || !modules.is_dir() {
        anyhow::bail!("Modules directory does not exist: {:?}", modules);
    }

    if !input.exists() || !input.is_dir() {
        anyhow::bail!("Input directory does not exist: {:?}", input);
    }

    if threads == 0 {
        anyhow::bail!("Number of threads must be at least 1");
    }

    // Configure resource limits
    let limits = ResourceLimits {
        fuel,
        max_memory,
        max_stack,
    };

    tracing::info!("Configuration:");
    tracing::info!("  Modules directory: {:?}", modules);
    tracing::info!("  Input directory: {:?}", input);
    tracing::info!("  Elasticsearch URL: {}", es_url);
    tracing::info!("  Elasticsearch index: {}", es_index);
    tracing::info!("  Worker threads: {}", threads);
    tracing::info!("  Max recursion depth: {}", max_recursion_depth);

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

    // Load WASM modules (uses precompiled cache if available)
    tracing::info!("Loading WASM modules...");
    let mut runtime = WasmRuntime::new(limits)?;
    runtime.load_modules(&modules)?;

    // Create metadata store (connects to Elasticsearch)
    tracing::info!("Connecting to Elasticsearch...");
    let metadata_store = MetadataStore::new(&es_url, &es_index)?;

    // Load input files
    tracing::info!("Loading input files...");
    let contents = load_files(&input)?;
    tracing::info!("Found {} input files", contents.len());

    // Create processor
    let processor = ContentProcessor::new(
        runtime,
        metadata_store,
        max_recursion_depth,
    );

    // Process content
    tracing::info!("Starting processing...");
    processor.process(contents, threads)?;

    tracing::info!("============================================");
    tracing::info!("Processing complete! Results indexed to: {}/{}", es_url, es_index);

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
