"""WASI sysconfigdata stub.

This provides minimal sysconfig data for WASI builds.
"""

build_time_vars = {
    'HOST_GNU_TYPE': 'wasm32-wasi',
    'SOABI': 'cpython-313-wasm32-wasi',
    'EXT_SUFFIX': '.cpython-313-wasm32-wasi.so',
    'prefix': '/usr/local',
    'exec_prefix': '/usr/local',
    'LIBDIR': '/usr/local/lib',
    'INCLUDEPY': '/usr/local/include/python3.13',
    'py_version_nodot': '313',
    'VERSION': '3.13',
}
