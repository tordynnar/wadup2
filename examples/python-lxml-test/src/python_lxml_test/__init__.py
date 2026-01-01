"""Test lxml C extension in WADUP.

This example parses XML content using lxml.etree and outputs
the parsed elements to a wadup table.
"""

import wadup
from lxml import etree


def main():
    """Parse XML from input and output elements to a table."""
    # Define output table for XML elements
    wadup.define_table("xml_elements", [
        ("depth", "Int64"),
        ("tag", "String"),
        ("text", "String"),
        ("attribs", "String"),
    ])

    # Read input data from /data.bin
    with open('/data.bin', 'rb') as f:
        content = f.read()

    try:
        # Parse XML
        root = etree.fromstring(content)

        # Walk the tree and output elements
        def walk(elem, depth=0):
            # Get text content (strip whitespace)
            text = (elem.text or "").strip()

            # Format attributes as key=value pairs
            attribs = ", ".join(f"{k}={v}" for k, v in elem.attrib.items())

            # Insert row
            wadup.insert_row("xml_elements", [depth, elem.tag, text, attribs])

            # Recurse into children
            for child in elem:
                walk(child, depth + 1)

        walk(root)

    except etree.XMLSyntaxError as e:
        # Handle parse errors
        wadup.define_table("parse_errors", [("error", "String")])
        wadup.insert_row("parse_errors", [str(e)])

    wadup.flush()
