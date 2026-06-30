//! Python bindings for geo-report core tools.

use pyo3::prelude::*;

/// Generate a carbon accounting report in Markdown.
///
/// Returns the Markdown string.
#[pyfunction]
#[pyo3(signature = (title, aoi_name, year, source, total_tco2e, breakdown))]
pub fn report_carbon(
    title: &str,
    aoi_name: &str,
    year: u16,
    source: &str,
    total_tco2e: f64,
    breakdown: Vec<(String, f64, f64, f64)>, // (class, area_ha, factor, tco2e)
) -> PyResult<String> {
    let gen = geo_report::ReportGenerator::new()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    let bd: Vec<geo_report::report::LandcoverBreakdown> = breakdown
        .iter()
        .map(
            |(class, area_ha, factor, tco2e)| geo_report::report::LandcoverBreakdown {
                class: class.clone(),
                area_ha: *area_ha,
                factor: *factor,
                tco2e: *tco2e,
            },
        )
        .collect();
    let data = geo_report::report::CarbonReportData {
        title: title.into(),
        aoi_name: aoi_name.into(),
        year,
        generated_at: chrono::Utc::now().to_rfc3339(),
        source: source.into(),
        total_tco2e,
        breakdown: bd,
        audit_trails: vec![],
    };
    gen.carbon_report(&data)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// Render a Tera template with JSON data.
///
/// `data_json` is a JSON string of the template variables.
/// Returns the rendered string output.
#[pyfunction]
pub fn report_render(template_dir: &str, template_name: &str, data_json: &str) -> PyResult<String> {
    let mut engine = geo_report::ReportEngine::new()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    let path = std::path::PathBuf::from(template_dir);
    engine
        .register_templates("plugin", &path)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    let serde_data: serde_json::Value = serde_json::from_str(data_json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    engine
        .render(template_name, &serde_data)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}
