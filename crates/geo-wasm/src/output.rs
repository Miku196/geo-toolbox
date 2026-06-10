//! Browser-side export generation.
//!
//! Generates GeoJSON, Excel (XLSX), and DXF outputs directly
//! from in-memory data — no server, no PostGIS, no GDAL.
//!
//! All files are returned as Uint8Array for JavaScript Blob/File API download.

use wasm_bindgen::prelude::*;

/// Export a data table to Excel (XLSX) format.
///
/// ## Input
///
/// - `columns`: JSON array of column names, e.g. `["Landcover", "Area (ha)", "tCO₂e"]`
/// - `rows`: JSON array of row arrays (values), e.g. `[["forest", 100, 500], ["grassland", 50, -50]]`
/// - `sheet_name`: Optional sheet name (default: "Sheet1")
///
/// ## Returns
///
/// Uint8Array containing the XLSX file bytes.
#[wasm_bindgen(js_name = exportExcel)]
pub fn export_excel(columns_json: &str, rows_json: &str, sheet_name: Option<String>) -> Result<Box<[u8]>, JsValue> {
    use rust_xlsxwriter::*;

    let columns: Vec<String> = serde_json::from_str(columns_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid columns: {e}")))?;
    let rows: Vec<Vec<serde_json::Value>> = serde_json::from_str(rows_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid rows: {e}")))?;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    let name = sheet_name.unwrap_or_else(|| "Sheet1".into());
    sheet.set_name(&name)
        .map_err(|e| JsValue::from_str(&format!("Sheet name error: {e}")))?;

    // Header style
    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .clone();

    // Write headers
    for (i, col_name) in columns.iter().enumerate() {
        sheet.write_string_with_format(0, i as u16, col_name, &header_fmt)
            .map_err(|e| JsValue::from_str(&format!("Write header error: {e}")))?;
    }

    // Write data rows
    for (row_idx, row) in rows.iter().enumerate() {
        let r = (row_idx + 1) as u32;
        for (col_idx, value) in row.iter().enumerate() {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        sheet.write_number(r, col_idx as u16, f).ok();
                    } else if let Some(i) = n.as_i64() {
                        sheet.write_number(r, col_idx as u16, i as f64).ok();
                    }
                }
                serde_json::Value::String(s) => {
                    sheet.write_string(r, col_idx as u16, s).ok();
                }
                serde_json::Value::Bool(b) => {
                    sheet.write_boolean(r, col_idx as u16, *b).ok();
                }
                _ => {
                    sheet.write_string(r, col_idx as u16, "").ok();
                }
            }
        }
    }

    // Auto-fit column widths
    for (i, name) in columns.iter().enumerate() {
        let w = (name.len() as u16).max(12);
        sheet.set_column_width(i as u16, w + 4).ok();
    }
    sheet.set_freeze_panes(1, 0).ok();

    // Write to buffer
    let buffer = workbook.save_to_buffer()
        .map_err(|e| JsValue::from_str(&format!("XLSX save: {e}")))?;

    Ok(buffer.into_boxed_slice())
}

/// Export data as GeoJSON FeatureCollection.
///
/// ## Input
///
/// - `features`: JSON array of GeoJSON Feature objects.
///
/// ## Returns
///
/// JSON string of the FeatureCollection.
#[wasm_bindgen(js_name = exportGeoJson)]
pub fn export_geojson(features_json: &str) -> Result<String, JsValue> {
    let features: Vec<serde_json::Value> = serde_json::from_str(features_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid features: {e}")))?;

    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
    });

    serde_json::to_string_pretty(&fc)
        .map_err(|e| JsValue::from_str(&format!("Serialization: {e}")))
}

