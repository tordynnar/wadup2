"""Stub asyncio.coroutines module for WASI.

Provides minimal coroutine detection stubs for libraries that need to check
if functions are coroutines (like typing_extensions @deprecated decorator).
"""


def iscoroutinefunction(func):
    """Check if func is a coroutine function.

    In WASI, we don't support async, so always return False.
    """
    return False


def iscoroutine(obj):
    """Check if obj is a coroutine.

    In WASI, we don't support async, so always return False.
    """
    return False
