"""Stub mmap module for WASI.

mmap is not fully supported in WASI because it requires memory mapping syscalls.
This stub provides minimal functionality to allow packages to import without
crashing, though actual mmap functionality will not work.
"""

# Constants
PROT_READ = 1
PROT_WRITE = 2
PROT_EXEC = 4

MAP_SHARED = 1
MAP_PRIVATE = 2
MAP_ANONYMOUS = 32

ACCESS_READ = 1
ACCESS_WRITE = 2
ACCESS_COPY = 3
ACCESS_DEFAULT = 0

PAGESIZE = 4096
ALLOCATIONGRANULARITY = 4096


class WASINotSupportedError(Exception):
    """Raised when attempting to use mmap functionality in WASI."""
    pass


class mmap:
    """Stub mmap class for WASI.

    Creates a memory-mapped file object. In WASI, this raises an error
    since mmap syscalls are not available.
    """

    def __init__(self, fileno, length, tagname=None, access=ACCESS_WRITE, offset=0):
        raise WASINotSupportedError(
            "mmap is not supported in WASI builds - memory mapping requires "
            "syscalls that are not available in the WASI sandbox"
        )

    def close(self):
        pass

    def flush(self, offset=0, size=0):
        pass

    def move(self, dest, src, count):
        pass

    def read(self, n=-1):
        return b''

    def read_byte(self):
        return 0

    def readline(self):
        return b''

    def resize(self, newsize):
        pass

    def seek(self, pos, whence=0):
        pass

    def size(self):
        return 0

    def tell(self):
        return 0

    def write(self, bytes):
        pass

    def write_byte(self, byte):
        pass

    def find(self, sub, start=0, end=None):
        return -1

    def rfind(self, sub, start=0, end=None):
        return -1

    def __len__(self):
        return 0

    def __getitem__(self, index):
        raise WASINotSupportedError("mmap is not supported in WASI builds")

    def __setitem__(self, index, value):
        raise WASINotSupportedError("mmap is not supported in WASI builds")

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()
