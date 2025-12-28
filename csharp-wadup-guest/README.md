# C# WADUP Guest Library

A shared library for building WADUP WASM modules in C#/.NET.

## Overview

This library provides the interface between C# WASM modules and the WADUP host runtime. It handles:

- Table schema definition
- Row insertion
- Metadata serialization
- Sub-content emission (zero-copy streaming and slice references)
- File-based communication with WADUP

## Installation

Reference the project in your `.csproj` file:

```xml
<ItemGroup>
  <ProjectReference Include="../../csharp-wadup-guest/CSharpWadupGuest.csproj" />
</ItemGroup>
```

## Usage

### Basic Example

```csharp
using CSharpWadupGuest;

public static void Main()
{
    // Read input
    var content = File.ReadAllText("/data.bin");

    // Define table schema
    var table = new TableBuilder("my_table")
        .AddColumn("name", DataType.String)
        .AddColumn("count", DataType.Int64)
        .AddColumn("score", DataType.Float64)
        .Build();

    // Insert rows
    table.InsertRow(
        Value.FromString("example"),
        Value.FromInt64(42),
        Value.FromFloat64(3.14)
    );

    // IMPORTANT: Flush metadata before exit
    MetadataWriter.Flush();
}
```

### TableBuilder

Create table schemas using the fluent builder pattern:

```csharp
var table = new TableBuilder("table_name")
    .AddColumn("column1", DataType.String)
    .AddColumn("column2", DataType.Int64)
    .AddColumn("column3", DataType.Float64)
    .Build();
```

Supported data types:
- `DataType.Int64` - 64-bit signed integer
- `DataType.Float64` - 64-bit floating point
- `DataType.String` - UTF-8 string

### Value Types

Create values using static factory methods:

```csharp
Value.FromInt64(123)
Value.FromFloat64(3.14159)
Value.FromString("hello")
```

### InsertRow

Insert rows with values matching the table schema:

```csharp
table.InsertRow(
    Value.FromString("data"),
    Value.FromInt64(100),
    Value.FromFloat64(99.9)
);
```

### MetadataWriter.Flush()

**Critical:** Call `Flush()` at the end of your processing to ensure metadata is written:

```csharp
// At end of processing
MetadataWriter.Flush();
```

This writes the accumulated metadata to `/metadata/output_0.json` for WADUP to read.

### SubContentWriter (Sub-Content Emission)

Emit sub-content for recursive processing by WADUP:

```csharp
using CSharpWadupGuest;

// Emit data with streaming (zero-copy)
var emitter = SubContentWriter.Emit("extracted.bin");
emitter.Stream.Write(data, 0, data.Length);  // Write directly to the stream
emitter.Complete();  // Close stream and trigger WADUP processing

// Emit a slice of the input as sub-content (zero-copy, no data copied)
SubContentWriter.EmitSlice("embedded.dat", offset: 100, length: 500);
```

**Two emission modes (both zero-copy):**

1. **Streaming data** (`Emit`): Returns an emitter for direct stream writing:
   - Write to `emitter.Stream` (writes directly to `/subcontent/data_N.bin`)
   - Call `emitter.Complete()` to close data and write metadata

2. **Slice reference** (`EmitSlice`): References a range of the input content:
   - No data copying, just offset and length in metadata

When the metadata file is closed, WADUP:
1. For streaming data: Extracts the data file as `Bytes` without copying
2. For slice: Uses the offset/length to reference the parent content directly
3. Queues the sub-content for recursive processing by all modules

The data flows from your WASM write directly to nested processing without any memory copies.

**Avoiding Infinite Recursion:**

If your module might process its own emitted sub-content, ensure the sub-content is distinguishable:
- Use different file extensions (e.g., emit `.txt` from a JSON analyzer)
- Check content signatures at the start of processing
- Use parent UUID tracking in the database to detect recursion

## How It Works

Unlike Rust, Go, or Python modules which use FFI imports, C# modules use file-based communication:

