"""Test pydantic BaseModel in WADUP.

This example demonstrates using Pydantic's BaseModel for data validation
in a WADUP WASM module.
"""

import wadup


def main():
    """Run pydantic test."""
    # Import pydantic modules in order to avoid deep import chains
    import pydantic._internal._config
    import pydantic._internal._fields
    import pydantic._internal._generate_schema
    import pydantic.main

    from pydantic import BaseModel, Field

    # Define a schema for the output
    wadup.define_table("users", [
        ("name", "String"),
        ("age", "Int64"),
        ("email", "String"),
    ])

    wadup.define_table("info", [
        ("key", "String"),
        ("value", "String"),
    ])

    # Create a Pydantic model
    class User(BaseModel):
        name: str
        age: int = Field(ge=0, le=150)
        email: str | None = None

    # Create some test users
    users = [
        User(name="Alice", age=30, email="alice@example.com"),
        User(name="Bob", age=25),
        User(name="Claude", age=2),
    ]

    # Insert users into the output table
    for user in users:
        wadup.insert_row("users", [user.name, user.age, user.email or ""])

    # Report versions
    import pydantic
    import pydantic_core
    wadup.insert_row("info", ["pydantic_version", pydantic.__version__])
    wadup.insert_row("info", ["pydantic_core_version", pydantic_core.__version__])
    wadup.insert_row("info", ["status", "success"])

    wadup.flush()
