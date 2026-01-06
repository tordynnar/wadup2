"""pydantic_core - Core validation library for Pydantic V2."""
from __future__ import annotations

import sys as _sys
from typing import Any as _Any

from typing_extensions import Sentinel

from _pydantic_core import (
    ArgsKwargs,
    MultiHostUrl,
    PydanticCustomError,
    PydanticKnownError,
    PydanticOmit,
    PydanticSerializationError,
    PydanticSerializationUnexpectedValue,
    PydanticUndefined,
    PydanticUndefinedType,
    PydanticUseDefault,
    SchemaError,
    SchemaSerializer,
    SchemaValidator,
    Some,
    TzInfo,
    Url,
    ValidationError,
    __version__,
    from_json,
    to_json,
    to_jsonable_python,
)

# Import core_schema for full pydantic compatibility
from . import core_schema
from .core_schema import CoreConfig, CoreSchema, CoreSchemaType, ErrorType

if _sys.version_info < (3, 11):
    from typing_extensions import NotRequired as _NotRequired
else:
    from typing import NotRequired as _NotRequired

if _sys.version_info < (3, 12):
    from typing_extensions import TypedDict as _TypedDict
else:
    from typing import TypedDict as _TypedDict

# Sentinel values
UNSET: Sentinel = Sentinel('UNSET')
MISSING: Sentinel = Sentinel('MISSING')

__all__ = [
    '__version__',
    'MISSING',
    'UNSET',
    'CoreConfig',
    'CoreSchema',
    'CoreSchemaType',
    'SchemaValidator',
    'SchemaSerializer',
    'Some',
    'Url',
    'MultiHostUrl',
    'ArgsKwargs',
    'PydanticUndefined',
    'PydanticUndefinedType',
    'SchemaError',
    'ErrorDetails',
    'InitErrorDetails',
    'ValidationError',
    'PydanticCustomError',
    'PydanticKnownError',
    'PydanticOmit',
    'PydanticUseDefault',
    'PydanticSerializationError',
    'PydanticSerializationUnexpectedValue',
    'TzInfo',
    'to_json',
    'from_json',
    'to_jsonable_python',
    'core_schema',
]


class ErrorDetails(_TypedDict):
    type: str
    loc: tuple[int | str, ...]
    msg: str
    input: _Any
    ctx: _NotRequired[dict[str, _Any]]
    url: _NotRequired[str]


class InitErrorDetails(_TypedDict):
    type: str | PydanticCustomError
    loc: _NotRequired[tuple[int | str, ...]]
    input: _Any
    ctx: _NotRequired[dict[str, _Any]]
