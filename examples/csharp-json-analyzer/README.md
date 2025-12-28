# C# JSON Analyzer for WADUP

A JSON document analyzer that extracts structural metadata using .NET 8 with System.Text.Json library.

## Overview

This example demonstrates how to build WASM modules for WADUP using **.NET 8** with the Wasi.Sdk. It showcases:

- .NET 8 compilation to WASM using `Wasi.Sdk` NuGet package
- Single-file WASM bundling with `WasmSingleFileBundle`
- Command pattern with `_start` export (module reinstantiated per file)
- File-based metadata communication (no custom imports required)
- Using the shared `csharp-wadup-guest` library for metadata output

## What It Does

1. Reads JSON content from WADUP's virtual filesystem at `/data.bin`
2. Parses JSON using `System.Text.Json`
3. Analyzes structure: depth, keys, arrays, objects
4. Outputs results to `json_metadata` table with columns:
   - `max_depth` (Int64) - Maximum nesting depth
   - `total_keys` (Int64) - Total object keys found
   - `unique_keys` (Int64) - Unique object keys
   - `total_arrays` (Int64) - Number of arrays
   - `total_objects` (Int64) - Number of objects
   - `parser_used` (String) - Parser library used
   - `size_bytes` (Int64) - Input size
5. Outputs each unique key to `json_keys` table with columns:
   - `key_name` (String) - The key name
   - `occurrence_count` (Int64) - How many times it appears
6. Emits string values as sub-content (.txt files) for recursive processing

## Prerequisites

- **.NET 8 SDK** - .NET SDK with WASI workload
- **Make** - Build orchestration

### Install .NET WASI Workload

```bash
dotnet workload install wasi-experimental
```

## Building

```bash
make
```

Output: `target/csharp_json_analyzer.wasm` (~17 MB)

## Running

```bash
# Build WADUP CLI (if not already built)
cd ../../
cargo build --release

# Run on a directory containing JSON files
./target/release/wadup \
  --modules examples/csharp-json-analyzer/target \
  --input /path/to/json/files \
  --output results.db
```

## Architecture

### Command Pattern with `_start` Export

C# modules use the **command pattern** (module reinstantiated per file):

```csharp
public static int Main()
{
    try
    {
        Run();
        return 0;
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"Error: {ex.Message}");
        return 1;
    }
}
```

**How It Works:**

1. WADUP detects module has `_start` export (no `process` export)
2. WADUP reinstantiates module for each file
3. Module reads `/data.bin`, processes, writes to `/metadata/*.json`
4. WADUP reads metadata files and applies to database

### File-Based Metadata Communication

.NET WASI SDK doesn't support custom WASM imports. Instead, metadata is communicated via the virtual filesystem:

1. Module writes JSON to `/metadata/*.json`
2. When the file is closed, WADUP reads it immediately and deletes it
3. Any remaining files are processed after `_start` completes (fallback)
4. Metadata format matches WADUP's internal schema

```json
{
  "tables": [
    {"name": "table_name", "columns": [{"name": "col", "data_type": "Int64"}]}
  ],
  "rows": [
    {"table_name": "table_name", "values": [{"Int64": 42}]}
  ]
}
```

### WADUP Guest Library

Import the shared `csharp-wadup-guest` library:

```csharp
using CSharpWadupGuest;
```

The library provides:

**Table Builder Pattern:**
```csharp
var table = new TableBuilder("table_name")
    .AddColumn("col1", DataType.String)
    .AddColumn("col2", DataType.Int64)
    .Build();
```

**Insert Rows:**
```csharp
table.InsertRow(
    Value.FromString("value1"),
    Value.FromInt64(42)
);
```

**Flush Metadata:**
```csharp
// Must be called at the end to write metadata files
MetadataWriter.Flush();
```

**Emit Sub-Content:**
```csharp
// Emit data with streaming (zero-copy)
var emitter = SubContentWriter.Emit("extracted.bin");
emitter.Stream.Write(data, 0, data.Length);
emitter.Complete();

// Emit a slice of the input as sub-content (zero-copy, no data copied)
SubContentWriter.EmitSlice("embedded.dat", offset: 100, length: 500);
```

**Two emission modes (both zero-copy):**

1. **Streaming data** (`Emit`): Returns emitter for direct stream writing:
   - Write to `emitter.Stream` → `/subcontent/data_N.bin`
   - Call `emitter.Complete()` to close and trigger processing

2. **Slice reference** (`EmitSlice`): References input content range:
   - `/subcontent/metadata_N.json` - Filename + offset + length (no data file)

When the metadata file is closed, WADUP extracts the data without copying and queues it for recursive processing. To avoid infinite recursion, emit content that won't trigger your own module (e.g., emit `.txt` from a JSON analyzer).

**Data Types:**
- `DataType.Int64` - 64-bit signed integer
- `DataType.Float64` - 64-bit floating point
- `DataType.String` - UTF-8 string

## Key Learnings: C# + WADUP

### ✅ What Works

**1. .NET 8 with Wasi.Sdk**
- Add `Wasi.Sdk` NuGet package
- Works with standard C# syntax
- Full System.Text.Json support

**2. Single-File WASM Bundle**
- `<WasmSingleFileBundle>true</WasmSingleFileBundle>`
- Embeds .NET runtime and assemblies
- Single 17MB output file

**3. File-Based Metadata**
- Write to `/metadata/*.json`
- No custom WASM imports needed
- WADUP processes files immediately on close

**4. Zero-Copy Sub-Content**
- Owned data: Write to `/subcontent/data_N.bin` + `/subcontent/metadata_N.json`
- Slice reference: Write only `/subcontent/metadata_N.json` with offset/length
- WADUP extracts data without copying (BytesMut → Bytes freeze for owned, slice for references)
- Efficient recursive processing of extracted content

