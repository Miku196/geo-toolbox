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

        CrsAction::Transform { from, to, x, y, batch } => {
            let reg = CrsRegistry::new();

            if batch {
                // Read "x,y" from stdin, one per line, write transformed result
                use std::io::{self, BufRead};
                let stdin = io::stdin();
                for line in stdin.lock().lines() {
                    let line = line?;
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') { continue; }
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() < 2 {
                        eprintln!("Skipping invalid line (expected x,y): {line}");
                        continue;
                    }
                    let px: f64 = parts[0].trim().parse()?;
                    let py: f64 = parts[1].trim().parse()?;
                    match reg.transform_point(from, to, px, py) {
                        Ok((ox, oy)) => println!("{ox:.6},{oy:.6}"),
                        Err(e) => eprintln!("Error ({px},{py}): {e}"),
                    }
                }
            } else {
                let x = x.ok_or("x coordinate required (or use --batch)")?;
                let y = y.ok_or("y coordinate required (or use --batch)")?;
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
        }

        CrsAction::Register { epsg, name, proj4 } => {
            println!("CRS registration (runtime-only) not yet implemented.");
            println!("Would register EPSG:{epsg} \"{name}\" with PROJ: {proj4}");
        }
    }

    Ok(())
}
