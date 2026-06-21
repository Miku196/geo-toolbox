//! Tool registration — Urban plugin.
use crate::UrbanPlugin;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
fn default_plugin() -> UrbanPlugin {
    UrbanPlugin::new(Default::default())
}
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "urban", "Urban planning: FAR, land use, solar analysis, UHI, ventilation", PluginCategory::Process, [
        sync "urban_far" => "Compute FAR, building density, average height, and compliance check" ; serde_json::json!({"type":"object","properties":{"total_floor_area_m2":{"type":"number"},"building_footprint_m2":{"type":"number"},"site_area_m2":{"type":"number"}},"required":["total_floor_area_m2","building_footprint_m2","site_area_m2"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let tfa = args["total_floor_area_m2"].as_f64().unwrap_or(0.0);
        let bf = args["building_footprint_m2"].as_f64().unwrap_or(0.0);
        let sa = args["site_area_m2"].as_f64().unwrap_or(0.0);
        let far = p.far(tfa, sa);
        let density = p.building_density(bf, sa);
        let avg_h = p.estimate_avg_height(far, density);
        let (fc, dc) = p.check_compliance(far, density);
        Ok(serde_json::json!({"far":far,"building_density":density,"estimated_avg_height_m":avg_h,"far_compliant":fc,"density_compliant":dc}))
    },
        sync "urban_land_use" => "Classify land use (NLCD) from NDVI and impervious surface arrays" ; serde_json::json!({"type":"object","properties":{"ndvi_values":{"type":"array","items":{"type":"number"}},"impervious_values":{"type":"array","items":{"type":"number"}},"total_area_ha":{"type":"number"}}}) => |args| -> ToolResult {
        let p = default_plugin();
        let ndvi: Vec<Option<f64>> = args["ndvi_values"].as_array().map(|a| a.iter().map(|v| v.as_f64()).collect()).unwrap_or_default();
        let imp: Vec<Option<f64>> = args["impervious_values"].as_array().map(|a| a.iter().map(|v| v.as_f64()).collect()).unwrap_or_default();
        let stats = p.land_use_stats(&ndvi, &imp, args["total_area_ha"].as_f64().unwrap_or(0.0));
        Ok(serde_json::json!({"land_use_areas_ha": stats}))
    },
        sync "urban_heat_island" => "Compute urban heat island index" ; serde_json::json!({"type":"object","properties":{"impervious_ratio":{"type":"number"},"building_density":{"type":"number"},"green_ratio":{"type":"number"}},"required":["impervious_ratio","building_density","green_ratio"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let uhi = p.uhi_index(args["impervious_ratio"].as_f64().unwrap_or(0.0), args["building_density"].as_f64().unwrap_or(0.0), args["green_ratio"].as_f64().unwrap_or(0.0));
        Ok(serde_json::json!({"uhi_index":uhi.uhi_index,"risk_level":uhi.risk_level}))
    },
        sync "urban_green_space" => "Compute green ratio, per capita green space, and compliance" ; serde_json::json!({"type":"object","properties":{"green_area_m2":{"type":"number"},"total_area_m2":{"type":"number"},"population":{"type":"integer"}},"required":["green_area_m2","total_area_m2"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let ga = args["green_area_m2"].as_f64().unwrap_or(0.0);
        let ta = args["total_area_m2"].as_f64().unwrap_or(0.0);
        let pop = args["population"].as_u64().unwrap_or(0);
        let ratio = p.green_ratio(ga, ta);
        let pc = p.green_per_capita(ga, pop);
        let min_ratio = p.config().vegetation.min_green_ratio;
        let min_pc = p.config().vegetation.min_green_per_capita_m2;
        Ok(serde_json::json!({"green_ratio":ratio,"green_per_capita_m2":pc,"ratio_compliant":ratio>=min_ratio,"per_capita_compliant":pc>=min_pc}))
    },
        sync "urban_solar" => "Solar / shadow analysis for a building (winter + summer)" ; serde_json::json!({"type":"object","properties":{"building_height_m":{"type":"number"},"neighbor_distance_m":{"type":"number"}},"required":["building_height_m"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let sr = p.solar_analysis(args["building_height_m"].as_f64().unwrap_or(30.0), args["neighbor_distance_m"].as_f64().unwrap_or(50.0));
        Ok(crate::trait_impl::solar_to_json(&sr))
    },
        sync "urban_assess" => "Comprehensive urban planning assessment (all indicators at once)" ; serde_json::json!({"type":"object","properties":{"total_floor_area_m2":{"type":"number"},"building_footprint_m2":{"type":"number"},"site_area_m2":{"type":"number"},"green_area_m2":{"type":"number"},"population":{"type":"integer"},"impervious_ratio":{"type":"number"},"ndvi_values":{"type":"array","items":{"type":"number"}},"impervious_values":{"type":"array","items":{"type":"number"}}},"required":["total_floor_area_m2","building_footprint_m2","site_area_m2"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let ndvi: Vec<Option<f64>> = args["ndvi_values"].as_array().map(|a| a.iter().map(|v| v.as_f64()).collect()).unwrap_or_default();
        let iv: Vec<Option<f64>> = args["impervious_values"].as_array().map(|a| a.iter().map(|v| v.as_f64()).collect()).unwrap_or_default();
        let a = p.assess(args["total_floor_area_m2"].as_f64().unwrap_or(0.0),args["building_footprint_m2"].as_f64().unwrap_or(0.0),args["site_area_m2"].as_f64().unwrap_or(0.0),args["green_area_m2"].as_f64().unwrap_or(0.0),args["population"].as_u64().unwrap_or(0),args["impervious_ratio"].as_f64().unwrap_or(0.0),&ndvi,&iv);
        serde_json::to_value(&a).map_err(geo_core::errors::GeoError::Serde)
    },
        // ── Urban Flood ──
        sync "urban_flood_inlet_capacity" => "Inlet capacity (curb/grate/area/combination)" ; serde_json::json!({"type":"object","properties":{"inlet_type":{"type":"string","enum":["curb_opening","grate","combination","area"]},"grate_area_m2":{"type":"number"},"clogging_factor":{"type":"number","minimum":0,"maximum":1}},"required":["inlet_type","grate_area_m2"]}) => |args| -> ToolResult {
            let cap = crate::urban_flood::urban_flood_inlet_capacity(args["inlet_type"].as_str().unwrap_or("grate"), args["grate_area_m2"].as_f64().unwrap_or(0.0), args["clogging_factor"].as_f64().unwrap_or(0.0));
            Ok(serde_json::json!({"capacity_m3_s": (cap * 1000.0).round() / 1000.0}))
        },
        sync "urban_flood_pipe_capacity" => "Pipe capacity via Manning's equation" ; serde_json::json!({"type":"object","properties":{"diameter_m":{"type":"number"},"slope":{"type":"number"},"manning_n":{"type":"number","default":0.013}},"required":["diameter_m","slope"]}) => |args| -> ToolResult {
            let cap = crate::urban_flood::urban_flood_pipe_capacity(args["diameter_m"].as_f64().unwrap_or(0.0), args["slope"].as_f64().unwrap_or(0.005), args["manning_n"].as_f64().unwrap_or(0.013));
            Ok(serde_json::json!({"capacity_m3_s": (cap * 1000.0).round() / 1000.0}))
        },
        sync "urban_flood_risk" => "Urban pluvial flood risk from runoff vs drainage" ; serde_json::json!({"type":"object","properties":{"runoff_volume_m3":{"type":"number"},"drainage_capacity_m3_s":{"type":"number"},"area_ha":{"type":"number"},"impervious_ratio":{"type":"number"},"duration_hrs":{"type":"number","default":1}},"required":["runoff_volume_m3","drainage_capacity_m3_s","area_ha"]}) => |args| -> ToolResult {
            let r = crate::urban_flood::urban_flood_inundation(args["runoff_volume_m3"].as_f64().unwrap_or(0.0), args["drainage_capacity_m3_s"].as_f64().unwrap_or(0.0), args["area_ha"].as_f64().unwrap_or(0.0), args["impervious_ratio"].as_f64().unwrap_or(0.5), args["duration_hrs"].as_f64().unwrap_or(1.0));
            serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "urban_flood_dual_drainage" => "Dual drainage (major + minor system) analysis" ; serde_json::json!({"type":"object","properties":{"major_system_capacity_m3_s":{"type":"number"},"minor_system_capacity_m3_s":{"type":"number"},"runoff_m3":{"type":"number"}},"required":["major_system_capacity_m3_s","minor_system_capacity_m3_s","runoff_m3"]}) => |args| -> ToolResult {
            let r = crate::urban_flood::urban_flood_dual_drainage(args["major_system_capacity_m3_s"].as_f64().unwrap_or(0.0), args["minor_system_capacity_m3_s"].as_f64().unwrap_or(0.0), args["runoff_m3"].as_f64().unwrap_or(0.0));
            serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "urban_flood_assessment" => "Comprehensive urban flood assessment" ; serde_json::json!({"type":"object","properties":{"total_runoff_m3":{"type":"number"},"pipe_capacities":{"type":"array","items":{"type":"number"}},"surface_storage_m3":{"type":"number"},"area_ha":{"type":"number"},"impervious_ratio":{"type":"number"}},"required":["total_runoff_m3","pipe_capacities","area_ha"]}) => |args| -> ToolResult {
            let caps: Vec<f64> = args["pipe_capacities"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let r = crate::urban_flood::urban_flood_assessment(args["total_runoff_m3"].as_f64().unwrap_or(0.0), &caps, args["surface_storage_m3"].as_f64().unwrap_or(0.0), args["area_ha"].as_f64().unwrap_or(0.0), args["impervious_ratio"].as_f64().unwrap_or(0.5));
            serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
    },
        // ── Accessibility / 15-minute City ──
        sync "urban_accessibility_score" => "15-minute city: facility accessibility score" ; serde_json::json!({"type":"object","properties":{"facilities":{"type":"array","items":{"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"type":{"type":"string"}}}},"origin_lat":{"type":"number"},"origin_lon":{"type":"number"},"max_walk_min":{"type":"number","default":15},"walking_speed_kmh":{"type":"number","default":5}},"required":["facilities","origin_lat","origin_lon"]}) => |args| -> ToolResult {
            let facilities: Vec<(f64,f64,String)> = args["facilities"].as_array().map(|a| a.iter().filter_map(|v| {
                let lat = v["lat"].as_f64()?;
                let lon = v["lon"].as_f64()?;
                let ftype = v["type"].as_str()?.to_string();
                Some((lat, lon, ftype))
            }).collect()).unwrap_or_default();
            let frefs: Vec<(f64,f64,&str)> = facilities.iter().map(|(lat, lon, t)| (*lat, *lon, t.as_str())).collect();
            let r = crate::accessibility::accessibility_score(&frefs, args["origin_lat"].as_f64().unwrap_or(0.0), args["origin_lon"].as_f64().unwrap_or(0.0), args["max_walk_min"].as_f64().unwrap_or(15.0), args["walking_speed_kmh"].as_f64().unwrap_or(5.0));
            Ok(r)
        },
        sync "urban_accessibility_assessment" => "15-minute city composite assessment" ; serde_json::json!({"type":"object","properties":{"population":{"type":"integer"},"green_space_ha":{"type":"number"},"transit_stops":{"type":"integer"},"schools":{"type":"integer"},"hospitals":{"type":"integer"},"shops":{"type":"integer"},"area_km2":{"type":"number"}},"required":["population","area_km2"]}) => |args| -> ToolResult {
            let r = crate::accessibility::accessibility_assessment(args["population"].as_u64().unwrap_or(0), args["green_space_ha"].as_f64().unwrap_or(0.0), args["transit_stops"].as_u64().unwrap_or(0), args["schools"].as_u64().unwrap_or(0), args["hospitals"].as_u64().unwrap_or(0), args["shops"].as_u64().unwrap_or(0), args["area_km2"].as_f64().unwrap_or(0.0));
            serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "urban_service_area_gap" => "Service area gap analysis against planning standards" ; serde_json::json!({"type":"object","properties":{"population_density_per_km2":{"type":"number"},"facility_type":{"type":"string","enum":["school","hospital","grocery","park"]},"existing_count":{"type":"integer"},"area_km2":{"type":"number"}},"required":["population_density_per_km2","facility_type","existing_count","area_km2"]}) => |args| -> ToolResult {
            let r = crate::accessibility::service_area_gap(args["population_density_per_km2"].as_f64().unwrap_or(0.0), args["facility_type"].as_str().unwrap_or(""), args["existing_count"].as_u64().unwrap_or(0), args["area_km2"].as_f64().unwrap_or(0.0));
            serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
        },
    ]);
}
