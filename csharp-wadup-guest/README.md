# C# WADUP Guest Library

A shared library for building WADUP WASM modules in C#/.NET.

## Overview

This library provides the interface between C# WASM modules and the WADUP host runtime. It handles:

- Table schema definition
- Row insertion
- Metadata serialization
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
├── Types.cs                  # DataType, Column, Value
└── WadupException.cs         # Exception class
```

## Related

- [csharp-json-analyzer](../examples/csharp-json-analyzer/) - Example C# module
- [go-wadup-guest](../go-wadup-guest/) - Go equivalent library
- [python-wadup-guest](../python-wadup-guest/) - Python equivalent library
- [wadup-guest](../crates/wadup-guest/) - Rust equivalent library
