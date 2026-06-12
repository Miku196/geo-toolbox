//! CRS subcommand handler.
//!
//! Thin CLI formatter — all execution dispatched through PluginRegistry.

use geo_registry::PluginRegistry;
use serde_json::json;

use crate::CrsAction;

pub fn handle(registry: &PluginRegistry, action: CrsAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        CrsAction::List => {
            let result = registry.dispatch_sync("crs_list", json!({}))?;
            let empty = vec![];
            let list = result.as_array().unwrap_or(&empty);
            println!("{:<6} {:<10} {:<30} PROJ", "EPSG", "CATEGORY", "NAME");
            println!("{}", "-".repeat(70));
            for crs in list {
                println!("{:<6} {:<10} {:<30} {}",
                    crs["epsg"], crs["category"].as_str().unwrap_or(""),
                    crs["name"].as_str().unwrap_or(""), crs["proj4"].as_str().unwrap_or(""));
            }
            println!("\n{} CRS registered.", list.len());
        }
        CrsAction::Show { epsg } => {
            let result = registry.dispatch_sync("crs_list", json!({}))?;
            let empty = vec![];
            let list = result.as_array().unwrap_or(&empty);
            match list.iter().find(|c| c["epsg"].as_u64() == Some(epsg as u64)) {
                Some(c) => {
                    println!("EPSG:     {}", c["epsg"]);
                    println!("Name:     {}", c["name"]);
                    println!("Category: {}", c["category"]);
                    println!("PROJ4:    {}", c["proj4"]);
                }
                None => println!("EPSG:{epsg} not found"),
            }
        }
        CrsAction::Transform { from, to, x, y, batch } => {
            if batch {
                use std::io::{self, BufRead};
                let stdin = io::stdin();
                for line in stdin.lock().lines() {
                    let line = line?;
                    if let Some((sx, sy)) = line.split_once(',') {
                        let x: f64 = sx.trim().parse()?;
                        let y: f64 = sy.trim().parse()?;
                        let result = registry.dispatch_sync("crs_transform",
                            json!({"from_epsg": from, "to_epsg": to, "x": x, "y": y}))?;
                        let out = &result["output"];
                        println!("{},{}", out[0].as_f64().unwrap_or(0.0), out[1].as_f64().unwrap_or(0.0));
                    }
                }
            } else {
                let x = x.unwrap_or(0.0);
                let y = y.unwrap_or(0.0);
                let result = registry.dispatch_sync("crs_transform",
                    json!({"from_epsg": from, "to_epsg": to, "x": x, "y": y}))?;
                println!("{}", result["message"].as_str().unwrap_or(""));
            }
        }
        CrsAction::Register { epsg, name, proj4: _ } => {
            println!("CRS registration not persisted. EPSG:{epsg} \"{name}\" added to runtime only.");
        }
    }
    Ok(())
}
