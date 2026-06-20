//! IO utilities — CSV parsing, GeoJSON generation, Excel export, carbon report.

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Parse CSV text into a list of dicts.
pub fn parse_csv_to_json_impl(
    csv_text: &str,
) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_text.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?
        .iter()
        .map(|h| h.to_string())
        .collect();
    let mut rows = Vec::new();
    for result in reader.records() {
        let record =
            result.map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let mut row = std::collections::HashMap::new();
        for (i, val) in record.iter().enumerate() {
            if i < headers.len() {
                row.insert(headers[i].clone(), val.to_string());
            }
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Generate a GeoJSON FeatureCollection string from a list of Feature dicts.
pub fn generate_geojson_impl(features: &Bound<'_, PyList>) -> PyResult<String> {
    let py = features.py();
    let json_mod = py.import("json")?;
    let json_str: String = json_mod.call_method1("dumps", (features,))?.extract()?;
    let features_vec: Vec<serde_json::Value> = serde_json::from_str(&json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features_vec,
    });
    serde_json::to_string(&fc)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Generate an Excel file (.xlsx) and return the bytes.
pub fn generate_excel_impl(
    columns: Vec<String>,
    rows: Vec<Vec<PyObject>>,
    sheet_name: Option<String>,
) -> PyResult<Vec<u8>> {
    use rust_xlsxwriter::*;
    let mut workbook = Workbook::new();
    let name = sheet_name.as_deref().unwrap_or("Sheet1");
    let _name = name; // suppress unused warning
    let worksheet = workbook.add_worksheet();

    for (col, h) in columns.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, h)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    }

    Python::with_gil(|py| {
        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let val = cell.bind(py);
                if let Ok(s) = val.extract::<String>() {
                    worksheet
                        .write_string((row_idx + 1) as u32, col_idx as u16, &s)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
                        })?;
                } else if let Ok(n) = val.extract::<f64>() {
                    worksheet
                        .write_number((row_idx + 1) as u32, col_idx as u16, n)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
                        })?;
                } else if let Ok(b) = val.extract::<bool>() {
                    worksheet
                        .write_boolean((row_idx + 1) as u32, col_idx as u16, b)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
                        })?;
                }
            }
        }
        Ok::<_, PyErr>(())
    })?;

    workbook
        .save_to_buffer()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Generate a carbon report in Markdown format.
pub fn generate_carbon_report_md_impl(
    report: &Bound<'_, PyAny>,
    aoi_name: &str,
    auditor: &str,
) -> PyResult<String> {
    let py = report.py();
    let json_mod = py.import("json")?;
    let json_str: String = json_mod.call_method1("dumps", (report,))?.extract()?;
    let report: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let total = report["total_tco2e"].as_f64().unwrap_or(0.0);

    let mut md = String::new();
    md.push_str(&format!("# 碳核算报告\n\n"));
    md.push_str(&format!("**项目区域**: {aoi_name}\n"));
    md.push_str(&format!("**审核人**: {auditor}\n"));
    md.push_str(&format!(
        "**生成时间**: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    ));
    md.push_str(&format!("## 总碳排放\n\n"));
    md.push_str(&format!("**总计**: {:.2} tCO₂e\n\n", total));

    if let Some(scopes) = report["by_scope"].as_object() {
        md.push_str("| 范围 | 排放量 (tCO₂e) |\n");
        md.push_str("|------|---------------|\n");
        for (scope, val) in scopes {
            md.push_str(&format!(
                "| {} | {:.2} |\n",
                scope,
                val.as_f64().unwrap_or(0.0)
            ));
        }
    }
    if let Some(cats) = report["by_category"].as_object() {
        md.push_str("\n## 分类排放\n\n");
        md.push_str("| 类别 | 排放量 (tCO₂e) |\n");
        md.push_str("|------|---------------|\n");
        for (cat, val) in cats {
            md.push_str(&format!(
                "| {} | {:.2} |\n",
                cat,
                val.as_f64().unwrap_or(0.0)
            ));
        }
    }
    if let Some(lc) = report["by_landcover"].as_object() {
        md.push_str("\n## 土地覆盖明细\n\n");
        md.push_str("| 类型 | 面积 (ha) | 碳储量 (tCO₂e) |\n");
        md.push_str("|------|----------|----------------|\n");
        for (name, info) in lc {
            let area = info["area_ha"].as_f64().unwrap_or(0.0);
            let stock = info["carbon_stock_tco2e"].as_f64().unwrap_or(0.0);
            md.push_str(&format!("| {name} | {area:.2} | {stock:.2} |\n"));
        }
    }
    Ok(md)
}
