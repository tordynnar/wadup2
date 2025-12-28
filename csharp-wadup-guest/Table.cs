using System.Text.Json;

namespace CSharpWadupGuest;

/// <summary>
/// Represents a table that can have rows inserted into it.
/// </summary>
public class Table
{
    private readonly string _name;

    internal Table(string name) => _name = name;

    /// <summary>
    /// Insert a row with the given values into this table.
    /// </summary>
    public void InsertRow(params Value[] values)
    {
        MetadataWriter.AddRow(_name, values);
    }
}

/// <summary>
/// Builder for creating table definitions.
/// </summary>
public class TableBuilder
{
    private readonly string _name;
    private readonly List<Column> _columns = new();

    public TableBuilder(string name) => _name = name;

    public TableBuilder AddColumn(string name, DataType dataType)
    {
        _columns.Add(new Column(name, dataType));
        return this;
    }

    public Table Build()
    {
        MetadataWriter.AddTable(_name, _columns.ToArray());
        return new Table(_name);
    }
}

/// <summary>
/// Accumulates metadata (table definitions and rows) and writes to /metadata/*.json files.
/// Call Flush() at the end of processing to ensure all metadata is written.
/// </summary>
public static class MetadataWriter
{
    private static readonly List<TableDef> _tables = new();
    private static readonly List<RowDef> _rows = new();
    private static int _fileCounter = 0;

    internal class TableDef
    {
        public string Name { get; set; }
        public Column[] Columns { get; set; }
    }

    internal class RowDef
    {
        public string TableName { get; set; }
        public Value[] Values { get; set; }
    }

    internal static void AddTable(string name, Column[] columns)
    {
        _tables.Add(new TableDef { Name = name, Columns = columns });
    }

    internal static void AddRow(string tableName, Value[] values)
    {
        _rows.Add(new RowDef { TableName = tableName, Values = values });
    }

    /// <summary>
    /// Flush all accumulated metadata to /metadata/output.json.
    /// Must be called at the end of processing.
    /// </summary>
    public static void Flush()
    {
        if (_tables.Count == 0 && _rows.Count == 0)
            return;

        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = false }))
        {
            writer.WriteStartObject();

            // Write tables array
            writer.WritePropertyName("tables");
            writer.WriteStartArray();
            foreach (var table in _tables)
            {
                writer.WriteStartObject();
                writer.WriteString("name", table.Name);
                writer.WritePropertyName("columns");
                writer.WriteStartArray();
                foreach (var col in table.Columns)
                {
                    writer.WriteStartObject();
                    writer.WriteString("name", col.Name);
                    writer.WriteString("data_type", col.DataType.ToString());
                    writer.WriteEndObject();
                }
                writer.WriteEndArray();
                writer.WriteEndObject();
            }
            writer.WriteEndArray();

            // Write rows array
            writer.WritePropertyName("rows");
            writer.WriteStartArray();
            foreach (var row in _rows)
            {
                writer.WriteStartObject();
                writer.WriteString("table_name", row.TableName);
                writer.WritePropertyName("values");
                writer.WriteStartArray();
                foreach (var val in row.Values)
                {
                    val.WriteTo(writer);
                }
                writer.WriteEndArray();
                writer.WriteEndObject();
            }
            writer.WriteEndArray();

            writer.WriteEndObject();
        }

        // Write to /metadata/output_{counter}.json using FileStream for WASI compatibility
        var filename = $"/metadata/output_{_fileCounter++}.json";
        var bytes = stream.ToArray();

        try
        {
            using var fs = new FileStream(filename, FileMode.Create, FileAccess.Write, FileShare.None);
            fs.Write(bytes, 0, bytes.Length);
            fs.Flush();
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Failed to write metadata file: {ex.Message}");
        }

        // Clear for potential reuse
        _tables.Clear();
        _rows.Clear();
    }
}
