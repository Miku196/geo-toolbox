//! Carbon accounting subcommand handler.
//! All execution dispatched through PluginRegistry.

use geo_registry::PluginRegistry;
#[cfg(feature = "postgis")]
use serde_json::json;
use super::super::{CarbonAction, EfAction};

/// Handle `carbon emission-factor | lca`.
pub async fn handle(registry: &PluginRegistry, action: CarbonAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        CarbonAction::EmissionFactor { action } => handle_ef(registry, action).await,
        CarbonAction::Lca { inventory } => {
            let result = geo_plugin_carbon::lca::submit_lca(&inventory)?;
            println!("{result}");
            Ok(())
        }
    }
}

#[cfg(feature = "postgis")]
async fn handle_ef(registry: &PluginRegistry, action: EfAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        EfAction::Register { csv } => {
            let result = registry.dispatch("carbon_import_factors", json!({"csv_path": csv})).await?;
            println!("Imported {} emission factors", result["imported"]);
        }
        EfAction::Calculate { aoi, year, source } => {
            let result = registry.dispatch("carbon_calculate",
                json!({"aoi_id": aoi, "year": year, "source": source})).await?;
            let total = result["total_tco2e"].as_f64().unwrap_or(0.0);
            println!("\n═══ Carbon Accounting Results ═══");
            println!("AOI:              {}", result["aoi_id"]);
            println!("Year:             {}", result["year"]);
            println!("Total tCO₂e:      {:.1}", total);
            if let Some(results) = result["results"].as_array() {
                println!("\n{:<22} {:>10} {:>14}", "Landcover Class", "Area(ha)", "tCO₂e");
                for r in results {
                    println!("{:<22} {:>10.1} {:>14.1}",
                        r["landcover_class"].as_str().unwrap_or(""),
                        r["area_ha"].as_f64().unwrap_or(0.0),
                        r["emission_tco2e"].as_f64().unwrap_or(0.0));
                }
            }
            println!();
        }
    }
    Ok(())
}

#[cfg(not(feature = "postgis"))]
async fn handle_ef(_registry: &PluginRegistry, _action: EfAction) -> Result<(), Box<dyn std::error::Error>> {
    Err("Carbon emission factor requires --features postgis".into())
}
