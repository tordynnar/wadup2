// Package wadup provides the WADUP guest library for Go modules.
package wadup

import (
	"encoding/json"
	"fmt"
	"os"
)

// DataType represents the type of a column value
type DataType int

const (
	String DataType = iota
	Int64
	Float64
	Bool
	Bytes
)

// Value represents a typed value for a table row
type Value struct {
	Type DataType
	data interface{}
}

// NewString creates a new string value
func NewString(s string) Value {
	return Value{Type: String, data: s}
}

// NewInt64 creates a new int64 value
func NewInt64(i int64) Value {
	return Value{Type: Int64, data: i}
}

// NewFloat64 creates a new float64 value
func NewFloat64(f float64) Value {
	return Value{Type: Float64, data: f}
}

// NewBool creates a new bool value
func NewBool(b bool) Value {
	return Value{Type: Bool, data: b}
}

// NewBytes creates a new bytes value
func NewBytes(b []byte) Value {
	return Value{Type: Bytes, data: b}
}

// Column represents a table column definition
type Column struct {
	Name string   `json:"name"`
	Type DataType `json:"type"`
}

// TableBuilder helps construct table definitions
type TableBuilder struct {
	name    string
	columns []Column
}

// Table represents a metadata table
type Table struct {
	Name    string   `json:"name"`
	Columns []Column `json:"columns"`
	Rows    [][]interface{} `json:"rows"`
}

// NewTableBuilder creates a new table builder
func NewTableBuilder(name string) *TableBuilder {
	return &TableBuilder{name: name, columns: []Column{}}
}

// Column adds a column to the table definition
func (tb *TableBuilder) Column(name string, dataType DataType) *TableBuilder {
	tb.columns = append(tb.columns, Column{Name: name, Type: dataType})
	return tb
}

// Build finalizes the table definition
func (tb *TableBuilder) Build() (*Table, error) {
	if tb.name == "" {
		return nil, fmt.Errorf("table name is required")
	}
	if len(tb.columns) == 0 {
		return nil, fmt.Errorf("at least one column is required")
	}

	table := &Table{
		Name:    tb.name,
		Columns: tb.columns,
		Rows:    [][]interface{}{},
	}

	// Register table globally
	tables = append(tables, table)

	return table, nil
}

// InsertRow inserts a row into the table
func (t *Table) InsertRow(values []Value) error {
	if len(values) != len(t.Columns) {
		return fmt.Errorf("expected %d values, got %d", len(t.Columns), len(values))
	}

	row := make([]interface{}, len(values))
	for i, v := range values {
		row[i] = v.data
	}
	t.Rows = append(t.Rows, row)
	return nil
}

// Global state
var tables []*Table

// Flush writes all metadata tables to the output file
func Flush() {
	output := map[string]interface{}{
		"tables": tables,
	}

	data, err := json.Marshal(output)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to marshal output: %v\n", err)
		return
	}

	// Write to the metadata output file
	err = os.WriteFile("/metadata.json", data, 0644)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to write metadata: %v\n", err)
	}
}
