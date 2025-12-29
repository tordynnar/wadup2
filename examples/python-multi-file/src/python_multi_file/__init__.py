"""Python multi-file WADUP module.

This example demonstrates:
1. Multiple Python source files in a project
2. Using pure-Python dependencies (chardet, humanize, python-slugify)
3. Importing between module files

The module analyzes files and detects their encoding using the
chardet library, formats sizes with humanize, and creates slugs
with python-slugify.
"""
import wadup
from .analyzer import analyze_content


def main():
    """Entry point called by WADUP for each file processed."""
    # Read input file
    with open('/data.bin', 'rb') as f:
        data = f.read()

    # Analyze the file using our analyzer module
    analysis = analyze_content(data)

    # Define table for file analysis results
    wadup.define_table("file_analysis", [
        ("total_bytes", "Int64"),
        ("line_count", "Int64"),
        ("word_count", "Int64"),
        ("char_count", "Int64"),
        ("human_size", "String"),
        ("encoding", "String"),
        ("encoding_confidence", "Float64"),
        ("encoding_language", "String"),
        ("encoding_slug", "String"),
    ])

    # Insert the analysis results
    d = analysis.to_dict()
    wadup.insert_row("file_analysis", [
        d['total_bytes'],
        d['line_count'],
        d['word_count'],
        d['char_count'],
        d['human_size'],
        d['encoding'],
        d['encoding_confidence'],
        d['encoding_language'],
        d['encoding_slug'],
    ])

    # Flush metadata
    wadup.flush()