/// Generate a carbon accounting Markdown report.
///
/// ## Input
///
/// - `report_json`: JSON string of a CarbonReport (from CarbonEngine.calculate).
/// - `aoi_name`: Name of the area of interest (e.g., "Chengdu High-tech Zone").
/// - `auditor`: Name of the auditor/operator.
///
/// ## Returns
///
/// Markdown string suitable for download or preview.
#[wasm_bindgen(js_name = exportCarbonReport)]
pub fn export_carbon_report(report_json: &str, aoi_name: &str, auditor: &str) -> Result<String, JsValue> {
    let report: serde_json::Value = serde_json::from_str(report_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid report: {e}")))?;

    let classes = report["classes"].as_array()
        .ok_or_else(|| JsValue::from_str("Report has no 'classes' array"))?;

    let total_area = report["total_area_ha"].as_f64().unwrap_or(0.0);
    let total_emission = report["total_emission_tco2e"].as_f64().unwrap_or(0.0);
    let year = report["year"].as_u64().unwrap_or(0);
    let calculated_at = report["calculated_at"].as_str().unwrap_or("");

    let mut md = String::new();

    md.push_str("# Carbon Accounting Report\n\n");
    md.push_str(&format!("**AOI:** {aoi_name}  \n"));
    md.push_str(&format!("**Year:** {year}  \n"));
    md.push_str(&format!("**Auditor:** {auditor}  \n"));
    md.push_str(&format!("**Calculated:** {calculated_at}  \n\n"));

    md.push_str("---\n\n");
    md.push_str("## Summary\n\n");
    md.push_str("| Metric | Value |\n");
    md.push_str("|--------|-------|\n");
    md.push_str(&format!("| Total Area | {:.1} ha |\n", total_area));
    md.push_str(&format!("| Net Emissions | {:.1} tCO₂e |\n", total_emission));
    md.push_str("\n---\n\n");

    md.push_str("## Landcover Breakdown\n\n");
    md.push_str("| Landcover Class | Area (ha) | Factor | Emission (tCO₂e) | Features | Source |\n");
    md.push_str("|----------------|-----------|--------|-----------------|----------|--------|\n");

    for class in classes {
        let name = class["landcover_class"].as_str().unwrap_or("?");
        let area = class["area_ha"].as_f64().unwrap_or(0.0);
        let factor = class["factor_value"].as_f64().unwrap_or(0.0);
        let emission = class["emission_tco2e"].as_f64().unwrap_or(0.0);
        let count = class["feature_count"].as_u64().unwrap_or(0);
        let source = class["factor_source"].as_str().unwrap_or("?");

        md.push_str(&format!(
            "| {name} | {area:.1} | {factor:.2} | {emission:.1} | {count} | {source} |\n"
        ));
    }

    md.push_str("\n---\n\n");
    md.push_str("*Report generated by geo-wasm — all computation performed client-side. No data transmitted.*\n");

    Ok(md)
}

/// Convert CSV text to JSON array of objects.
///
/// ## Input
///
/// - `csv_text`: Raw CSV string with header row.
///
/// ## Returns
///
/// JSON string: array of row objects.
#[wasm_bindgen(js_name = csvToJson)]
pub fn csv_to_json(csv_text: &str) -> Result<String, JsValue> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_text.as_bytes());

    let headers: Vec<String> = reader.headers()
        .map_err(|e| JsValue::from_str(&format!("CSV headers: {e}")))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut rows: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();

    for result in reader.records() {
        let record = result
            .map_err(|e| JsValue::from_str(&format!("CSV row: {e}")))?;
        let mut row = serde_json::Map::new();
        for (i, value) in record.iter().enumerate() {
            let key = headers.get(i).cloned().unwrap_or_else(|| format!("col_{i}"));
            // Try parse as number
            if let Ok(num) = value.parse::<f64>() {
                row.insert(key, serde_json::Value::Number(
                    serde_json::Number::from_f64(num).unwrap_or(serde_json::Number::from(0))
                ));
            } else {
                row.insert(key, serde_json::Value::String(value.to_string()));
            }
        }
        rows.push(row);
    }

    serde_json::to_string_pretty(&rows)
        .map_err(|e| JsValue::from_str(&format!("JSON: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_geojson() {
        let features = r#"[{"type":"Feature","geometry":{"type":"Point","coordinates":[104,30]},"properties":{"name":"test"}}]"#;
        let result = export_geojson(features).unwrap();
        assert!(result.contains("FeatureCollection"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_csv_to_json() {
        let csv = "name,lat,lng\nSiteA,30.5,104.0\nSiteB,30.6,104.1\n";
        let result = csv_to_json(csv).unwrap();
        let arr: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "SiteA");
        assert!((arr[0]["lat"].as_f64().unwrap() - 30.5).abs() < 0.01);
    }

    #[test]
    fn test_carbon_report_markdown() {
        let report = r#"{
            "classes": [
                {"landcover_class":"forest","area_ha":100.0,"factor_value":5.0,"emission_tco2e":500.0,"factor_source":"IPCC","feature_count":1},
                {"landcover_class":"grassland","area_ha":50.0,"factor_value":-1.0,"emission_tco2e":-50.0,"factor_source":"IPCC","feature_count":2}
            ],
            "total_area_ha": 150.0,
            "total_emission_tco2e": 450.0,
            "year": 2025,
            "calculated_at": "2025-01-01T00:00:00Z"
        }"#;

        let md = export_carbon_report(report, "Test Zone", "Alice").unwrap();
        assert!(md.contains("Test Zone"));
        assert!(md.contains("forest"));
        assert!(md.contains("450.0"));
        assert!(md.contains("client-side"));
    }
}
