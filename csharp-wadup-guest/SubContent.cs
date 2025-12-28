using System.Text.Json;

namespace CSharpWadupGuest;

/// <summary>
/// Emits sub-content for recursive processing by WADUP.
///
/// Sub-content is written as paired files:
/// - /subcontent/data_N.bin - Raw binary data (no encoding overhead)
/// - /subcontent/metadata_N.json - Filename metadata
///
/// WADUP reads both files when the metadata file is closed, then deletes them.
/// </summary>
public static class SubContentWriter
{
    private static readonly List<EmissionDef> _emissions = new();
    private static int _fileCounter = 0;

    internal class EmissionDef
    {
        public string Filename { get; set; }
        public byte[] Data { get; set; }
    }

    /// <summary>
    /// Queue a sub-content emission for processing.
    /// Call Flush() to actually write the emission to WADUP.
    /// </summary>
    /// <param name="filename">The filename for the sub-content (used for identification)</param>
    /// <param name="data">The raw bytes to emit as sub-content</param>
    public static void Emit(string filename, byte[] data)
    {
        _emissions.Add(new EmissionDef { Filename = filename, Data = data });
    }

    /// <summary>
    /// Queue a sub-content emission from a string.
    /// The string will be encoded as UTF-8 bytes.
    /// </summary>
    /// <param name="filename">The filename for the sub-content</param>
    /// <param name="text">The text to emit as sub-content</param>
    public static void EmitText(string filename, string text)
    {
        Emit(filename, System.Text.Encoding.UTF8.GetBytes(text));
    }

    /// <summary>
    /// Flush all queued sub-content emissions.
    /// Each emission writes two files:
    /// - /subcontent/data_N.bin (raw data, closed first)
    /// - /subcontent/metadata_N.json (filename, closed last - triggers WADUP processing)
    /// </summary>
    public static void Flush()
    {
        foreach (var emission in _emissions)
        {
            WriteEmission(emission, _fileCounter++);
        }
        _emissions.Clear();
    }

    private static void WriteEmission(EmissionDef emission, int n)
    {
        var dataPath = $"/subcontent/data_{n}.bin";
        var metadataPath = $"/subcontent/metadata_{n}.json";

        try
        {
            // Write data file first (raw bytes, no encoding)
            using (var dataFs = new FileStream(dataPath, FileMode.Create, FileAccess.Write, FileShare.None))
            {
                dataFs.Write(emission.Data, 0, emission.Data.Length);
                dataFs.Flush();
            }
            // Data file is now closed

            // Write metadata file second (triggers WADUP processing when closed)
            using (var metaFs = new FileStream(metadataPath, FileMode.Create, FileAccess.Write, FileShare.None))
            {
                var writer = new Utf8JsonWriter(metaFs, new JsonWriterOptions { Indented = false });
                writer.WriteStartObject();
                writer.WriteString("filename", emission.Filename);
                writer.WriteEndObject();
                writer.Flush();
                writer.Dispose();
                metaFs.Flush();
            }
            // Metadata file close triggers WADUP to read both files
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Failed to write subcontent files for '{emission.Filename}': {ex.Message}");
        }
    }
}
