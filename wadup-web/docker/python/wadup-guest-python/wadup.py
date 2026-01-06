"""WADUP Guest Library for Python modules.

This module provides the API for WADUP modules to define metadata tables
and insert extracted data.
"""
import json
import os
from typing import List, Tuple, Any, Dict

# Type mapping
TYPE_MAP = {
    "String": str,
    "Int64": int,
    "Float64": float,
    "Bool": bool,
    "Bytes": bytes,
}

# Global state for tables
_tables: Dict[str, "Table"] = {}


class Table:
    """Represents a metadata table."""

    def __init__(self, name: str, columns: List[Tuple[str, str]]):
        self.name = name
        self.columns = columns
        self.rows: List[List[Any]] = []

    def insert_row(self, values: List[Any]) -> None:
        """Insert a row into the table."""
        if len(values) != len(self.columns):
            raise ValueError(
                f"Expected {len(self.columns)} values, got {len(values)}"
            )
        self.rows.append(values)

    def to_dict(self) -> dict:
        """Convert table to dictionary for serialization."""
        return {
            "name": self.name,
            "columns": [{"name": c[0], "type": c[1]} for c in self.columns],
            "rows": self.rows,
        }


def define_table(name: str, columns: List[Tuple[str, str]]) -> Table:
    """Define a new metadata table.

    Args:
        name: The table name
        columns: List of (column_name, column_type) tuples
                Types: "String", "Int64", "Float64", "Bool", "Bytes"

    Returns:
        The created Table object
    """
    table = Table(name, columns)
    _tables[name] = table
    return table


def insert_row(table_name: str, values: List[Any]) -> None:
    """Insert a row into a table.

    Args:
        table_name: The name of the table
        values: List of values matching the table's column definitions
    """
    if table_name not in _tables:
        raise ValueError(f"Table '{table_name}' not defined")
    _tables[table_name].insert_row(values)


def flush() -> None:
    """Write all table data to the metadata output file."""
    output = {
        "tables": [table.to_dict() for table in _tables.values()]
    }

    # Write to metadata output file
    try:
        with open("/metadata.json", "w") as f:
            json.dump(output, f)
    except Exception as e:
        print(f"Warning: Failed to write metadata: {e}", file=__import__("sys").stderr)


def get_content_path() -> str:
    """Get the path to the input file content.

    Returns:
        Path to the input file (typically /data.bin)
    """
    return "/data.bin"


def get_filename() -> str:
    """Get the original filename of the input file.

    Returns:
        The filename from WADUP_FILENAME environment variable
    """
    return os.environ.get("WADUP_FILENAME", "unknown")
