//! Carbon accounting subcommand handler.

use super::super::{CarbonAction, EfAction};
use uuid::Uuid;

/// Handle `carbon emission-factor | lca`.
pub async fn handle(action: CarbonAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        CarbonAction::EmissionFactor { action } => handle_ef(action).await,
        CarbonAction::Lca { inventory } => {
            let result = geo_carbon::lca::submit_lca(&inventory)?;
            println!("{result}");
            Ok(())
        }
    }
}

async fn handle_ef(action: EfAction) -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://geo:geo@localhost:5432/geo_test".to_string());

    match action {
        EfAction::Register { csv } => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&db_url)
                .await?;

            let engine = geo_carbon::CarbonEngine::new(pool);
            let count = engine.import_factors_csv(&csv).await?;
            println!("Imported {count} emission factors from {csv}");
        }

        EfAction::Calculate { aoi, year, source } => {
            let aoi_id = Uuid::parse_str(&aoi)?;

            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&db_url)
                .await?;

            let engine = geo_carbon::CarbonEngine::new(pool);

            let results = engine
                .calculate_emission_factor(aoi_id, year, &source)
                .await?;

            print_results(&results, aoi_id, year, &source);
        }
    }
    Ok(())
}

fn print_results(results: &[geo_carbon::EmissionResult], aoi_id: Uuid, year: u16, source: &str) {
    let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();

    println!("\n═══ Carbon Accounting Results ═══");
    println!("AOI:              {aoi_id}");
    println!("Year:             {year}");
    println!("Factor source:    {source}\n");

    println!(
        "{:<22} {:>10} {:>12} {:>14}  Audit",
        "Landcover Class", "Area(ha)", "Factor", "tCO₂e"
    );
    println!("{}", "─".repeat(75));

    for r in results {
        let audit = if r.audit.is_complete() { "✓" } else { "?" };
        println!(
            "{:<22} {:>10.1} {:>12.2} {:>14.1}  {audit:>5}",
            r.landcover_class, r.area_ha, r.factor_value, r.emission_tco2e,
        );
    }

    println!("{}", "─".repeat(75));
    println!("{:<22} {:>10} {:>12} {:>14.1}", "TOTAL", "", "", total);

    println!("\nAudit Trail:");
    for r in results {
        println!("  {}: {}", r.landcover_class, r.audit.summary());
    }
    println!();
}
