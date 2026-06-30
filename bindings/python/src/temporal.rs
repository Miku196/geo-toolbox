//! Python bindings for geo-temporal core tools.

use pyo3::prelude::*;

/// Mann-Kendall trend test + linear regression slope.
///
/// Returns (sen_slope, ols_intercept, significant, mann_kendall_tau, p_value)
#[pyfunction]
pub fn temporal_trend(values: Vec<f64>) -> PyResult<(f64, f64, bool, f64, f64)> {
    if values.len() < 3 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "need at least 3 values",
        ));
    }
    let trend = geo_temporal::linear_trend(&values);
    let (tau, p_value) = geo_temporal::mann_kendall(&values);
    Ok((
        trend.sen_slope,
        trend.ols_intercept,
        trend.significant,
        tau,
        p_value,
    ))
}
