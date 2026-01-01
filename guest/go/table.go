package wadup

// Table represents a defined table that can accept row insertions
type Table struct {
	name string
}

// DefineTable defines a new table with the given columns
func DefineTable(name string, columns []Column) (*Table, error) {
	addTable(name, columns)
	return &Table{name: name}, nil
}

// InsertRow inserts a row of values into the table
func (t *Table) InsertRow(values []Value) error {
	addRow(t.name, values)
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
