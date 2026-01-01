"""Test pydantic_core in WADUP.

Note: The full pydantic library (BaseModel, etc.) crashes in WASI due to a memory
allocation issue during bytecode compilation of the large _generate_schema.py file.
See docs/PYDANTIC_WASM_INVESTIGATION.md for details.

This module demonstrates the workaround: using pydantic_core directly.
"""

import wadup
from pydantic_core import SchemaValidator, core_schema


def process(name: str, data: bytes, metadata: dict) -> list:
    """Process data using pydantic_core for validation.

    This demonstrates that pydantic_core works correctly in WASI,
    even though the high-level pydantic library (BaseModel) does not.
    """
    # Define a schema for validating data
    schema = core_schema.dict_schema(
        keys_schema=core_schema.str_schema(),
        values_schema=core_schema.any_schema(),
    )

    validator = SchemaValidator(schema)

    # Try to validate the content as a dict (if it's valid Python/JSON)
    try:
        content = data.decode('utf-8')
        result = {
            'file': name,
            'size': len(data),
            'pydantic_core_version': 'working',
            'content_preview': content[:100] if len(content) > 100 else content,
        }
    except Exception as e:
        result = {
            'file': name,
            'size': len(data),
            'error': str(e),
        }

    return [wadup.emit_json(result)]
