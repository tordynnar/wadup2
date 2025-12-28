package wadup

import (
	"encoding/json"
	"fmt"
)

// DataType represents the type of data in a column
type DataType string

const (
	Int64   DataType = "Int64"
	Float64 DataType = "Float64"
	String  DataType = "String"
)

// Column represents a column definition in a table
type Column struct {
	Name     string   `json:"name"`
	DataType DataType `json:"data_type"`
}

// Value represents a value that can be inserted into a table
type Value struct {
	data interface{}
}

// NewInt64 creates a new Int64 value
func NewInt64(v int64) Value {
	return Value{data: v}
}

// NewFloat64 creates a new Float64 value
func NewFloat64(v float64) Value {
	return Value{data: v}
}

// NewString creates a new String value
func NewString(v string) Value {
	return Value{data: v}
}

// MarshalJSON implements custom JSON encoding for Value
// Encodes as a tagged union: {"Int64": 42}, {"String": "foo"}, etc.
func (v Value) MarshalJSON() ([]byte, error) {
	switch val := v.data.(type) {
	case int64:
		return json.Marshal(map[string]int64{"Int64": val})
	case float64:
		return json.Marshal(map[string]float64{"Float64": val})
	case string:
		return json.Marshal(map[string]string{"String": val})
	default:
		return nil, fmt.Errorf("unsupported value type: %T", val)
	}
}
