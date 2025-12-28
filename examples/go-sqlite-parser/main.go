package main

import (
	"database/sql"
	"fmt"
	"os"

	_ "github.com/ncruces/go-sqlite3/driver"
	_ "github.com/ncruces/go-sqlite3/embed"
	"github.com/tordynnar/wadup2/go-wadup-guest"
)

const ContentPath = "/data.bin"

// TableStat holds statistics about a single table
type TableStat struct {
	TableName string
	RowCount  int64
}

func main() {
	// For standard Go WASM modules called via _start (reload-per-call):
	// Put the module logic directly in main()
	// The wadup runtime will reload this module for each file processed
	if err := run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		return
	}
}

func run() error {
	// Check if file is SQLite database
	isSQLite, err := isSQLiteDatabase()
	if err != nil {
		return err
	}
	if !isSQLite {
		// Not a SQLite database, silently skip
		return nil
	}

	// Open database using database/sql with pure Go SQLite driver
	// Use file URI with immutable and read-only mode for WASI compatibility
	db, err := sql.Open("sqlite3", "file:"+ContentPath+"?mode=ro&immutable=1")
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	// Query for user tables and count rows
	stats, err := executeQueries(db)
	if err != nil {
		return err
	}

	// Define output table
	table, err := wadup.NewTableBuilder("db_table_stats").
		Column("table_name", wadup.String).
		Column("row_count", wadup.Int64).
		Build()
	if err != nil {
		return err
	}

	// Insert statistics
	for _, stat := range stats {
		err := table.InsertRow([]wadup.Value{
			wadup.NewString(stat.TableName),
			wadup.NewInt64(stat.RowCount),
		})
		if err != nil {
			return err
		}
	}

	return nil
}

func isSQLiteDatabase() (bool, error) {
	f, err := os.Open(ContentPath)
	if err != nil {
		return false, err
	}
	defer f.Close()

	header := make([]byte, 16)
	n, err := f.Read(header)
	if err != nil || n < 16 {
		return false, nil
	}

	expected := "SQLite format 3\x00"
	return string(header) == expected, nil
}

func executeQueries(db *sql.DB) ([]TableStat, error) {
	var stats []TableStat

	// Query for user tables
	rows, err := db.Query(
		"SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
	)
	if err != nil {
		return nil, fmt.Errorf("failed to query tables: %w", err)
	}
	defer rows.Close()

	var tableNames []string
	for rows.Next() {
		var tableName string
		if err := rows.Scan(&tableName); err != nil {
			return nil, fmt.Errorf("failed to scan table name: %w", err)
		}
		tableNames = append(tableNames, tableName)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating tables: %w", err)
	}

	// Count rows in each table
	for _, tableName := range tableNames {
		var count int64
		err := db.QueryRow(
			fmt.Sprintf(`SELECT COUNT(*) FROM "%s"`, tableName),
		).Scan(&count)
		if err != nil {
			return nil, fmt.Errorf("failed to count rows in %s: %w", tableName, err)
		}

		stats = append(stats, TableStat{
			TableName: tableName,
			RowCount:  count,
		})
	}

	return stats, nil
}
