//! CRS subcommand handler.

use geo_core::crs::CrsRegistry;

use super::super::{CrsAction};

/// Handle `crs list` and `crs transform`.
pub fn handle(action: CrsAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        CrsAction::List => {
            let reg = CrsRegistry::new();
            println!("{:<6} {:<10} {:<30} PROJ", "EPSG", "CATEGORY", "NAME");
            println!("{}", "-".repeat(70));
            for crs in reg.list() {
                println!(
                    "{:<6} {:<10?} {:<30} {}",
                    crs.epsg, crs.category, crs.name, crs.proj4
                );
            }
            println!("\n{} CRS registered.", reg.list().count());
        }

        CrsAction::Show { epsg } => {
            let reg = CrsRegistry::new();
            match reg.get(epsg) {
                Some(crs) => {
                    println!("EPSG:     {}", crs.epsg);
                    println!("Name:     {}", crs.name);
                    println!("Category: {:?}", crs.category);
                    println!("PROJ:     {}", crs.proj4);
                }
                None => {
                    eprintln!("CRS EPSG:{} not found. Run `geo-toolbox crs list` to see available.", epsg);
                }
            }
        }

        CrsAction::Transform { from, to, x, y } => {
            let reg = CrsRegistry::new();
            match reg.transform_point(from, to, x, y) {
                Ok((out_x, out_y)) => {
                    println!("Source (EPSG:{from}):  x={x}, y={y}");
                    println!("Target (EPSG:{to}):  x={out_x:.4}, y={out_y:.4}");
                }
                Err(e) => {
                    eprintln!("Transform failed: {e}");
                }
            }
        }

        CrsAction::Register { epsg, name, proj4 } => {
            println!("CRS registration (runtime-only) not yet implemented.");
            println!("Would register EPSG:{epsg} \"{name}\" with PROJ: {proj4}");
        }
    }

    Ok(())
}
