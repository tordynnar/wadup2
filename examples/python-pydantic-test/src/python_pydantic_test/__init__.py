"""Test pydantic_core validation in WADUP.

NOTE: Full pydantic (BaseModel) cannot be used in WASI due to a crash
when loading pydantic/_internal/_generate_schema.py. This test uses
pydantic_core directly, which works correctly.
"""

import sys
print("DEBUG: Module loading", file=sys.stderr, flush=True)

import wadup
print("DEBUG: wadup imported", file=sys.stderr, flush=True)

import pydantic_core
from pydantic_core import SchemaValidator, core_schema
print("DEBUG: pydantic_core imported", file=sys.stderr, flush=True)


def main():
    """Test pydantic_core validation functionality."""
    print("DEBUG: main() called", file=sys.stderr, flush=True)

    # Define a User schema using pydantic_core directly
    user_schema = core_schema.typed_dict_schema({
        'name': core_schema.typed_dict_field(core_schema.str_schema()),
        'age': core_schema.typed_dict_field(core_schema.int_schema(ge=0)),
        'email': core_schema.typed_dict_field(core_schema.str_schema()),
    })

    validator = SchemaValidator(user_schema)

    # Create test users
    users_data = [
        {"name": "Alice", "age": 30, "email": "alice@example.com"},
        {"name": "Bob", "age": 25, "email": "bob@example.com"},
        {"name": "Charlie", "age": 35, "email": "charlie@example.com"},
    ]

    # Define tables
    wadup.define_table("users", [
        ("name", "String"),
        ("age", "Int64"),
        ("email", "String"),
    ])
    wadup.define_table("info", [
        ("key", "String"),
        ("value", "String"),
    ])

    # Validate and insert users
    validated_users = []
    for user_data in users_data:
        validated = validator.validate_python(user_data)
        validated_users.append(validated)
        wadup.insert_row("users", [validated['name'], validated['age'], validated['email']])
        print(f"DEBUG: Validated user: {validated['name']}", file=sys.stderr, flush=True)

    # Record status
    wadup.insert_row("info", ["status", "success"])
    wadup.insert_row("info", ["pydantic_core_version", pydantic_core.__version__])
    wadup.insert_row("info", ["users_validated", str(len(validated_users))])

    wadup.flush()
    print("DEBUG: All done!", file=sys.stderr, flush=True)
