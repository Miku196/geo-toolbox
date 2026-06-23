//! Tool registration — Seismology plugin.
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "seismology", "Seismic hazard: PGA/PGV, PSHA, G-R catalog analysis", PluginCategory::Process, [
        sync "seismology_ground_motion" => "PGA/PGV/intensity from magnitude + distance + site class" ; serde_json::json!({"type":"object","properties":{"magnitude":{"type":"number"},"distance_km":{"type":"number"},"site_class":{"type":"string","default":"II","enum":["I0","I1","II","III","IV"]}},"required":["magnitude","distance_km"]}) => |args| -> ToolResult {
            let mag = args["magnitude"].as_f64().unwrap_or(0.0);
            let dist = args["distance_km"].as_f64().unwrap_or(0.0);
            let site = args["site_class"].as_str().unwrap_or("II");
            let result = crate::ground_motion::ground_motion_assessment(mag, dist, site);
            serde_json::to_value(result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "seismology_response_spectrum" => "Acceleration response spectrum from PGA" ; serde_json::json!({"type":"object","properties":{"pga_g":{"type":"number"},"damping":{"type":"number","default":0.05}},"required":["pga_g"]}) => |args| -> ToolResult {
            let pga = args["pga_g"].as_f64().unwrap_or(0.0);
            let damp = args["damping"].as_f64().unwrap_or(0.05);
            let periods = crate::ground_motion::default_periods();
            let rs = crate::ground_motion::response_spectrum(pga, &periods, damp);
            serde_json::to_value(rs).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "seismology_psha_curve" => "Probabilistic seismic hazard curve for given source(s)" ; serde_json::json!({"type":"object","properties":{"sources":{"type":"array","items":{"type":"object","properties":{"name":{"type":"string"},"longitude":{"type":"number"},"latitude":{"type":"number"},"annual_rate":{"type":"number"},"b_value":{"type":"number"},"m_min":{"type":"number"},"m_max":{"type":"number"}},"required":["name","longitude","latitude","annual_rate","b_value","m_min","m_max"]}},"site_lon":{"type":"number"},"site_lat":{"type":"number"},"site_class":{"type":"string","default":"II"}},"required":["sources","site_lon","site_lat"]}) => |args| -> ToolResult {
            let site_lon = args["site_lon"].as_f64().unwrap_or(0.0);
            let site_lat = args["site_lat"].as_f64().unwrap_or(0.0);
            let site_class = args["site_class"].as_str().unwrap_or("II");
            let sources: Vec<crate::psha::SeismicSource> = serde_json::from_value(args["sources"].clone()).map_err(|e| geo_core::errors::GeoError::Validation(e.to_string()))?;
            let rps = vec![50.0, 100.0, 475.0, 975.0, 2475.0];
            let curve = crate::psha::psha_hazard_curve(&sources, site_lon, site_lat, &rps, site_class);
            serde_json::to_value(curve).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "seismology_uhs" => "Uniform hazard spectrum from seismic sources" ; serde_json::json!({"type":"object","properties":{"sources":{"type":"array","items":{"type":"object"}},"site_lon":{"type":"number"},"site_lat":{"type":"number"},"return_period":{"type":"number","default":475},"site_class":{"type":"string","default":"II"}},"required":["sources","site_lon","site_lat"]}) => |args| -> ToolResult {
            let site_lon = args["site_lon"].as_f64().unwrap_or(0.0);
            let site_lat = args["site_lat"].as_f64().unwrap_or(0.0);
            let rp = args["return_period"].as_f64().unwrap_or(475.0);
            let site_class = args["site_class"].as_str().unwrap_or("II");
            let sources: Vec<crate::psha::SeismicSource> = serde_json::from_value(args["sources"].clone()).map_err(|e| geo_core::errors::GeoError::Validation(e.to_string()))?;
            let periods = vec![0.1, 0.2, 0.4, 0.6, 1.0, 2.0];
            let uhs = crate::psha::uniform_hazard_spectrum(&sources, site_lon, site_lat, &[rp], &periods, 0.05, site_class);
            serde_json::to_value(uhs).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "seismology_deagg" => "PSHA deaggregation: source contribution analysis" ; serde_json::json!({"type":"object","properties":{"sources":{"type":"array","items":{"type":"object"}},"site_lon":{"type":"number"},"site_lat":{"type":"number"},"pga_g":{"type":"number","default":0.1},"site_class":{"type":"string","default":"II"}},"required":["sources","site_lon","site_lat"]}) => |args| -> ToolResult {
            let site_lon = args["site_lon"].as_f64().unwrap_or(0.0);
            let site_lat = args["site_lat"].as_f64().unwrap_or(0.0);
            let pga = args["pga_g"].as_f64().unwrap_or(0.1);
            let site_class = args["site_class"].as_str().unwrap_or("II");
            let sources: Vec<crate::psha::SeismicSource> = serde_json::from_value(args["sources"].clone()).map_err(|e| geo_core::errors::GeoError::Validation(e.to_string()))?;
            let bins = crate::psha::deaggregation(&sources, site_lon, site_lat, pga, site_class);
            serde_json::to_value(bins).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "seismicity_catalog" => "G-R analysis from earthquake catalog magnitudes" ; serde_json::json!({"type":"object","properties":{"magnitudes":{"type":"array","items":{"type":"number"}},"min_mag":{"type":"number","default":3.0},"time_span_years":{"type":"number","default":50.0}},"required":["magnitudes"]}) => |args| -> ToolResult {
            let mags: Vec<f64> = args["magnitudes"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
            let min_mag = args["min_mag"].as_f64().unwrap_or(3.0);
            let years = args["time_span_years"].as_f64().unwrap_or(50.0);
            let result = crate::seismicity::seismicity_analysis(&mags, min_mag, years);
            serde_json::to_value(result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "seismicity_poisson" => "Poisson probability of earthquake occurrence" ; serde_json::json!({"type":"object","properties":{"annual_rate":{"type":"number"},"time_years":{"type":"number"}},"required":["annual_rate","time_years"]}) => |args| -> ToolResult {
            let rate = args["annual_rate"].as_f64().unwrap_or(0.0);
            let years = args["time_years"].as_f64().unwrap_or(50.0);
            let prob = crate::seismicity::poisson_probability(rate, years);
            let tr = crate::seismicity::recurrence_interval(rate);
            let tr_val = if tr.is_finite() {
                serde_json::json!((tr*100.0).round()/100.0)
            } else {
                serde_json::Value::Null
            };
            Ok(serde_json::json!({"probability_pct": (prob*10000.0).round()/100.0, "recurrence_interval_years": tr_val}))
        },
    ]);
}
