//! Output subcommand handler (Excel / GeoJSON / DXF / Report).

use super::super::OutputAction;
use uuid::Uuid;

/// Recursively transform all coordinate pairs in a GeoJSON geometry.
fn transform_geojson_coords(
    reg: &geo_core::crs::CrsRegistry,
    value: &mut serde_json::Value,
    from_epsg: u16,
    to_epsg: u16,
) {
    match value {
        serde_json::Value::Array(arr) => {
            // Check if this is a coordinate pair [x, y]
            if arr.len() == 2
                && arr[0].is_number()
                && arr[1].is_number()
                && !arr[0].is_array()
            {
                let x = arr[0].as_f64().unwrap_or(0.0);
                let y = arr[1].as_f64().unwrap_or(0.0);
                if let Ok((nx, ny)) = reg.transform_point(from_epsg, to_epsg, x, y) {
                    arr[0] = serde_json::json!(nx);
                    arr[1] = serde_json::json!(ny);
                }
            } else {
                for item in arr.iter_mut() {
                    transform_geojson_coords(reg, item, from_epsg, to_epsg);
                }
            }
        }
        _ => {}
    }
}

/// Handle `output excel | geojson | dxf | report`.
pub async fn handle(action: OutputAction) -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://geo:geo@localhost:5432/geo_test".to_string());

    match action {
        OutputAction::Excel { sql, output, sheet } => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&db_url)
                .await?;

            let dashboard = geo_output::ExcelDashboard::new(pool);
            dashboard.from_sql(&sql, &output, &sheet).await?;
            println!("Excel dashboard: {output}");
        }

        OutputAction::Geojson { sql, output, aggregate, from_file, to_epsg } => {
            // --from-file mode: local file validation/compaction/reprojection
            if let Some(path) = from_file {
                let content = std::fs::read_to_string(&path)?;
                let mut geojson: serde_json::Value = serde_json::from_str(&content)?;
                let in_size = content.len();

                // Compact: remove extra whitespace for smaller file
                let count = geojson.get("features")
                    .and_then(|f| f.as_array())
                    .map(|f| f.len()).unwrap_or(0);

                // Reproject if requested
                if let Some(epsg) = to_epsg {
                    if epsg != 4326 {
                        let reg = geo_core::crs::CrsRegistry::new();
                        if let Some(features) = geojson.get_mut("features")
                            .and_then(|f| f.as_array_mut())
                        {
                            for feat in features.iter_mut() {
                                if let Some(geom) = feat.get_mut("geometry") {
                                    if let Some(coords) = geom.get_mut("coordinates") {
                                        transform_geojson_coords(&reg, coords, 4326, epsg);
                                    }
                                }
                            }
                        }
                    }
                }

                let compact = serde_json::to_string(&geojson)?;
                let out_size = compact.len();
                std::fs::write(&output, &compact)?;
                println!("GeoJSON: {output} ({count} features, {in_size}→{out_size} bytes, {:.0}%)",
                    out_size as f64 / in_size as f64 * 100.0);
                return Ok(());
            }

            // SQL mode (requires PostGIS)
            let sql = sql.ok_or("--sql required for PostGIS mode")?;
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&db_url)
                .await?;

            let exporter = geo_output::GeoJsonExporter::new(pool);

            let count = if aggregate {
                exporter.from_aggregate_sql(&sql, &output).await?
            } else {
                exporter.from_sql(&sql, &output).await?
            };
            println!("GeoJSON: {output} ({count} features)");
        }

        OutputAction::Dxf { sql, output, from_epsg, to_epsg } => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&db_url)
                .await?;

            let exporter = geo_output::DxfExporter::new(pool);
            let count = exporter.from_sql(&sql, &output, from_epsg, to_epsg).await?;
            println!("DXF: {output} ({count} entities)");
        }

        OutputAction::Report { aoi, year, name, source, format, output } => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&db_url)
                .await?;

            // Query carbon results for the AOI
            let aoi_id = Uuid::parse_str(&aoi)?;
            let engine = geo_carbon::CarbonEngine::new(pool);
            let results = engine.query_by_aoi(aoi_id).await?;

            let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();

            let breakdown: Vec<geo_output::report::LandcoverBreakdown> = results
                .iter()
                .map(|r| geo_output::report::LandcoverBreakdown {
                    class: r.landcover_class.clone(),
                    area_ha: r.area_ha,
                    factor: r.factor_value,
                    tco2e: r.emission_tco2e,
                })
                .collect();

            let audit_trails: Vec<geo_output::report::AuditTrailEntry> = results
                .iter()
                .map(|r| geo_output::report::AuditTrailEntry {
                    class: r.landcover_class.clone(),
                    lc_hash: r.audit.lc_dvc_hash.clone().unwrap_or_default(),
                    factor_id: r.audit.factor_set_id.clone(),
                    factor_hash: r.audit.factor_dvc_hash.clone().unwrap_or_default(),
                    complete: r.audit.is_complete(),
                })
                .collect();

            let report_data = geo_output::report::CarbonReportData {
                title: format!("Carbon Accounting Report: {name}"),
                aoi_name: name,
                year,
                generated_at: "2026-06-06T00:00:00Z".to_string(), // TODO: use real timestamp
                source,
                total_tco2e: total,
                breakdown,
                audit_trails,
            };

            let gen = geo_output::ReportGenerator::new()?;

            match format.as_str() {
                "html" => {
                    let html = gen.html_report(&report_data)?;
                    gen.save_report(&html, &output)?;
                }
                _ => {
                    let md = gen.carbon_report(&report_data)?;
                    gen.save_report(&md, &output)?;
                }
            }

            println!("Report: {output}");
        }
    }
    Ok(())
}
