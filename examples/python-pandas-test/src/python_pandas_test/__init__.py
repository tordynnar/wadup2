"""Pandas WASM test module for WADUP.

Tests Pandas DataFrame functionality in a WebAssembly environment.
"""

import wadup


def main():
    """Process input CSV data and demonstrate Pandas capabilities."""
    # Read input file
    with open('/data.bin', 'rb') as f:
        data = f.read()

    # Define output table for results
    wadup.define_table("pandas_result", [
        ("pandas_version", "String"),
        ("numpy_version", "String"),
        ("input_rows", "Int64"),
        ("input_cols", "Int64"),
        ("column_names", "String"),
        ("dtypes", "String"),
        ("sum_numeric", "String"),
        ("mean_numeric", "String"),
        ("describe_output", "String"),
        ("status", "String"),
    ])

    try:
        import pandas as pd
        import numpy as np
        import io

        # Try to parse input as CSV
        text = data.decode('utf-8', errors='replace').strip()

        if text and ',' in text:
            # Parse as CSV
            df = pd.read_csv(io.StringIO(text))
        else:
            # Create sample DataFrame for testing
            df = pd.DataFrame({
                'name': ['Alice', 'Bob', 'Charlie'],
                'age': [25, 30, 35],
                'score': [85.5, 92.0, 78.5],
            })

        # Get numeric columns for aggregations
        numeric_cols = df.select_dtypes(include=[np.number]).columns.tolist()

        # Calculate sums and means for numeric columns
        sums = {}
        means = {}
        for col in numeric_cols:
            sums[col] = float(df[col].sum())
            means[col] = float(df[col].mean())

        # Get head output (describe uses unavailable hash functions)
        describe_str = df.head().to_string()
        if len(describe_str) > 500:
            describe_str = describe_str[:500] + "..."

        wadup.insert_row("pandas_result", [
            pd.__version__,
            np.__version__,
            len(df),
            len(df.columns),
            str(list(df.columns)),
            str(dict(df.dtypes.astype(str))),
            str(sums),
            str(means),
            describe_str,
            "success",
        ])

    except Exception as e:
        import traceback
        tb = traceback.format_exc()
        wadup.insert_row("pandas_result", [
            "N/A",
            "N/A",
            0,
            0,
            "N/A",
            "N/A",
            "N/A",
            "N/A",
            "N/A",
            f"error: {type(e).__name__}: {e} | {tb[:300]}",
        ])

    wadup.flush()