1. Your module defines tables and inserts rows
2. Metadata is accumulated in memory
3. `Flush()` writes JSON to `/metadata/*.json`
4. When the file is closed, WADUP reads it immediately and deletes it
5. Any files not closed are processed after `_start` completes (fallback)
6. WADUP applies the schema and data to the output database

### Metadata Format

The library generates JSON matching WADUP's expected format:

```json
{
  "tables": [
    {
      "name": "table_name",
      "columns": [
        {"name": "col1", "data_type": "String"},
        {"name": "col2", "data_type": "Int64"}
      ]
    }
  ],
  "rows": [
    {
      "table_name": "table_name",
      "values": [
        {"String": "value"},
        {"Int64": 42}
      ]
    }
  ]
}
```

## Project Configuration

### Required Settings

Your C# WASM project needs these settings:

```xml
<PropertyGroup>
  <TargetFramework>net8.0</TargetFramework>
  <RuntimeIdentifier>wasi-wasm</RuntimeIdentifier>
  <OutputType>Exe</OutputType>
  <WasmSingleFileBundle>true</WasmSingleFileBundle>
</PropertyGroup>

<ItemGroup>
  <PackageReference Include="Wasi.Sdk" Version="0.1.4-preview.10061" />
</ItemGroup>
```

### Guest Library Settings

The guest library itself uses:

```xml
<PropertyGroup>
  <TargetFramework>net8.0</TargetFramework>
  <Nullable>disable</Nullable>
  <ImplicitUsings>enable</ImplicitUsings>
  <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
</PropertyGroup>
```

Note: `Nullable` is disabled to avoid TypeLoadException in WASI.

## API Reference

### Classes

**TableBuilder**
- `TableBuilder(string name)` - Create a new table builder
- `AddColumn(string name, DataType type)` - Add a column (fluent)
- `Build()` - Create the table and register schema

**Table**
- `InsertRow(params Value[] values)` - Insert a row

**MetadataWriter**
- `Flush()` - Write all accumulated metadata to file

**SubContentWriter**
- `Emit(string filename)` - Begin streaming emission, returns `SubContentEmitter`
- `EmitSlice(string filename, long offset, long length)` - Emit slice of input (zero-copy)

**SubContentEmitter**
- `Stream` - Stream to write data to (writes directly to filesystem)
- `Complete()` - Close stream and write metadata to trigger WADUP processing
- `Dispose()` - Cancel emission if not completed

### Enums

**DataType**
- `Int64` - 64-bit signed integer
- `Float64` - 64-bit floating point
- `String` - UTF-8 string

### Value Factory Methods

- `Value.FromInt64(long value)`
- `Value.FromFloat64(double value)`
- `Value.FromString(string value)`

## Limitations

### No Custom WASM Imports

.NET WASI SDK doesn't support custom WASM imports. This library uses file-based metadata instead of FFI calls like other language implementations.

### Command Pattern Only

C# WASM modules use the `_start` entry point and are reinstantiated for each file. Unlike Rust/Go/Python which use the reactor pattern (module reuse), C# modules have per-file initialization overhead.

### Nullable Reference Types

Disabled in this library due to NullableAttribute TypeLoadException in WASI runtime. Use explicit null checks instead.

## Example Project

See `examples/csharp-json-analyzer/` for a complete example using this library.

## Files

```
csharp-wadup-guest/
├── README.md                 # This file
├── CSharpWadupGuest.csproj   # Project file
├── Table.cs                  # TableBuilder, Table, MetadataWriter
├── SubContent.cs             # SubContentWriter, SubContentEmitter (zero-copy emission)
├── Types.cs                  # DataType, Column, Value
└── WadupException.cs         # Exception class
```

## Related

- [csharp-json-analyzer](../examples/csharp-json-analyzer/) - Example C# module
- [go-wadup-guest](../go-wadup-guest/) - Go equivalent library
- [python-wadup-guest](../python-wadup-guest/) - Python equivalent library
- [wadup-guest](../crates/wadup-guest/) - Rust equivalent library
