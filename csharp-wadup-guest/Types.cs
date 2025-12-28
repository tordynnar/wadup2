using System.Text.Json;
using System.Text.Json.Serialization;

namespace CSharpWadupGuest;

public enum DataType
{
    Int64,
    Float64,
    String
}

public class Column
{
    [JsonPropertyName("name")]
    public string Name { get; set; }

    [JsonPropertyName("data_type")]
    public string DataTypeString => DataType.ToString();

    [JsonIgnore]
    public DataType DataType { get; set; }

    public Column(string name, DataType dataType)
    {
        Name = name;
        DataType = dataType;
    }
}

/// <summary>
/// Value types using tagged union pattern compatible with wadup's Rust serialization.
/// Each value is serialized as {"TypeName": data} e.g. {"Int64": 42}
/// </summary>
public abstract class Value
{
    public abstract void WriteTo(Utf8JsonWriter writer);

    public static Value FromInt64(long value) => new Int64Value(value);
    public static Value FromFloat64(double value) => new Float64Value(value);
    public static Value FromString(string value) => new StringValue(value);

    private sealed class Int64Value : Value
    {
        private readonly long _data;
        public Int64Value(long data) => _data = data;
        public override void WriteTo(Utf8JsonWriter writer)
        {
            writer.WriteStartObject();
            writer.WriteNumber("Int64", _data);
            writer.WriteEndObject();
        }
    }

    private sealed class Float64Value : Value
    {
        private readonly double _data;
        public Float64Value(double data) => _data = data;
        public override void WriteTo(Utf8JsonWriter writer)
        {
            writer.WriteStartObject();
            writer.WriteNumber("Float64", _data);
            writer.WriteEndObject();
        }
    }

    private sealed class StringValue : Value
    {
        private readonly string _data;
        public StringValue(string data) => _data = data;
        public override void WriteTo(Utf8JsonWriter writer)
        {
            writer.WriteStartObject();
            writer.WriteString("String", _data);
            writer.WriteEndObject();
        }
    }
}
