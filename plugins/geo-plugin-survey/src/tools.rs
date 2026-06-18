//! Tool registration — Survey plugin.
use crate::SurveyPlugin;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
fn default_plugin() -> SurveyPlugin {
    SurveyPlugin::new(Default::default())
}
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "survey", "Surveying: grid earthwork, cross-section, TIN, control network adjustment", PluginCategory::Process, [
        sync "survey_earthwork" => "Grid method earthwork calculation (cut/fill/net volumes)" ; serde_json::json!({"type":"object","properties":{"existing_elevation":{"type":"array","items":{"type":"number"}},"design_elevation":{"type":"number"},"grid_cols":{"type":"integer"},"grid_rows":{"type":"integer"}},"required":["existing_elevation","design_elevation","grid_cols","grid_rows"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let elev: Vec<f64> = args["existing_elevation"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let r = p.grid_earthwork(&elev, args["design_elevation"].as_f64().unwrap_or(0.0), args["grid_cols"].as_u64().unwrap_or(0) as usize, args["grid_rows"].as_u64().unwrap_or(0) as usize);
        serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
    },
        sync "survey_cross_section" => "Average end area cross-section earthwork (road/rail)" ; serde_json::json!({"type":"object","properties":{"cut_areas_m2":{"type":"array","items":{"type":"number"}},"fill_areas_m2":{"type":"array","items":{"type":"number"}},"distances_m":{"type":"array","items":{"type":"number"}}},"required":["cut_areas_m2","fill_areas_m2","distances_m"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let cuts: Vec<f64> = args["cut_areas_m2"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let fills: Vec<f64> = args["fill_areas_m2"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let dists: Vec<f64> = args["distances_m"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let n = cuts.len().min(fills.len());
        let sections: Vec<(f64, f64)> = (0..n).map(|i| (cuts[i], fills[i])).collect();
        let r = p.cross_section_earthwork(&sections, &dists);
        serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
    },
        sync "survey_adjustment" => "Control network adjustment (simplified least squares)" ; serde_json::json!({"type":"object","properties":{"observations":{"type":"array"},"initial":{"type":"number"}},"required":["observations"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let obs: Vec<(f64, f64)> = args["observations"].as_array().map(|a| a.iter().filter_map(|v| {let arr=v.as_array()?;Some((arr.first()?.as_f64()?,arr.get(1)?.as_f64()?))}).collect()).unwrap_or_default();
        let r = p.control_network_adjustment(&obs, args["initial"].as_f64().unwrap_or(0.0));
        serde_json::to_value(&r).map_err(geo_core::errors::GeoError::Serde)
    },
        sync "survey_tin" => "TIN (triangular prism) earthwork volume calculation" ; serde_json::json!({"type":"object","properties":{"points":{"type":"array","items":{"type":"object"}},"design_elevation":{"type":"number"}},"required":["points","design_elevation"]}) => |args| -> ToolResult {
        let p = default_plugin();
        let pts: Vec<crate::survey::ElevationPoint> = args["points"].as_array().map(|a| a.iter().filter_map(|v| Some(crate::survey::ElevationPoint{x:v["x"].as_f64()?,y:v["y"].as_f64()?,z:v["z"].as_f64()?})).collect()).unwrap_or_default();
        let vol = p.tin_earthwork(&pts, args["design_elevation"].as_f64().unwrap_or(0.0));
        Ok(serde_json::json!({"volume_m3": vol}))
    },
        sync "survey_gauss_forward" => "Gauss-Krüger forward: (B,L,L0) → plane (X,Y)" ; serde_json::json!({"type":"object","properties":{"b":{"type":"number"},"l":{"type":"number"},"l0":{"type":"number"},"ellipsoid":{"type":"string","default":"CGCS2000"}},"required":["b","l","l0"]}) => |args| -> ToolResult {
        use crate::gauss::{gauss_forward, Ellipsoid};
        let ell = match args["ellipsoid"].as_str().unwrap_or("CGCS2000") {
            "Xian80" => Ellipsoid::Xian80,
            "Beijing54" => Ellipsoid::Beijing54,
            "WGS84" => Ellipsoid::WGS84,
            _ => Ellipsoid::CGCS2000,
        };
        let (x, y) = gauss_forward(args["b"].as_f64().unwrap_or(0.0), args["l"].as_f64().unwrap_or(0.0), args["l0"].as_f64().unwrap_or(0.0), ell);
        Ok(serde_json::json!({"x": x, "y": y}))
    },
        sync "survey_gauss_inverse" => "Gauss-Krüger inverse: plane (X,Y) → (B,L)" ; serde_json::json!({"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"},"l0":{"type":"number"},"ellipsoid":{"type":"string","default":"CGCS2000"}},"required":["x","y","l0"]}) => |args| -> ToolResult {
        use crate::gauss::{gauss_inverse, Ellipsoid};
        let ell = match args["ellipsoid"].as_str().unwrap_or("CGCS2000") {
            "Xian80" => Ellipsoid::Xian80,
            "Beijing54" => Ellipsoid::Beijing54,
            "WGS84" => Ellipsoid::WGS84,
            _ => Ellipsoid::CGCS2000,
        };
        let (b, l) = gauss_inverse(args["x"].as_f64().unwrap_or(0.0), args["y"].as_f64().unwrap_or(0.0), args["l0"].as_f64().unwrap_or(0.0), ell);
        Ok(serde_json::json!({"b": b, "l": l}))
    },
        sync "survey_zone_transform" => "Coordinate zone transform between Gauss-Krüger zones" ; serde_json::json!({"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"},"from_zone":{"type":"integer"},"to_zone":{"type":"integer"},"is_3_degree":{"type":"boolean","default":false},"ellipsoid":{"type":"string","default":"CGCS2000"}},"required":["x","y","from_zone","to_zone"]}) => |args| -> ToolResult {
        use crate::gauss::{zone_transform, Ellipsoid};
        let ell = match args["ellipsoid"].as_str().unwrap_or("CGCS2000") {
            "Xian80" => Ellipsoid::Xian80,
            "Beijing54" => Ellipsoid::Beijing54,
            "WGS84" => Ellipsoid::WGS84,
            _ => Ellipsoid::CGCS2000,
        };
        let (x2, y2) = zone_transform(args["x"].as_f64().unwrap_or(0.0), args["y"].as_f64().unwrap_or(0.0), args["from_zone"].as_u64().unwrap_or(1) as u16, args["to_zone"].as_u64().unwrap_or(1) as u16, args["is_3_degree"].as_bool().unwrap_or(false), ell);
        Ok(serde_json::json!({"x": x2, "y": y2}))
    },
        sync "survey_zone_info" => "Get zone info from longitude (3-degree and 6-degree bands)" ; serde_json::json!({"type":"object","properties":{"lon":{"type":"number"}},"required":["lon"]}) => |args| -> ToolResult {
        use crate::gauss::zone_info;
        let z = zone_info(args["lon"].as_f64().unwrap_or(0.0));
        Ok(serde_json::json!({"zone_6deg": z.zone6, "cm_6deg": z.central_meridian_6_deg, "zone_3deg": z.zone3, "cm_3deg": z.central_meridian_3_deg}))
    }]);
}
