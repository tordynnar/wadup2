//! Minimal PyO3 extension demonstrating OnceLock/PyOnceLock crash on WASI.
//!
//! This extension tests multiple PyOnceLock patterns used in pydantic_core:
//! 1. std::sync::OnceLock - standard Rust sync primitive
//! 2. pyo3::sync::PyOnceLock - PyO3's Python-aware version
//! 3. PyOnceLock with Py::new() - creates a pyclass instance (like PydanticUndefinedType)
//! 4. Multiple chained PyOnceLock calls during module init

use std::sync::OnceLock;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyType, PyAnyMethods};
use pyo3::exceptions::PyNotImplementedError;

/// Static std::sync::OnceLock - this is what pydantic_core uses for version strings
static STD_ONCELOCK: OnceLock<String> = OnceLock::new();

/// Static PyOnceLock that caches a Python type.
static PY_ONCELOCK: PyOnceLock<Py<PyType>> = PyOnceLock::new();

/// PyOnceLock for our custom undefined type (like pydantic_core's PydanticUndefinedType)
static UNDEFINED_CELL: PyOnceLock<Py<UndefinedType>> = PyOnceLock::new();

/// Another PyOnceLock for a second type (like pydantic_core's ArgsKwargs)
static MARKER_CELL: PyOnceLock<Py<MarkerType>> = PyOnceLock::new();

/// Custom pyclass that mimics PydanticUndefinedType exactly
#[pyclass(module = "pyoncelock_demo", frozen)]
#[derive(Debug)]
pub struct UndefinedType {}

#[pymethods]
impl UndefinedType {
    #[new]
    pub fn py_new(_py: Python) -> PyResult<Self> {
        // Mimics pydantic_core: "Creating instances of UndefinedType is not supported"
        Err(PyNotImplementedError::new_err(
            "Creating instances of \"UndefinedType\" is not supported",
        ))
    }

    #[staticmethod]
    #[pyo3(name = "new")]
    pub fn get(py: Python<'_>) -> &Py<Self> {
        eprintln!("[Rust] UndefinedType::get() called");
        UNDEFINED_CELL.get_or_init(py, || {
            eprintln!("[Rust] UNDEFINED_CELL initializing with Py::new()...");
            Py::new(py, UndefinedType {}).unwrap()
        })
    }

    fn __repr__(&self) -> &'static str {
        "Undefined"
    }

    fn __copy__(&self, py: Python) -> Py<Self> {
        UNDEFINED_CELL.get(py).unwrap().clone_ref(py)
    }
}

/// Another custom pyclass that gets initialized during module init
#[pyclass(module = "pyoncelock_demo", frozen)]
#[derive(Debug)]
pub struct MarkerType {}

#[pymethods]
impl MarkerType {
    #[staticmethod]
    pub fn get(py: Python<'_>) -> &Py<Self> {
        eprintln!("[Rust] MarkerType::get() called");
        MARKER_CELL.get_or_init(py, || {
            eprintln!("[Rust] MARKER_CELL initializing with Py::new()...");
            Py::new(py, MarkerType {}).unwrap()
        })
    }

    fn __repr__(&self) -> &'static str {
        "Marker"
    }
}

/// PyOnceLock that imports a Python type from a module (like pydantic_core's FRACTION_TYPE)
static FRACTION_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

/// Get the Fraction type - this imports fractions module during initialization
/// This mimics pydantic_core's pattern: `py.import("fractions")?.getattr("Fraction")?`
fn get_fraction_type(py: Python<'_>) -> PyResult<&Py<PyType>> {
    FRACTION_TYPE.get_or_try_init(py, || {
        eprintln!("[Rust] FRACTION_TYPE initializing - importing fractions module...");
        let fractions = py.import("fractions")?;
        eprintln!("[Rust] fractions module imported");
        let fraction_type: Bound<'_, PyType> = fractions.getattr("Fraction")?.downcast_into()?;
        eprintln!("[Rust] Got Fraction type");
        Ok(fraction_type.unbind())
    })
}

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

/// Python module definition - mimics pydantic_core's module_init pattern
#[pymodule]
fn pyoncelock_demo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    eprintln!("[Rust] pyoncelock_demo module initializing");

    // Add classes first (like pydantic_core does)
    m.add_class::<UndefinedType>()?;
    m.add_class::<MarkerType>()?;

    // This mimics exactly what pydantic_core does in module_init:
    // m.add("PydanticUndefined", PydanticUndefinedType::get(m.py()))?;
    eprintln!("[Rust] Calling UndefinedType::get() during module init...");
    m.add("Undefined", UndefinedType::get(m.py()))?;
    eprintln!("[Rust] UndefinedType initialized successfully");

    // Add another type that also uses PyOnceLock (like pydantic_core has multiple)
    eprintln!("[Rust] Calling MarkerType::get() during module init...");
    m.add("Marker", MarkerType::get(m.py()))?;
    eprintln!("[Rust] MarkerType initialized successfully");

    // Try importing a Python module during PyOnceLock init (like pydantic_core does)
    // This is key: pydantic_core imports fractions, decimal, uuid etc. during init
    eprintln!("[Rust] Calling get_fraction_type() during module init...");
    match get_fraction_type(m.py()) {
        Ok(frac_type) => {
            m.add("FractionType", frac_type)?;
            eprintln!("[Rust] FractionType added successfully");
        }
        Err(e) => {
            eprintln!("[Rust] Failed to get FractionType: {:?}", e);
            // Don't fail the module init, just log
        }
    }

    m.add_function(wrap_pyfunction!(simple_add, m)?)?;
    m.add_function(wrap_pyfunction!(test_std_oncelock, m)?)?;
    m.add_function(wrap_pyfunction!(test_py_oncelock, m)?)?;
    eprintln!("[Rust] pyoncelock_demo module initialized successfully");
    Ok(())
}
