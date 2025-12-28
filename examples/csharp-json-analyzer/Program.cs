using CSharpWadupGuest;

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

        // Define output table
        var table = new TableBuilder("json_metadata")
            .AddColumn("max_depth", DataType.Int64)
            .AddColumn("total_keys", DataType.Int64)
            .AddColumn("unique_keys", DataType.Int64)
            .AddColumn("total_arrays", DataType.Int64)
            .AddColumn("total_objects", DataType.Int64)
            .AddColumn("parser_used", DataType.String)
            .AddColumn("size_bytes", DataType.Int64)
            .Build();

        // Insert metadata row
        table.InsertRow(
            Value.FromInt64(metadata.MaxDepth),
            Value.FromInt64(metadata.TotalKeys),
            Value.FromInt64(metadata.UniqueKeys),
            Value.FromInt64(metadata.TotalArrays),
            Value.FromInt64(metadata.TotalObjects),
            Value.FromString(metadata.ParserUsed),
            Value.FromInt64(metadata.SizeBytes)
        );

        // Flush metadata to /metadata directory
        MetadataWriter.Flush();
        Console.Error.WriteLine("DEBUG: Metadata written successfully");
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
