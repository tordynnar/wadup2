using System.Text.Json;
using Newtonsoft.Json.Linq;

public record JsonMetadata(
    long MaxDepth,
    long TotalKeys,
    long UniqueKeys,
    long TotalArrays,
    long TotalObjects,
    string ParserUsed,
    long SizeBytes
);

public class JsonAnalyzer
{
    public JsonMetadata Analyze(string json)
    {
        var sizeBytes = System.Text.Encoding.UTF8.GetByteCount(json);

        // Try System.Text.Json first (built-in, faster)
        try
        {
            using var doc = JsonDocument.Parse(json);
            return AnalyzeWithSystemTextJson(doc.RootElement, sizeBytes);
        }
        catch
        {
            // Fall back to Newtonsoft.Json (more permissive)
            try
            {
                var token = JToken.Parse(json);
                return AnalyzeWithNewtonsoft(token, sizeBytes);
            }
            catch
            {
                // Not valid JSON
                return new JsonMetadata(0, 0, 0, 0, 0, "none", sizeBytes);
            }
        }
    }

    private JsonMetadata AnalyzeWithSystemTextJson(JsonElement root, long size)
    {
        var uniqueKeys = new HashSet<string>();
        long totalKeys = 0;
        long maxDepth = 0;
        long arrays = 0;
        long objects = 0;

        void Traverse(JsonElement element, long depth)
        {
            maxDepth = Math.Max(maxDepth, depth);

            switch (element.ValueKind)
            {
                case JsonValueKind.Object:
                    objects++;
                    foreach (var prop in element.EnumerateObject())
                    {
                        uniqueKeys.Add(prop.Name);
                        totalKeys++;
                        Traverse(prop.Value, depth + 1);
                    }
                    break;

                case JsonValueKind.Array:
                    arrays++;
                    foreach (var item in element.EnumerateArray())
                        Traverse(item, depth + 1);
                    break;
            }
        }

        Traverse(root, 1);

        return new JsonMetadata(
            maxDepth,
            totalKeys,
            uniqueKeys.Count,
            arrays,
            objects,
            "System.Text.Json",
            size
        );
    }

    private JsonMetadata AnalyzeWithNewtonsoft(JToken root, long size)
    {
        var uniqueKeys = new HashSet<string>();
        long totalKeys = 0;
        long maxDepth = 0;
        long arrays = 0;
        long objects = 0;

        void Traverse(JToken token, long depth)
        {
            maxDepth = Math.Max(maxDepth, depth);

            if (token is JObject obj)
            {
                objects++;
                foreach (var prop in obj.Properties())
                {
                    uniqueKeys.Add(prop.Name);
                    totalKeys++;
                    Traverse(prop.Value, depth + 1);
                }
            }
            else if (token is JArray arr)
            {
                arrays++;
                foreach (var item in arr)
                    Traverse(item, depth + 1);
            }
        }

        Traverse(root, 1);

        return new JsonMetadata(
            maxDepth,
            totalKeys,
            uniqueKeys.Count,
            arrays,
            objects,
            "Newtonsoft.Json",
            size
        );
    }
}
