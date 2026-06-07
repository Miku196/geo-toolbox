//! Output subcommand handler (Excel / GeoJSON / DXF / Report).

use super::super::OutputAction;
use uuid::Uuid;

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

        OutputAction::Geojson { sql, output, aggregate } => {
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
