"""Minimal reproduction of WASI Python dlmalloc crash.

This file triggers a crash in Python WASI dlmalloc when bytecode is compiled.
The crash occurs when compiling a method with >= 28 if/elif branches.

Bug: Python WASI memory allocator (dlmalloc) crashes during bytecode compilation
     of methods with many if/elif branches.

Crash location: WASM function 15532 (dlmalloc)

Key finding:
- 27 if/elif branches: WORKS
- 28 if/elif branches: CRASHES
"""

from __future__ import annotations
from typing import Any


class CrashTrigger:
    """Class with a method that has 28 if/elif branches - triggers dlmalloc crash."""

    def method_with_many_branches(self, obj: Any) -> str:
        """Method with 28 if/elif branches - crashes during bytecode compilation."""
        if obj == 0:
            return "case_0"
        elif obj == 1:
            return "case_1"
        elif obj == 2:
            return "case_2"
        elif obj == 3:
            return "case_3"
        elif obj == 4:
            return "case_4"
        elif obj == 5:
            return "case_5"
        elif obj == 6:
            return "case_6"
        elif obj == 7:
            return "case_7"
        elif obj == 8:
            return "case_8"
        elif obj == 9:
            return "case_9"
        elif obj == 10:
            return "case_10"
        elif obj == 11:
            return "case_11"
        elif obj == 12:
            return "case_12"
        elif obj == 13:
            return "case_13"
        elif obj == 14:
            return "case_14"
        elif obj == 15:
            return "case_15"
        elif obj == 16:
            return "case_16"
        elif obj == 17:
            return "case_17"
        elif obj == 18:
            return "case_18"
        elif obj == 19:
            return "case_19"
        elif obj == 20:
            return "case_20"
        elif obj == 21:
            return "case_21"
        elif obj == 22:
            return "case_22"
        elif obj == 23:
            return "case_23"
        elif obj == 24:
            return "case_24"
        elif obj == 25:
            return "case_25"
        elif obj == 26:
            return "case_26"
        elif obj == 27:
            return "case_27"
        return "default"