**5. Standard .NET Libraries**
- System.Text.Json works perfectly
- Newtonsoft.Json also works (included in project)
- Most pure-managed libraries work

### ❌ What Doesn't Work

**1. Custom WASM Imports**
- .NET WASI SDK only supports `wasi_snapshot_preview1` imports
- No custom module imports (like `env`)
- Use file-based metadata instead

**2. Reactor Pattern**
- .NET WASI modules use `_start` export only
- No `process` export support
- Module reinstantiated per file (slower but works)

**3. Some .NET Features**
- Reflection may have limitations
- Some APIs not available in WASI
- Test your specific needs

**4. Nullable Reference Types with Serialization**
- Disable `<Nullable>enable</Nullable>` in library projects
- NullableAttribute causes TypeLoadException
- Use explicit null handling instead

## Technical Details

### Build Process

The Makefile invokes `dotnet build`:

```makefile
$(TARGET): Program.cs JsonAnalyzer.cs CSharpJsonAnalyzer.csproj
	dotnet build -c Release
	cp $(SOURCE_WASM) $(TARGET)
```

### Project Configuration

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <RuntimeIdentifier>wasi-wasm</RuntimeIdentifier>
    <OutputType>Exe</OutputType>
    <WasmSingleFileBundle>true</WasmSingleFileBundle>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Wasi.Sdk" Version="0.1.4-preview.10061" />
  </ItemGroup>
</Project>
```

### Runtime Behavior

**File Processing:**
1. WADUP creates fresh WASM instance
2. Sets `/data.bin` content
3. Calls `_start()` (Main entry point)
4. Module processes file, writes to `/metadata/`
5. WADUP reads metadata and applies to database
6. Instance discarded

### Module Size

- **17 MB** - Includes .NET runtime and all bundled assemblies
- Larger than Rust (~2.5 MB) or Go (~8 MB)
- Tradeoff for .NET ecosystem and developer familiarity

### Performance

- Module reinstantiated per file (command pattern)
- .NET runtime initialized each time (~200ms overhead)
- Suitable for larger files where processing dominates
- Consider Rust/Go for very high volume scenarios

## Project Structure

```
examples/csharp-json-analyzer/
├── README.md                   # This file
├── Makefile                    # Build configuration
├── CSharpJsonAnalyzer.csproj   # C# project file
├── Program.cs                  # Entry point
├── JsonAnalyzer.cs            # Analysis logic
├── test.json                   # Test input
└── target/
    └── csharp_json_analyzer.wasm  # Compiled module

csharp-wadup-guest/            # Shared library
├── CSharpWadupGuest.csproj
├── Table.cs                   # TableBuilder, MetadataWriter
├── SubContent.cs              # SubContentWriter for sub-content emission
├── Types.cs                   # DataType, Value, Column
└── WadupException.cs
```

## Dependencies

### NuGet Packages

- `Wasi.Sdk` 0.1.4-preview.10061 - WASI compilation support
- `System.Text.Json` - JSON parsing (built-in)
- `Newtonsoft.Json` 13.0.3 - Alternative JSON library

### Project Reference

- `csharp-wadup-guest` - WADUP metadata library

## Comparison: C# vs Rust vs Python vs Go

| Feature | C# | Rust | Python | Go |
|---------|-----|------|--------|-----|
| Build Tool | dotnet build | cargo build | WASI SDK + make | go build |
| WASM Size | 17 MB | 2.5 MB | 20 MB | 8 MB |
| Build Time | ~15s | ~30s | ~5m (first) | ~10s |
| Entry Point | `_start` | `process` | `process` | `process` |
| Module Pattern | Command | Reactor | Reactor | Reactor |
| Per-File Overhead | ~200ms | ~0ms | ~0ms | ~0ms |
| Ecosystem | .NET | Crates | PyPI | Go modules |

**Use C# When:**
- You prefer C# and .NET ecosystem
- You have existing C# code to reuse
- Per-file overhead is acceptable
- You want familiar debugging/tooling

**Use Rust When:**
- You need smallest WASM size
- You want maximum performance
- You prefer Rust's type system

**Use Go When:**
- You want simple, fast builds
- Standard library access is important
- Module size is acceptable

**Use Python When:**
- You want fastest prototyping
- You need Python-specific libraries
- Build time is less critical

## Troubleshooting

### "Cannot access a closed file" Error

This can happen during initialization calls. Add null checks:
```csharp
if (string.IsNullOrEmpty(content))
    return;
```

### NullableAttribute TypeLoadException

Disable nullable reference types in library projects:
```xml
<Nullable>disable</Nullable>
```

### "Module missing required 'process' or '_start' export"

Ensure your project outputs an executable with Main:
```xml
<OutputType>Exe</OutputType>
```

### Build fails with missing Wasi.Sdk

Install the WASI workload:
```bash
dotnet workload install wasi-experimental
```

### Metadata not appearing in database

Ensure you call `MetadataWriter.Flush()` at the end of processing.

## Related Examples

- **`sqlite-parser`** - Rust module with reactor pattern
- **`go-sqlite-parser`** - Go module with reactor pattern
- **`python-sqlite-parser`** - Python module with reactor pattern
- **`byte-counter`** - Simple Rust module example

## Additional Resources

- [.NET WASI Documentation](https://github.com/AzCiS/dotnet-wasi-sdk)
- [Wasi.Sdk NuGet](https://www.nuget.org/packages/Wasi.Sdk)
- [System.Text.Json Documentation](https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-overview)
- [WADUP Guest Library](../../csharp-wadup-guest/)

## License

This example is part of the WADUP project.
