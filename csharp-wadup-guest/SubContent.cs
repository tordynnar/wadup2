using System.Text.Json;

namespace CSharpWadupGuest;

/// <summary>
/// Emits sub-content for recursive processing by WADUP.
///
/// Sub-content can be emitted in two ways:
///
/// 1. Owned data (Emit): Returns an emitter with a Stream to write data directly (zero-copy):
///    - Write to the Stream property
///    - Call Complete() to close the stream and trigger WADUP processing
///
/// 2. Slice reference (EmitSlice): References a range of the input content (zero-copy):
///    - No data copying, just offset and length
///
/// WADUP reads the files when metadata is closed, then deletes them.
/// </summary>
public static class SubContentWriter
{
    private static int _fileCounter = 0;

    /// <summary>
    /// Begin emitting sub-content with direct stream access (zero-copy).
    /// Write data directly to the returned emitter's Stream property,
    /// then call Complete() to finish the emission.
    /// </summary>
    /// <param name="filename">The filename for the sub-content (used for identification)</param>
    /// <returns>An emitter with a Stream to write to and a Complete method to finish</returns>
    public static SubContentEmitter Emit(string filename)
    {
        int n = _fileCounter++;
        return new SubContentEmitter(filename, n);
    }

    /// <summary>
    /// Emit a slice of the input content as sub-content (zero-copy).
    /// The slice references a range of the original /data.bin content without copying.
    /// </summary>
    /// <param name="filename">The filename for the sub-content (used for identification)</param>
    /// <param name="offset">Byte offset into the input content</param>
    /// <param name="length">Number of bytes to include</param>
    public static void EmitSlice(string filename, long offset, long length)
    {
        int n = _fileCounter++;
        var metadataPath = $"/subcontent/metadata_{n}.json";

        try
        {
            using var metaFs = new FileStream(metadataPath, FileMode.Create, FileAccess.Write, FileShare.None);
            var writer = new Utf8JsonWriter(metaFs, new JsonWriterOptions { Indented = false });
            writer.WriteStartObject();
            writer.WriteString("filename", filename);
            writer.WriteNumber("offset", offset);
            writer.WriteNumber("length", length);
            writer.WriteEndObject();
            writer.Flush();
            writer.Dispose();
            metaFs.Flush();
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Failed to write subcontent slice for '{filename}': {ex.Message}");
        }
    }
}

/// <summary>
/// Handles streaming sub-content emission with zero-copy semantics.
/// Write directly to the Stream property, then call Complete() to finish.
/// </summary>
public class SubContentEmitter : IDisposable
{
    private readonly string _filename;
    private readonly int _n;
    private readonly FileStream _dataStream;
    private bool _finalized = false;
    private bool _disposed = false;

    internal SubContentEmitter(string filename, int n)
    {
        _filename = filename;
        _n = n;
        var dataPath = $"/subcontent/data_{n}.bin";
        _dataStream = new FileStream(dataPath, FileMode.Create, FileAccess.Write, FileShare.None);
    }

    /// <summary>
    /// The stream to write sub-content data to.
    /// Write directly to this stream for zero-copy emission.
    /// </summary>
    public Stream Stream => _dataStream;

    /// <summary>
    /// Complete the emission: closes the data stream and writes metadata to trigger WADUP processing.
    /// Must be called after writing all data to the Stream.
    /// </summary>
    public void Complete()
    {
        if (_finalized) return;
        _finalized = true;

        try
        {
            // Close data file first
            _dataStream.Flush();
            _dataStream.Dispose();

            // Write metadata file (triggers WADUP processing when closed)
            var metadataPath = $"/subcontent/metadata_{_n}.json";
            using var metaFs = new FileStream(metadataPath, FileMode.Create, FileAccess.Write, FileShare.None);
            var writer = new Utf8JsonWriter(metaFs, new JsonWriterOptions { Indented = false });
            writer.WriteStartObject();
            writer.WriteString("filename", _filename);
            writer.WriteEndObject();
            writer.Flush();
            writer.Dispose();
            metaFs.Flush();
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Failed to finalize subcontent for '{_filename}': {ex.Message}");
        }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        if (!_finalized)
        {
            // If not completed, just close the data stream without writing metadata
            // This effectively cancels the emission
            try { _dataStream.Dispose(); } catch { }
        }
    }
}
