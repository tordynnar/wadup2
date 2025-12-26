# SQLite Parser Example

A WADUP module that parses SQLite databases and extracts table statistics using SQL queries.

## Overview

This example demonstrates how to:
- Use the `rusqlite` library within a WASM module
- Execute SQL queries to extract metadata from SQLite databases
- Work with WASI (WebAssembly System Interface) for file system access
- Build WASM modules that require C dependencies (bundled SQLite)

## What It Does

The sqlite-parser module:
1. Detects if a file is a SQLite database by checking the header
2. Reads the database into memory
3. Uses `rusqlite` to execute SQL queries:
   - Queries `sqlite_master` to find all user tables
   - Executes `COUNT(*)` queries to get row counts for each table
4. Emits metadata into the `db_table_stats` table with:
   - `table_name`: Name of the table
   - `row_count`: Number of rows in the table

## Building

### Prerequisites

This module requires the WASI SDK to compile SQLite's C code to WebAssembly. The build script will automatically download it if not present.

**Manual WASI SDK Download:**
- **Linux x86_64**: https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz
- **Linux ARM64**: https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-arm64-linux.tar.gz
- **macOS x86_64**: https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-macos.tar.gz
- **macOS ARM64**: https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-arm64-macos.tar.gz
- **Windows**: https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-windows.tar.gz

More releases: https://github.com/WebAssembly/wasi-sdk/releases

### Using the Build Script

The easiest way to build is using the provided build script:

```bash
./build.sh
```

This script will:
1. Detect your platform (Linux/macOS, x86_64/ARM64)
2. Download WASI SDK 24.0 if not already present in `/tmp/wasi-sdk-*`
3. Add the `wasm32-wasip1` Rust target if needed
4. Build the module in release mode

The compiled WASM module will be at: `target/wasm32-wasip1/release/sqlite_parser.wasm`

### Manual Build

If you prefer to build manually:

1. **Install WASI SDK:**
   ```bash
   # Example for macOS ARM64
   cd /tmp
   curl -LO https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-arm64-macos.tar.gz
   tar -xzf wasi-sdk-24.0-arm64-macos.tar.gz
   ```

2. **Add Rust WASI target:**
   ```bash
   rustup target add wasm32-wasip1
   ```

3. **Build:**
   ```bash
   WASI_SDK_PATH=/tmp/wasi-sdk-24.0-arm64-macos \
   LIBSQLITE3_FLAGS="-DSQLITE_THREADSAFE=0" \
   cargo build --target wasm32-wasip1 --release
   ```

## Technical Details

### Why WASI?

This module uses WASI (wasm32-wasip1 target) instead of pure WebAssembly (wasm32-unknown-unknown) because:

1. **rusqlite with bundled SQLite** requires compiling C code, which needs:
   - Standard C library headers (stdio.h, etc.)
   - File system access for temporary files

2. **WASI provides:**
   - POSIX-like system calls
   - File system access (with pre-opened directories)
   - Standard I/O

### Threading Considerations

SQLite is compiled with `SQLITE_THREADSAFE=0` because:
- WASI doesn't support pthread operations in the same way as native platforms
- The WASM module runs in a single-threaded environment
- Thread-safe mode would require pthread_join and other threading primitives not available in WASI

This is configured in:
- `.cargo/config.toml`: Sets target features to disable atomics
- Build environment: `LIBSQLITE3_FLAGS` disables thread-safe mode

### File System Access

The module writes the database to `/tmp/temp_db.sqlite` temporarily. The WADUP runtime pre-opens the `/tmp` directory with read/write permissions for WASI modules, allowing this temporary file creation.

## Dependencies

- **wadup-guest**: WADUP guest library for WASM modules
- **rusqlite**: Rust SQLite library (with bundled SQLite C library)
- **WASI SDK**: Toolchain for compiling C to WebAssembly

## Example Usage

```bash
# Build the module
./build.sh

# Run with WADUP CLI
wadup --modules /path/to/modules \
      --input /path/to/databases \
      --output results.db
```

The module will process any SQLite database files in the input directory and emit table statistics to the output database.

## Troubleshooting

### "pthread_join not defined" error

This means SQLite was compiled with thread support enabled. Make sure:
- `LIBSQLITE3_FLAGS="-DSQLITE_THREADSAFE=0"` is set during build
- `.cargo/config.toml` is present with the correct target features

### "Failed to find a pre-opened file descriptor" error

This means the WASI runtime doesn't have `/tmp` pre-opened. The WADUP runtime handles this automatically, but if running in a different WASI environment, you may need to configure directory permissions.

### WASI SDK not found

The build script should download it automatically. If it fails:
1. Check your internet connection
2. Manually download from the URLs above
3. Extract to `/tmp/wasi-sdk-24.0-{arch}-{os}`
4. Set `WASI_SDK_PATH` environment variable

## License

This example is part of the WADUP project.
