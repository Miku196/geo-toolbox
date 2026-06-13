use geo_registry::PluginRegistry;
use super::super::OutputAction;

/// Create a Postgres connection pool from DATABASE_URL env var.
async fn connect_db() -> Result<sqlx::PgPool, Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must be set")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2).connect(&db_url).await?;
    Ok(pool)
}

/// Recursively transform all coordinate pairs in a GeoJSON geometry.
/// Returns an error if a coordinate value cannot be parsed as f64.
fn transform_geojson_coords(
    reg: &geo_core::crs::CrsRegistry, value: &mut serde_json::Value,
    from_epsg: u16, to_epsg: u16, depth: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    const MAX_DEPTH: u32 = 64;
    if depth > MAX_DEPTH {
        return Err("Coordinate nesting exceeds 64 levels".into());
    }
    if let serde_json::Value::Array(arr) = value {
        if arr.len() == 2 && arr[0].is_number() && arr[1].is_number() && !arr[0].is_array() {
            let x = arr[0].as_f64()
                .ok_or_else(|| format!("Invalid coordinate value: {}", arr[0]))?;
            let y = arr[1].as_f64()
                .ok_or_else(|| format!("Invalid coordinate value: {}", arr[1]))?;
            if let Ok((nx, ny)) = reg.transform_point(from_epsg, to_epsg, x, y) {
                arr[0] = serde_json::json!(nx);
                arr[1] = serde_json::json!(ny);
            }
        } else {
            for item in arr.iter_mut() {
                transform_geojson_coords(reg, item, from_epsg, to_epsg, depth + 1)?;
            }
        }
    }
    Ok(())
}

/// Handle GeoJSON from-file mode (local file → transform → write).
async fn handle_geojson_from_file(
    path: &str, output: &str, to_epsg: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut geojson: serde_json::Value = serde_json::from_str(&content)?;
    let in_size = content.len();
    let count = geojson.get("features").and_then(|f| f.as_array()).map(|f| f.len()).unwrap_or(0);
    if let Some(epsg) = to_epsg {
        if epsg != 4326 {
            let reg = geo_core::crs::CrsRegistry::new();
            if let Some(features) = geojson.get_mut("features").and_then(|f| f.as_array_mut()) {
                for feat in features.iter_mut() {
                    if let Some(geom) = feat.get_mut("geometry") {
                        if let Some(coords) = geom.get_mut("coordinates") {
                            transform_geojson_coords(&reg, coords, 4326, epsg, 0)?;
                        }
                    }
                }
            }
        }
    }
    let compact = serde_json::to_string(&geojson)?;
    let out_size = compact.len();
    std::fs::write(output, &compact)?;
    println!("GeoJSON: {output} ({count} features, {in_size}→{out_size} bytes, {:.0}%)",
        out_size as f64 / in_size as f64 * 100.0);
    Ok(())
}

/// Handle GeoJSON from PostGIS mode (SQL query → export).
#[cfg(all(feature = "postgis", feature = "cad"))]
async fn handle_geojson_from_db(
    sql: &str, output: &str, aggregate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = connect_db().await?;
    let exporter = geo_adapter_cad::GeoJsonExporter::new(pool);
    let count = if aggregate {
        exporter.from_aggregate_sql(sql, output).await?
    } else {
        exporter.from_sql(sql, output).await?
    };
    println!("GeoJSON: {output} ({count} features)");
    Ok(())
}

/// Handle Report generation (PostGIS query → template render).
#[cfg(feature = "postgis")]
async fn handle_report(
    aoi: &str, year: u16, name: &str, source: &str, format: &str, output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = connect_db().await?;
    let aoi_id = uuid::Uuid::parse_str(aoi)?;
    let engine = geo_adapter_postgis::PostgisCarbonEngine::new(pool);
    let results = engine.query_by_aoi(aoi_id).await?;
    let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();
    let breakdown: Vec<geo_report::report::LandcoverBreakdown> = results.iter().map(|r| geo_report::report::LandcoverBreakdown {
        class: r.landcover_class.clone(), area_ha: r.area_ha, factor: r.factor_value, tco2e: r.emission_tco2e,
    }).collect();
    let audit_trails: Vec<geo_report::report::AuditTrailEntry> = results.iter().map(|r| geo_report::report::AuditTrailEntry {
        class: r.landcover_class.clone(),
        lc_hash: r.audit.lc_dvc_hash.clone().unwrap_or_default(),
        factor_id: r.audit.factor_set_id.clone(),
        factor_hash: r.audit.factor_dvc_hash.clone().unwrap_or_default(),
        complete: r.audit.is_complete(),
    }).collect();
    let report_data = geo_report::report::CarbonReportData {
        title: format!("Carbon Accounting Report: {name}"),
        aoi_name: name.to_string(), year,
        generated_at: chrono::Utc::now().to_rfc3339(),
        source: source.to_string(), total_tco2e: total,
        breakdown, audit_trails,
    };
    let gen = geo_report::ReportGenerator::new()?;
    match format {
        "html" => { let html = gen.html_report(&report_data)?; gen.save_report(&html, output)?; }
        _ => { let md = gen.carbon_report(&report_data)?; gen.save_report(&md, output)?; }
    }
    println!("Report: {output}");
    Ok(())
}

/// Dispatch output action to the appropriate handler.
pub async fn handle(_registry: &PluginRegistry, action: OutputAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        #[cfg(feature = "cad")]
        OutputAction::Excel { sql, output, sheet } => {
            let pool = connect_db().await?;
            let dashboard = geo_adapter_cad::ExcelDashboard::new(pool);
            dashboard.from_sql(&sql, &output, &sheet).await?;
            println!("Excel dashboard: {output}");
        }

        OutputAction::Geojson { sql, output, aggregate, from_file, to_epsg } => {
            if let Some(path) = from_file {
                handle_geojson_from_file(&path, &output, to_epsg).await?;
                return Ok(());
            }

            #[cfg(all(feature = "postgis", feature = "cad"))]
            {
                let sql = sql.ok_or("--sql required for PostGIS mode")?;
                handle_geojson_from_db(&sql, &output, aggregate).await?;
            }
            #[cfg(not(all(feature = "postgis", feature = "cad")))]
            {
                let _ = (sql, aggregate);
                println!("GeoJSON SQL export requires --features postgis,cad");
            }
        }

        #[cfg(all(feature = "postgis", feature = "cad"))]
        OutputAction::Dxf { sql, output, from_epsg, to_epsg } => {
            let pool = connect_db().await?;
            let exporter = geo_adapter_cad::DxfExporter::new(pool);
            let count = exporter.from_sql(&sql, &output, from_epsg, to_epsg).await?;
            println!("DXF: {output} ({count} entities)");
        }

        OutputAction::Report { aoi, year, name, source, format, output } => {
            #[cfg(feature = "postgis")]
            {
                handle_report(&aoi, year, &name, &source, &format, &output).await?;
            }
            #[cfg(not(feature = "postgis"))]
            {
                let _ = (&aoi, &year, &name, &source, &format, &output);
                println!("Report requires --features postgis");
            }
        }
    }
    Ok(())
}
