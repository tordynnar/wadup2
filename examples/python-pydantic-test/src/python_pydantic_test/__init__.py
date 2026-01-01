"""Test pydantic_core validation in WADUP.

This example demonstrates pydantic_core schema validation
running on WASI, validating JSON data and outputting results.

Note: The full pydantic library (BaseModel, etc.) requires complex imports
that exceed WASI stack limits. Use pydantic_core directly for WASI modules.
"""

import wadup
import json
from pydantic_core import SchemaValidator, SchemaSerializer, ValidationError


def main():
    """Validate JSON data using pydantic_core schemas."""
    # Define output tables
    # Note: Bool is stored as Int64 (0 = False, 1 = True)
    wadup.define_table("validation_results", [
        ("input_type", "String"),
        ("input_value", "String"),
        ("valid", "Int64"),  # 1 for True, 0 for False
        ("output", "String"),
        ("error", "String"),
    ])

    wadup.define_table("info", [
        ("key", "String"),
        ("value", "String"),
    ])

    # Import and log version
    import pydantic_core
    wadup.insert_row("info", ["pydantic_core_version", pydantic_core.__version__])

    # Define some test schemas
    schemas = {
        "string": {"type": "str"},
        "int": {"type": "int"},
        "float": {"type": "float"},
        "bool": {"type": "bool"},
        "list_of_ints": {"type": "list", "items_schema": {"type": "int"}},
        "dict": {"type": "dict", "keys_schema": {"type": "str"}, "values_schema": {"type": "int"}},
    }

    # Test data
    test_cases = [
        ("string", "hello world"),
        ("string", 123),  # Should coerce to string
        ("int", 42),
        ("int", "not an int"),  # Should fail
        ("float", 3.14),
        ("float", "3.14"),  # Should coerce
        ("bool", True),
        ("bool", "yes"),  # Should fail (strict)
        ("list_of_ints", [1, 2, 3]),
        ("list_of_ints", [1, "two", 3]),  # Should fail on "two"
        ("dict", {"a": 1, "b": 2}),
    ]

    # Read optional input data from /data.bin
    try:
        with open('/data.bin', 'rb') as f:
            content = f.read()
            if content:
                # Try to parse as JSON and add to test cases
                try:
                    data = json.loads(content)
                    if isinstance(data, list):
                        for item in data:
                            if isinstance(item, dict) and "schema" in item and "value" in item:
                                test_cases.append((item["schema"], item["value"]))
                except json.JSONDecodeError:
                    pass
    except FileNotFoundError:
        pass

    # Run validation tests
    for schema_name, value in test_cases:
        if schema_name not in schemas:
            wadup.insert_row("validation_results", [
                schema_name,
                str(value)[:100],
                False,
                "",
                f"Unknown schema: {schema_name}",
            ])
            continue

        schema = schemas[schema_name]
        validator = SchemaValidator(schema)

        try:
            result = validator.validate_python(value)
            wadup.insert_row("validation_results", [
                schema_name,
                str(value)[:100],
                True,
                str(result)[:100],
                "",
            ])
        except ValidationError as e:
            wadup.insert_row("validation_results", [
                schema_name,
                str(value)[:100],
                False,
                "",
                str(e)[:200],
            ])

    # Test serialization
    wadup.define_table("serialization_results", [
        ("schema", "String"),
        ("input", "String"),
        ("json_output", "String"),
    ])

    serialization_tests = [
        ("string", "hello"),
        ("int", 42),
        ("list_of_ints", [1, 2, 3]),
        ("dict", {"x": 10, "y": 20}),
    ]

    for schema_name, value in serialization_tests:
        if schema_name in schemas:
            serializer = SchemaSerializer(schemas[schema_name])
            try:
                json_bytes = serializer.to_json(value)
                wadup.insert_row("serialization_results", [
                    schema_name,
                    str(value)[:100],
                    json_bytes.decode('utf-8'),
                ])
            except Exception as e:
                wadup.insert_row("serialization_results", [
                    schema_name,
                    str(value)[:100],
                    f"Error: {e}",
                ])

    wadup.flush()
