using CSharpWadupGuest;
using System.Text.Json;

public class Program
{
    private const string ContentPath = "/data.bin";

    private static void Run()
    {
        // Check if file exists
        if (!File.Exists(ContentPath))
        {
            return;
        }

        // Read content
        var content = File.ReadAllText(ContentPath);

        // Skip if no content (initialization call)
        if (string.IsNullOrEmpty(content))
        {
            return;
        }

        // Analyze JSON
        var analyzer = new JsonAnalyzer();
        var metadata = analyzer.Analyze(content);

        // Skip if not valid JSON
        if (metadata.ParserUsed == "none")
        {
            return;
        }

        // Define the main metadata table and flush immediately
        // This demonstrates that WADUP reads metadata when the file is closed
        var metadataTable = new TableBuilder("json_metadata")
            .AddColumn("max_depth", DataType.Int64)
            .AddColumn("total_keys", DataType.Int64)
            .AddColumn("unique_keys", DataType.Int64)
            .AddColumn("total_arrays", DataType.Int64)
            .AddColumn("total_objects", DataType.Int64)
            .AddColumn("parser_used", DataType.String)
            .AddColumn("size_bytes", DataType.Int64)
            .Build();

        // Insert main metadata row and flush (first flush)
        metadataTable.InsertRow(
            Value.FromInt64(metadata.MaxDepth),
            Value.FromInt64(metadata.TotalKeys),
            Value.FromInt64(metadata.UniqueKeys),
            Value.FromInt64(metadata.TotalArrays),
            Value.FromInt64(metadata.TotalObjects),
            Value.FromString(metadata.ParserUsed),
            Value.FromInt64(metadata.SizeBytes)
        );
        MetadataWriter.Flush();
        Console.Error.WriteLine("C#: Flushed metadata row 1 (main analysis)");

        // Define a second table for detailed key analysis
        var keysTable = new TableBuilder("json_keys")
            .AddColumn("key_name", DataType.String)
            .AddColumn("occurrence_count", DataType.Int64)
            .Build();

        // Insert rows for each unique key found, flushing after each batch
        // This demonstrates multiple incremental flushes
        var keys = analyzer.GetUniqueKeys();
        int batchNum = 2;
        foreach (var key in keys)
        {
            keysTable.InsertRow(
                Value.FromString(key.Key),
                Value.FromInt64(key.Value)
            );
            MetadataWriter.Flush();
            Console.Error.WriteLine($"C#: Flushed metadata row {batchNum} (key: {key.Key})");
            batchNum++;
        }

        Console.Error.WriteLine($"C#: All {batchNum - 1} metadata flushes complete");

        // Emit sub-content: extract string values as plain text files
        // This demonstrates file-based sub-content emission.
        // The sub-content is plain text, not JSON, so it won't trigger the JSON analyzer
        // and won't cause infinite recursion.
        EmitStringValues(content);
    }

    /// <summary>
    /// Extract string values from JSON and emit them as plain text sub-content.
    /// These are emitted as .txt files which won't be processed by the JSON analyzer.
    /// </summary>
    private static void EmitStringValues(string jsonContent)
    {
        try
        {
            using var doc = JsonDocument.Parse(jsonContent);
            EmitStringsFromElement(doc.RootElement, "root");
        }
        catch
        {
            // Not valid JSON, skip
        }
    }

    private static void EmitStringsFromElement(JsonElement element, string path)
    {
        switch (element.ValueKind)
        {
            case JsonValueKind.String:
                // Emit this string value as a plain text sub-content file
                var value = element.GetString();
                if (!string.IsNullOrEmpty(value))
                {
                    // Use .txt extension to make it clear this is not JSON
                    var filename = $"{path}.txt";
                    SubContentWriter.EmitText(filename, value);
                    SubContentWriter.Flush();
                    Console.Error.WriteLine($"C#: Emitted subcontent '{filename}' ({value.Length} bytes)");
                }
                break;

            case JsonValueKind.Object:
                foreach (var prop in element.EnumerateObject())
                {
                    EmitStringsFromElement(prop.Value, $"{path}_{prop.Name}");
                }
                break;

            case JsonValueKind.Array:
                int i = 0;
                foreach (var item in element.EnumerateArray())
                {
                    EmitStringsFromElement(item, $"{path}_{i}");
                    i++;
                }
                break;
        }
    }

    // Main entry point - returns int for WASM compatibility
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
}
