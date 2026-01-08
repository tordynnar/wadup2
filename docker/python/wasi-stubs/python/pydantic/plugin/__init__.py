"""Pydantic plugin stub for WASI.

Plugin interface for Pydantic plugins, and related types.
"""

from __future__ import annotations

from typing import Any, Callable, Literal, NamedTuple

from pydantic_core import CoreConfig, CoreSchema, ValidationError
from typing_extensions import Protocol, TypeAlias

__all__ = (
    'PydanticPluginProtocol',
    'BaseValidateHandlerProtocol',
    'ValidatePythonHandlerProtocol',
    'ValidateJsonHandlerProtocol',
    'ValidateStringsHandlerProtocol',
    'NewSchemaReturns',
    'SchemaTypePath',
    'SchemaKind',
)

NewSchemaReturns: TypeAlias = 'tuple[ValidatePythonHandlerProtocol | None, ValidateJsonHandlerProtocol | None, ValidateStringsHandlerProtocol | None]'


class SchemaTypePath(NamedTuple):
    """Path defining where `schema_type` was defined, or where `TypeAdapter` was called."""

    module: str
    name: str


SchemaKind: TypeAlias = Literal['BaseModel', 'TypeAdapter', 'dataclass', 'create_model', 'validate_call']


class PydanticPluginProtocol(Protocol):
    """Protocol defining the interface for Pydantic plugins."""

    def new_schema_validator(
        self,
        schema: CoreSchema,
        schema_type: Any,
        schema_type_path: SchemaTypePath,
        schema_kind: SchemaKind,
        config: CoreConfig | None,
        plugin_settings: dict[str, object],
    ) -> tuple[
        ValidatePythonHandlerProtocol | None, ValidateJsonHandlerProtocol | None, ValidateStringsHandlerProtocol | None
    ]:
        raise NotImplementedError('Pydantic plugins should implement `new_schema_validator`.')


class BaseValidateHandlerProtocol(Protocol):
    """Base class for plugin callbacks protocols."""

    on_enter: Callable[..., None]

    def on_success(self, result: Any) -> None:
        return

    def on_error(self, error: ValidationError) -> None:
        return

    def on_exception(self, exception: Exception) -> None:
        return


class ValidatePythonHandlerProtocol(BaseValidateHandlerProtocol, Protocol):
    """Event handler for `SchemaValidator.validate_python`."""

    def on_enter(
        self,
        input: Any,
        *,
        strict: bool | None = None,
        from_attributes: bool | None = None,
        context: Any | None = None,
        self_instance: Any | None = None,
        by_alias: bool | None = None,
        by_name: bool | None = None,
        **kwargs: Any,
    ) -> None:
        pass


class ValidateJsonHandlerProtocol(BaseValidateHandlerProtocol, Protocol):
    """Event handler for `SchemaValidator.validate_json`."""

    def on_enter(
        self,
        input: str | bytes | bytearray,
        *,
        strict: bool | None = None,
        context: Any | None = None,
        self_instance: Any | None = None,
        by_alias: bool | None = None,
        by_name: bool | None = None,
        **kwargs: Any,
    ) -> None:
        pass


StringInput: TypeAlias = 'dict[str, StringInput]'


class ValidateStringsHandlerProtocol(BaseValidateHandlerProtocol, Protocol):
    """Event handler for `SchemaValidator.validate_strings`."""

    def on_enter(
        self,
        input: StringInput,
        *,
        strict: bool | None = None,
        context: Any | None = None,
        by_alias: bool | None = None,
        by_name: bool | None = None,
        **kwargs: Any,
    ) -> None:
        pass


# Re-export from _loader
from ._loader import get_plugins
from ._schema_validator import create_schema_validator
