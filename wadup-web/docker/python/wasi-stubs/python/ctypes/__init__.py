"""Stub ctypes module for WASI.

ctypes is not available in WASI because it requires dynamic linking (dlopen).
This stub provides minimal functionality to allow packages to import without
crashing, though actual ctypes functionality will not work.
"""

class WASINotSupportedError(Exception):
    """Raised when attempting to use ctypes functionality in WASI."""
    pass


def _not_supported(*args, **kwargs):
    raise WASINotSupportedError("ctypes is not supported in WASI builds")


# Type definitions (stubs)
class c_void_p:
    pass

class c_char_p:
    pass

class c_wchar_p:
    pass

class c_int:
    pass

class c_uint:
    pass

class c_long:
    pass

class c_ulong:
    pass

class c_longlong:
    pass

class c_ulonglong:
    pass

class c_float:
    pass

class c_double:
    pass

class c_size_t:
    pass

class c_ssize_t:
    pass

class c_bool:
    pass

class c_char:
    pass

class c_byte:
    pass

class c_ubyte:
    pass

class c_short:
    pass

class c_ushort:
    pass

class c_int8:
    pass

class c_int16:
    pass

class c_int32:
    pass

class c_int64:
    pass

class c_uint8:
    pass

class c_uint16:
    pass

class c_uint32:
    pass

class c_uint64:
    pass


class Structure:
    """Stub Structure class."""
    _fields_ = []


class Union:
    """Stub Union class."""
    _fields_ = []


class Array:
    """Stub Array class."""
    pass


class POINTER:
    """Stub POINTER class."""
    def __init__(self, type_):
        pass


class CFUNCTYPE:
    """Stub CFUNCTYPE class."""
    def __init__(self, restype, *argtypes):
        pass


class CDLL:
    """Stub CDLL class."""
    def __init__(self, name, mode=0, handle=None, use_errno=False, use_last_error=False):
        raise WASINotSupportedError(f"Cannot load library '{name}': ctypes not supported in WASI")


class LibraryLoader:
    """Stub LibraryLoader class."""
    def __init__(self, dlltype):
        self._dlltype = dlltype

    def LoadLibrary(self, name):
        raise WASINotSupportedError(f"Cannot load library '{name}': ctypes not supported in WASI")

    def __getattr__(self, name):
        raise WASINotSupportedError(f"Cannot load library '{name}': ctypes not supported in WASI")


cdll = LibraryLoader(CDLL)
pydll = LibraryLoader(CDLL)


# Functions
def sizeof(type_):
    """Return size of a type."""
    return 0


def addressof(obj):
    """Return address of object."""
    raise WASINotSupportedError("addressof not supported in WASI")


def byref(obj, offset=0):
    """Return a pointer to object."""
    raise WASINotSupportedError("byref not supported in WASI")


def cast(obj, type_):
    """Cast object to type."""
    raise WASINotSupportedError("cast not supported in WASI")


def create_string_buffer(init_or_size, size=None):
    """Create mutable string buffer."""
    raise WASINotSupportedError("create_string_buffer not supported in WASI")


def create_unicode_buffer(init_or_size, size=None):
    """Create mutable unicode buffer."""
    raise WASINotSupportedError("create_unicode_buffer not supported in WASI")


def pointer(obj):
    """Create a pointer to object."""
    raise WASINotSupportedError("pointer not supported in WASI")


def resize(obj, size):
    """Resize a ctypes instance."""
    raise WASINotSupportedError("resize not supported in WASI")


def get_errno():
    """Return current errno value."""
    return 0


def set_errno(value):
    """Set errno value."""
    return 0


# Platform-specific
import sys
if sys.platform == 'win32':
    windll = LibraryLoader(CDLL)
    oledll = LibraryLoader(CDLL)
    WinDLL = CDLL
    OleDLL = CDLL
