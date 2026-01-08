"""Stub asyncio module for WASI.

WASI doesn't support async I/O, but some libraries (like typing_extensions)
import asyncio.coroutines to check if functions are coroutines. This stub
provides minimal functionality to satisfy those imports.
"""

from . import coroutines

__all__ = ['coroutines']
