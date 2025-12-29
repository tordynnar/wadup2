"""WADUP metadata API for Python WASM modules.

This module provides functions for defining metadata tables and emitting
sub-content from Python WASM modules running under WADUP.

Example:
    import wadup

    wadup.define_table("my_table", [
        ("name", "String"),
        ("count", "Int64"),
    ])

    wadup.insert_row("my_table", ["example", 42])
    wadup.flush()

Note: This module avoids using the `json` module because it requires
`copyreg` which isn't frozen in the Python WASI build.
"""
import os

# Metadata accumulation
_tables = []
_rows = []
_flush_counter = 0


def _escape_string(s):
    """Escape a string for JSON output."""
    result = []
    for c in s:
        if c == '"':
            result.append('\\"')
        elif c == '\\':
            result.append('\\\\')
        elif c == '\n':
            result.append('\\n')
        elif c == '\r':
            result.append('\\r')
        elif c == '\t':
            result.append('\\t')
        else:
            result.append(c)
    return ''.join(result)


def define_table(name, columns):
    """Define a metadata table.

    Args:
        name: Table name (string)
        columns: List of (column_name, column_type) tuples.
                 Supported types: "String", "Int64", "Float64"

    Example:
        wadup.define_table("files", [
            ("filename", "String"),
            ("size", "Int64"),
        ])
    """
    # Build columns JSON
    cols_json = []
    for col_name, col_type in columns:
        cols_json.append(f'{{"name":"{_escape_string(col_name)}","data_type":"{_escape_string(col_type)}"}}')

    _tables.append({
        "name": name,
        "columns_json": "[" + ",".join(cols_json) + "]"
    })


def insert_row(table_name, values):
    """Insert a row into a previously defined table.

    Args:
        table_name: Name of the target table
        values: List of values (int, float, or str)

    Example:
        wadup.insert_row("files", ["readme.txt", 1024])
    """
    # Build values JSON
    vals_json = []
    for v in values:
        if isinstance(v, bool):
            # bool must be checked before int since bool is a subclass of int
            vals_json.append(f'{{"Int64":{1 if v else 0}}}')
        elif isinstance(v, int):
            vals_json.append(f'{{"Int64":{v}}}')
        elif isinstance(v, float):
            vals_json.append(f'{{"Float64":{v}}}')
        else:
            vals_json.append(f'{{"String":"{_escape_string(str(v))}"}}')

    _rows.append({
        "table_name": table_name,
        "values_json": "[" + ",".join(vals_json) + "]"
    })


def flush():
    """Flush accumulated metadata to file.

    Writes all accumulated table definitions and rows to a JSON file
    in /metadata/output_N.json. The file is processed by WADUP when closed.

    This function clears the accumulated data after writing.
    """
    global _flush_counter, _tables, _rows

    if not _tables and not _rows:
        return

    os.makedirs("/metadata", exist_ok=True)

    # Build JSON manually (avoiding json module which requires copyreg)
    tables_json = []
    for t in _tables:
        tables_json.append(f'{{"name":"{_escape_string(t["name"])}","columns":{t["columns_json"]}}}')

    rows_json = []
    for r in _rows:
        rows_json.append(f'{{"table_name":"{_escape_string(r["table_name"])}","values":{r["values_json"]}}}')

    json_output = '{"tables":[' + ",".join(tables_json) + '],"rows":[' + ",".join(rows_json) + ']}'

    with open(f"/metadata/output_{_flush_counter}.json", "w") as f:
        f.write(json_output)

    _flush_counter += 1
    _tables = []
    _rows = []


# Sub-content emission
_subcontent_counter = 0


def emit_bytes(data, filename):
    """Emit sub-content bytes to be processed by WADUP.

    Use this to extract embedded content from files (e.g., files within
    a zip archive). WADUP will recursively analyze the emitted content.

    Args:
        data: Raw bytes to emit (bytes object)
        filename: Suggested filename for the content (string)

    Example:
        # Extract a file from a zip archive
        with zipfile.ZipFile("/data.bin") as zf:
            for name in zf.namelist():
                wadup.emit_bytes(zf.read(name), name)
    """
    global _subcontent_counter
    n = _subcontent_counter
    _subcontent_counter += 1

    os.makedirs("/subcontent", exist_ok=True)

    # Write data file
    with open(f"/subcontent/data_{n}.bin", "wb") as f:
        f.write(data)

    # Write metadata file (triggers processing on close)
    # Build JSON manually
    metadata_json = f'{{"filename":"{_escape_string(filename)}"}}'
    with open(f"/subcontent/metadata_{n}.json", "w") as f:
        f.write(metadata_json)
