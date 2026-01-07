"""Stub for numpy.random package in WASI."""
from ._generator import Generator, BitGenerator, RandomState

__all__ = ['Generator', 'BitGenerator', 'RandomState']
