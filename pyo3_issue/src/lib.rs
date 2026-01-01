//! Minimal PyO3 extension demonstrating OnceLock/PyOnceLock crash on WASI.
//!
//! This extension tests both:
//! 1. std::sync::OnceLock - standard Rust sync primitive
//! 2. pyo3::sync::PyOnceLock - PyO3's Python-aware version
//!
//! Both use once_cell::sync::OnceCell internally, which requires threading
//! primitives that don't exist on WASI.

use std::sync::OnceLock;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyType;

/// Static std::sync::OnceLock - this is what pydantic_core uses for version strings
static STD_ONCELOCK: OnceLock<String> = OnceLock::new();

/// Static PyOnceLock that caches a Python type.
static PY_ONCELOCK: PyOnceLock<Py<PyType>> = PyOnceLock::new();

/// Test std::sync::OnceLock (no Python involved)
#[pyfunction]
fn test_std_oncelock() -> String {
    eprintln!("[Rust] test_std_oncelock called");

    let value = STD_ONCELOCK.get_or_init(|| {
        eprintln!("[Rust] std::sync::OnceLock initializing...");
        "initialized".to_string()
    });

    eprintln!("[Rust] std::sync::OnceLock returned: {}", value);
    value.clone()
}

/// Test pyo3::sync::PyOnceLock (Python-aware)
#[pyfunction]
fn test_py_oncelock(py: Python<'_>) -> PyResult<Py<PyType>> {
    eprintln!("[Rust] test_py_oncelock called");

    let type_obj = PY_ONCELOCK.get_or_init(py, || {
        eprintln!("[Rust] pyo3::sync::PyOnceLock initializing...");
        let int_type = py.get_type::<pyo3::types::PyInt>();
        eprintln!("[Rust] Got int type");
        int_type.unbind()
    });

    eprintln!("[Rust] pyo3::sync::PyOnceLock returned value");
    Ok(type_obj.clone_ref(py))
}

/// Simple function that doesn't use any OnceLock - should always work.
#[pyfunction]
fn simple_add(a: i32, b: i32) -> i32 {
    eprintln!("[Rust] simple_add({}, {}) called", a, b);
    a + b
}

/// Python module definition.
#[pymodule]
fn pyoncelock_demo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    eprintln!("[Rust] pyoncelock_demo module initializing");
    m.add_function(wrap_pyfunction!(simple_add, m)?)?;
    m.add_function(wrap_pyfunction!(test_std_oncelock, m)?)?;
    m.add_function(wrap_pyfunction!(test_py_oncelock, m)?)?;
    eprintln!("[Rust] pyoncelock_demo module initialized successfully");
    Ok(())
}
