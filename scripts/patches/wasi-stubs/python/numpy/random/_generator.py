"""Stub for numpy.random module in WASI.

This provides minimal classes to satisfy imports but raises
NotImplementedError if actually used.
"""

class BitGenerator:
    """Stub bit generator base class."""

    def __init__(self, seed=None):
        self._seed = seed

    def __getattr__(self, name):
        raise NotImplementedError(
            f"numpy.random.BitGenerator.{name} is not available in WASI builds. "
            "The NumPy random module has not been compiled for WASI."
        )


class Generator:
    """Stub random number generator.

    NumPy's random module is not available in WASI builds.
    """

    def __init__(self, bit_generator=None):
        # Allow construction for type checking purposes
        self._bit_generator = bit_generator

    def __getattr__(self, name):
        raise NotImplementedError(
            f"numpy.random.Generator.{name} is not available in WASI builds. "
            "The NumPy random module has not been compiled for WASI."
        )


class RandomState:
    """Stub RandomState for compatibility."""

    def __init__(self, seed=None):
        self._seed = seed

    def __getattr__(self, name):
        raise NotImplementedError(
            f"numpy.random.RandomState.{name} is not available in WASI builds. "
            "The NumPy random module has not been compiled for WASI."
        )
