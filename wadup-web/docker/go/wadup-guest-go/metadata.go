package wadup

import (
	"encoding/json"
	"fmt"
	"os"
	"sync"
)

// tableDef represents a table definition for serialization
type tableDef struct {
	Name    string   `json:"name"`
	Columns []Column `json:"columns"`
}

// rowDef represents a row for serialization
type rowDef struct {
	TableName string  `json:"table_name"`
	Values    []Value `json:"values"`
}

// metadataFile represents the complete metadata file structure
type metadataFile struct {
	Tables []tableDef `json:"tables"`
	Rows   []rowDef   `json:"rows"`
}

var (
	metadataMu      sync.Mutex
	accumulatedTabs []tableDef
	accumulatedRows []rowDef
	fileCounter     int
)

// addTable adds a table definition to the accumulated metadata
func addTable(name string, columns []Column) {
	metadataMu.Lock()
	defer metadataMu.Unlock()
	accumulatedTabs = append(accumulatedTabs, tableDef{
		Name:    name,
		Columns: columns,
	})
}

// addRow adds a row to the accumulated metadata
func addRow(tableName string, values []Value) {
	metadataMu.Lock()
	defer metadataMu.Unlock()
	accumulatedRows = append(accumulatedRows, rowDef{
		TableName: tableName,
		Values:    values,
	})
}

// Flush writes all accumulated metadata to a file.
//
// Writes to /metadata/output_N.json where N is an incrementing counter.
// The file is closed after writing, which triggers WADUP to read and process it.
//
// Returns nil if successful or if there's nothing to flush.
func Flush() error {
	metadataMu.Lock()
	defer metadataMu.Unlock()

	// Nothing to flush
	if len(accumulatedTabs) == 0 && len(accumulatedRows) == 0 {
		return nil
	}

	filename := fmt.Sprintf("/metadata/output_%d.json", fileCounter)
	fileCounter++

	metadata := metadataFile{
		Tables: accumulatedTabs,
		Rows:   accumulatedRows,
	}

	jsonData, err := json.Marshal(metadata)
	if err != nil {
		return fmt.Errorf("failed to serialize metadata: %w", err)
	}

	file, err := os.Create(filename)
	if err != nil {
		return fmt.Errorf("failed to create metadata file '%s': %w", filename, err)
	}
	defer file.Close()

	if _, err := file.Write(jsonData); err != nil {
		return fmt.Errorf("failed to write metadata file '%s': %w", filename, err)
	}

	// Clear accumulated data
	accumulatedTabs = nil
	accumulatedRows = nil

	return nil
}
