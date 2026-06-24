use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

use crate::lithology::{classify_lithology, lithology_from_code};
use crate::stratigraphy::{stratigraphic_column, stratigraphic_model_3d, LayerDefinition};
use crate::structures::{fault_plane_geometry, fold_geometry, structure_attitude};

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "geology", "Geology: stratigraphic 3D modeling, fault/fold geometry, lithology classification", PluginCategory::Process, [
        sync "geo_stratigraphy" => "3D stratigraphic model from DEM and layer definitions" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"layers":{"type":"array","items":{"type":"object","properties":{"name":{"type":"string"},"top_depth_m":{"type":"number"},"base_depth_m":{"type":"number"},"lithology_code":{"type":"string"},"density_kgm3":{"type":"number"}},"required":["name","top_depth_m","base_depth_m","lithology_code"]}},"cols":{"type":"integer"}},"required":["dem","layers","cols"]}) => |args| -> ToolResult {
            let dem: Vec<f64> = args["dem"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let cols = args["cols"].as_u64().unwrap_or(10) as usize;
            let larr = args["layers"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            let layers: Vec<LayerDefinition> = larr.iter().filter_map(|l| {
                let o = l.as_object()?;
                Some(LayerDefinition {
                    name: o.get("name")?.as_str()?.into(),
                    top_depth_m: o.get("top_depth_m")?.as_f64()?,
                    base_depth_m: o.get("base_depth_m")?.as_f64()?,
                    lithology_code: o.get("lithology_code")?.as_str()?.into(),
                    density_kgm3: o.get("density_kgm3").and_then(|v| v.as_f64()).unwrap_or(2500.0),
                })
            }).collect();
            let m = stratigraphic_model_3d(&dem, &layers, cols);
            serde_json::to_value(m).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "geo_structure" => "Fault plane, fold geometry, or structure attitude" ; serde_json::json!({"type":"object","properties":{"mode":{"type":"string","enum":["fault","fold","attitude"],"description":"Which structure to compute"},"name":{"type":"string","default":"Unnamed"},"strike_deg":{"type":"number"},"dip_deg":{"type":"number","default":45.0},"slip_m":{"type":"number","default":0.0},"length_km":{"type":"number","default":10.0},"fault_type":{"type":"string","default":"unknown"},"fold_type":{"type":"string","default":"anticline","enum":["anticline","syncline"]},"interlimb_angle_deg":{"type":"number","default":120.0},"wavelength_km":{"type":"number","default":5.0},"amplitude_m":{"type":"number","default":100.0},"dip_quadrant":{"type":"string","default":"SE","enum":["NW","NE","SE","SW"]}},"required":["mode","strike_deg"]}) => |args| -> ToolResult {
            let mode = args["mode"].as_str().unwrap_or("attitude");
            let strike = args["strike_deg"].as_f64().unwrap_or(0.0);
            let dip = args["dip_deg"].as_f64().unwrap_or(45.0);
            match mode {
                "fault" => {
                    let name = args["name"].as_str().unwrap_or("Fault");
                    let slip = args["slip_m"].as_f64().unwrap_or(0.0);
                    let len = args["length_km"].as_f64().unwrap_or(10.0);
                    let ft = args["fault_type"].as_str().unwrap_or("unknown");
                    let f = fault_plane_geometry(name, strike, dip, slip, len, ft);
                    serde_json::to_value(f).map_err(geo_core::errors::GeoError::Serde)
                },
                "fold" => {
                    let name = args["name"].as_str().unwrap_or("Fold");
                    let ft = args["fold_type"].as_str().unwrap_or("anticline");
                    let ia = args["interlimb_angle_deg"].as_f64().unwrap_or(120.0);
                    let wl = args["wavelength_km"].as_f64().unwrap_or(5.0);
                    let amp = args["amplitude_m"].as_f64().unwrap_or(100.0);
                    let f = fold_geometry(name, ft, strike, ia, wl, amp);
                    serde_json::to_value(f).map_err(geo_core::errors::GeoError::Serde)
                },
                _ => {
                    let dq = args["dip_quadrant"].as_str().unwrap_or("SE");
                    let sa = structure_attitude(strike, dip, dq);
                    serde_json::to_value(sa).map_err(geo_core::errors::GeoError::Serde)
                }
            }
        },

        sync "geo_lithology" => "Classify lithology from geological map codes and return engineering parameters" ; serde_json::json!({"type":"object","properties":{"codes":{"type":"array","items":{"type":"string"},"description":"Geological map codes (e.g. Q, γ, Ss, Lm, Gr, Ba)"}},"required":["codes"]}) => |args| -> ToolResult {
            let codes: Vec<String> = args["codes"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            let result = classify_lithology(&codes);
            serde_json::to_value(result).map_err(geo_core::errors::GeoError::Serde)
        },
    ]);
}
