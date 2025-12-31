"""Subprocess stub for WASI.

WASI does not support process spawning, so this module provides
minimal stubs to allow imports to succeed while raising errors
if actually used.
"""

# Exception classes
class SubprocessError(Exception):
    """Exception raised for subprocess-related errors."""
    pass

class CalledProcessError(SubprocessError):
    """Exception raised when a process returns a non-zero exit code."""
    def __init__(self, returncode, cmd, output=None, stderr=None):
        self.returncode = returncode
        self.cmd = cmd
        self.output = output
        self.stderr = stderr
        super().__init__(f"Command '{cmd}' returned non-zero exit status {returncode}")

class TimeoutExpired(SubprocessError):
    """Exception raised when a process times out."""
    def __init__(self, cmd, timeout, output=None, stderr=None):
        self.cmd = cmd
        self.timeout = timeout
        self.output = output
        self.stderr = stderr
        super().__init__(f"Command '{cmd}' timed out after {timeout} seconds")

# Constants
DEVNULL = -2
PIPE = -1
STDOUT = -2

# Stub functions
def run(*args, **kwargs):
    """Run command - not supported in WASI."""
    raise NotImplementedError("subprocess.run is not available in WASI builds")

def call(*args, **kwargs):
    """Call command - not supported in WASI."""
    raise NotImplementedError("subprocess.call is not available in WASI builds")

def check_call(*args, **kwargs):
    """Check call - not supported in WASI."""
    raise NotImplementedError("subprocess.check_call is not available in WASI builds")

def check_output(*args, **kwargs):
    """Check output - not supported in WASI."""
    raise NotImplementedError("subprocess.check_output is not available in WASI builds")

def getoutput(cmd):
    """Get output - not supported in WASI."""
    raise NotImplementedError("subprocess.getoutput is not available in WASI builds")

def getstatusoutput(cmd):
    """Get status and output - not supported in WASI."""
    raise NotImplementedError("subprocess.getstatusoutput is not available in WASI builds")

class Popen:
    """Stub Popen class."""

    def __init__(self, *args, **kwargs):
        raise NotImplementedError("subprocess.Popen is not available in WASI builds")

# Export all public names
__all__ = [
    'SubprocessError', 'CalledProcessError', 'TimeoutExpired',
    'DEVNULL', 'PIPE', 'STDOUT',
    'run', 'call', 'check_call', 'check_output',
    'getoutput', 'getstatusoutput',
    'Popen',
]
