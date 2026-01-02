"""Test pydantic BaseModel in WADUP.

This test verifies that full pydantic BaseModel works correctly.
Pydantic is pre-compiled to .pyc to avoid runtime bytecode compilation crashes.
"""

import wadup
from pydantic import BaseModel
import pydantic_core


class User(BaseModel):
    name: str
    age: int
    email: str


def main():
    """Test pydantic BaseModel functionality."""
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
        user = User(**user_data)
        validated_users.append(user)
        wadup.insert_row("users", [user.name, user.age, user.email])

    # Record status
    wadup.insert_row("info", ["status", "success"])
    wadup.insert_row("info", ["pydantic_core_version", pydantic_core.__version__])
    wadup.insert_row("info", ["users_validated", str(len(validated_users))])

    wadup.flush()
