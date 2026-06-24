//! Tool registration — RemoteSensing plugin
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "remote-sensing", "Radiometric correction, atmospheric correction, InSAR displacement monitoring", PluginCategory::Process, [
        sync "remote_toa_radiance" => "TOA radiance: DN × gain + bias (per band)" ; serde_json::json!({"type":"object","properties":{"dn_bands":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"gain":{"type":"array","items":{"type":"number"}},"bias":{"type":"array","items":{"type":"number"}}},"required":["dn_bands","gain","bias"]}) => |args| -> ToolResult {
            let dn: Vec<Vec<f64>> = args["dn_bands"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().map(|b| b.as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect()).collect();
            let gain: Vec<f64> = args["gain"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let bias: Vec<f64> = args["bias"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let result = crate::radiometric::toa_radiance(&dn, &gain, &bias);
            Ok(serde_json::json!({"toa_radiance": result}))
        },
        sync "remote_full_pipeline" => "Full radiometric pipeline: DN → TOA radiance → TOA reflectance → DOS correction → cloud mask" ; serde_json::json!({"type":"object","properties":{"dn_bands":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"gain":{"type":"array","items":{"type":"number"}},"bias":{"type":"array","items":{"type":"number"}},"sun_elevation_deg":{"type":"number","default":50},"sun_earth_distance_au":{"type":"number","default":1.0},"red_band_idx":{"type":"integer","default":3},"nir_band_idx":{"type":"integer","default":4}},"required":["dn_bands","gain","bias"]}) => |args| -> ToolResult {
            let dn: Vec<Vec<f64>> = args["dn_bands"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().map(|b| b.as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect()).collect();
            let gain: Vec<f64> = args["gain"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let bias: Vec<f64> = args["bias"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let sun_el = args["sun_elevation_deg"].as_f64().unwrap_or(50.0);
            let sed = args["sun_earth_distance_au"].as_f64().unwrap_or(1.0);
            let red = args["red_band_idx"].as_u64().unwrap_or(3) as usize;
            let nir = args["nir_band_idx"].as_u64().unwrap_or(4) as usize;
            let result = crate::radiometric::full_radiometric_pipeline(&dn, &gain, &bias, sun_el, sed, 0.01, 0.2, red, nir);
            serde_json::to_value(result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "remote_cloud_mask" => "Cloud mask from red+NIR bands using NDVI + brightness threshold" ; serde_json::json!({"type":"object","properties":{"red_band":{"type":"array","items":{"type":"number"}},"nir_band":{"type":"array","items":{"type":"number"}},"ndvi_threshold":{"type":"number","default":0.2},"brightness_threshold":{"type":"number","default":0.3}},"required":["red_band","nir_band"]}) => |args| -> ToolResult {
            let red: Vec<f64> = args["red_band"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let nir: Vec<f64> = args["nir_band"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let ndvi_t = args["ndvi_threshold"].as_f64().unwrap_or(0.2);
            let bright_t = args["brightness_threshold"].as_f64().unwrap_or(0.3);
            let mask = crate::radiometric::cloud_mask(&red, &nir, ndvi_t, bright_t);
            let cloud_count = mask.iter().filter(|&&m| m).count();
            Ok(serde_json::json!({"cloud_mask": mask, "cloud_pixels": cloud_count, "total_pixels": mask.len()}))
        },
        sync "remote_insar_coherence" => "InSAR coherence between master and slave SLC images" ; serde_json::json!({"type":"object","properties":{"master":{"type":"array","items":{"type":"number"}},"slave":{"type":"array","items":{"type":"number"}},"window":{"type":"integer","default":5}},"required":["master","slave"]}) => |args| -> ToolResult {
            let m: Vec<f64> = args["master"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let s: Vec<f64> = args["slave"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let w = args["window"].as_u64().unwrap_or(5) as usize;
            let coh = crate::insar::coherence(&m, &s, w);
            let mean_coh = if coh.is_empty() { 0.0 } else { coh.iter().sum::<f64>() / coh.len() as f64 };
            Ok(serde_json::json!({"coherence": coh, "mean_coherence": (mean_coh*10000.0).round()/10000.0}))
        },
        sync "remote_insar_full" => "Full InSAR pipeline: coherence → wrapped phase → unwrapping → LOS displacement" ; serde_json::json!({"type":"object","properties":{"master":{"type":"array","items":{"type":"number"}},"slave":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"window":{"type":"integer","default":5},"coherence_threshold":{"type":"number","default":0.3},"wavelength_cm":{"type":"number","default":5.6}},"required":["master","slave","cols"]}) => |args| -> ToolResult {
            let m: Vec<f64> = args["master"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let s: Vec<f64> = args["slave"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|v| v.as_f64()).collect();
            let cols = args["cols"].as_u64().unwrap_or(1) as usize;
            let w = args["window"].as_u64().unwrap_or(5) as usize;
            let ct = args["coherence_threshold"].as_f64().unwrap_or(0.3);
            let wl = args["wavelength_cm"].as_f64().unwrap_or(5.6);
            let result = crate::insar::full_insar_pipeline(&m, &s, w, cols, ct, wl, None);
            let disp_mm_mean = if result.los_displacement_m.is_empty() { 0.0 } else {
                result.los_displacement_m.iter().sum::<f64>() / result.los_displacement_m.len() as f64 * 1000.0
            };
            Ok(serde_json::json!({
                "mean_coherence": (result.mean_coherence*10000.0).round()/10000.0,
                "mean_displacement_mm": (disp_mm_mean*100.0).round()/100.0,
                "unwrap_anomalies": result.unwrap_anomaly_regions,
                "coherence": result.coherence,
                "los_displacement_m": result.los_displacement_m,
            }))
        },
        sync "remote_insar_displacement_class" => "Classify InSAR displacement magnitude" ; serde_json::json!({"type":"object","properties":{"displacement_mm":{"type":"number"}},"required":["displacement_mm"]}) => |args| -> ToolResult {
            let d = args["displacement_mm"].as_f64().unwrap_or(0.0);
            let cls = crate::insar::displacement_class(d);
            Ok(serde_json::json!({"class": cls, "displacement_mm": d}))
        },
    ]);
}
