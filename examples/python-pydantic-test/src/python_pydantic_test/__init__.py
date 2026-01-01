"""Test pydantic_core C extension in WADUP.

This example uses pydantic_core directly for validation. The high-level
pydantic library (BaseModel, Field) has deep import chains that are too
slow for WASM, so we use the lower-level pydantic_core API instead.
"""

import wadup


def main():
    """Run pydantic_core validation test."""
    from pydantic_core import SchemaValidator, core_schema
    import pydantic_core

    # Define output tables
    wadup.define_table("users", [
        ("name", "String"),
        ("age", "Int64"),
        ("email", "String"),
    ])

    wadup.define_table("info", [
        ("key", "String"),
        ("value", "String"),
    ])

    # Define a user schema using pydantic_core directly
    user_schema = core_schema.typed_dict_schema({
        'name': core_schema.typed_dict_field(core_schema.str_schema()),
        'age': core_schema.typed_dict_field(
            core_schema.int_schema(ge=0, le=150)
        ),
        'email': core_schema.typed_dict_field(
            core_schema.nullable_schema(core_schema.str_schema()),
            required=False
        ),
    })

    validator = SchemaValidator(user_schema)

    # Validate some test users
    test_users = [
        {'name': 'Alice', 'age': 30, 'email': 'alice@example.com'},
        {'name': 'Bob', 'age': 25},
        {'name': 'Claude', 'age': 2},
    ]

    for user_data in test_users:
        validated = validator.validate_python(user_data)
        wadup.insert_row("users", [
            validated['name'],
            validated['age'],
            validated.get('email', '') or ''
        ])

    # Report version and status
    wadup.insert_row("info", ["pydantic_core_version", pydantic_core.__version__])
    wadup.insert_row("info", ["status", "success"])

    wadup.flush()
