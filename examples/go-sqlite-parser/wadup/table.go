package wadup

import (
	"encoding/json"
	"fmt"
)

// Table represents a defined table that can accept row insertions
type Table struct {
	name string
}

// DefineTable defines a new table with the given columns
func DefineTable(name string, columns []Column) (*Table, error) {
	// Serialize columns to JSON
	columnsJSON, err := json.Marshal(columns)
	if err != nil {
		return nil, fmt.Errorf("failed to serialize columns: %w", err)
	}

	// Call FFI
	namePtr, nameLen := stringToFFI(name)
	colPtr, colLen := stringToFFI(string(columnsJSON))

	result := defineTableFFI(namePtr, nameLen, colPtr, colLen)
	if result < 0 {
		return nil, fmt.Errorf("failed to define table '%s': error code %d", name, result)
	}

	return &Table{name: name}, nil
}

// InsertRow inserts a row of values into the table
func (t *Table) InsertRow(values []Value) error {
	// Serialize values to JSON
	valuesJSON, err := json.Marshal(values)
	if err != nil {
		return fmt.Errorf("failed to serialize values: %w", err)
	}

	// Call FFI
	namePtr, nameLen := stringToFFI(t.name)
	valPtr, valLen := stringToFFI(string(valuesJSON))

	result := insertRowFFI(namePtr, nameLen, valPtr, valLen)
	if result < 0 {
		return fmt.Errorf("failed to insert row into '%s': error code %d", t.name, result)
	}

	return nil
}

// TableBuilder provides a fluent API for building tables
type TableBuilder struct {
	name    string
	columns []Column
}

// NewTableBuilder creates a new table builder
func NewTableBuilder(name string) *TableBuilder {
	return &TableBuilder{
		name:    name,
		columns: make([]Column, 0),
	}
}

// Column adds a column to the table
func (b *TableBuilder) Column(name string, dataType DataType) *TableBuilder {
	b.columns = append(b.columns, Column{
		Name:     name,
		DataType: dataType,
	})
	return b
}

// Build creates the table
func (b *TableBuilder) Build() (*Table, error) {
	return DefineTable(b.name, b.columns)
}
