//! Tool registration — Ecology plugin.
use crate::config::EcologyConfig;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

fn default_plugin() -> crate::ecology::EcologyPlugin {
    crate::ecology::EcologyPlugin::new(EcologyConfig::default())
}

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "ecology", "Ecological restoration assessment: NDVI change", PluginCategory::Process, [
        sync "ecology_ndvi_change" => "NDVI change detection from two RasterBand arrays" ; serde_json::json!({"type":"object","properties":{"red_before":{"type":"array","items":{"type":"number"}},"nir_before":{"type":"array","items":{"type":"number"}},"red_after":{"type":"array","items":{"type":"number"}},"nir_after":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"nodata":{"type":"number","default":-999}},"required":["red_before","nir_before","red_after","nir_after","cols","rows"]}) => |args| -> ToolResult {
            use geo_raster::RasterBand;
            let cols = args["cols"].as_u64().unwrap_or(1) as usize;
            let rows = args["rows"].as_u64().unwrap_or(1) as usize;
            let nodata = args["nodata"].as_f64().unwrap_or(-999.0);
            let red_before: Vec<f64> = args["red_before"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let nir_before: Vec<f64> = args["nir_before"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let red_after: Vec<f64> = args["red_after"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let nir_after: Vec<f64> = args["nir_after"].as_array().map(|a| a.iter().map(|v| v.as_f64().unwrap_or(nodata)).collect()).unwrap_or_default();
            let rb = RasterBand::new("B4_before", cols, rows, red_before, nodata);
            let nb = RasterBand::new("B8_before", cols, rows, nir_before, nodata);
            let ra = RasterBand::new("B4_after", cols, rows, red_after, nodata);
            let na = RasterBand::new("B8_after", cols, rows, nir_after, nodata);
            let p = default_plugin();
            let (prev, curr) = p.detect_ndvi_change(&rb, &nb, &ra, &na)?;
            Ok(serde_json::json!({"mean_ndvi_before": prev.mean_ndvi, "mean_ndvi_after": curr.mean_ndvi, "healthy_ratio_before": prev.healthy_ratio, "healthy_ratio_after": curr.healthy_ratio}))
        },
        sync "ecology_rusle_assessment" => "RUSLE soil loss assessment from DEM + NDVI" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"ndvi":{"type":"array","items":{"type":"number"}},"rows":{"type":"integer"},"cols":{"type":"integer"},"cellsize_m":{"type":"number"},"r_factor":{"type":"number"},"practice":{"type":"string","default":"None","enum":["None","Contouring","StripCropping","Terracing"]}},"required":["dem","ndvi","rows","cols","cellsize_m","r_factor"]}) => |args| -> ToolResult {
            let dem: Vec<f64> = args["dem"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let ndvi: Vec<f64> = args["ndvi"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
            let rows = args["rows"].as_u64().unwrap_or(1) as usize;
            let cols = args["cols"].as_u64().unwrap_or(1) as usize;
            let cellsize_m = args["cellsize_m"].as_f64().unwrap_or(30.0);
            let r_factor = args["r_factor"].as_f64().unwrap_or(4000.0);
            let practice = match args["practice"].as_str().unwrap_or("None") {
                "Contouring" => crate::rusle::PracticeType::Contouring,
                "StripCropping" => crate::rusle::PracticeType::StripCropping,
                "Terracing" => crate::rusle::PracticeType::Terracing,
                _ => crate::rusle::PracticeType::None,
            };
            let result = crate::rusle::assess_soil_loss(&dem, None, cellsize_m, rows, cols, r_factor, None, &ndvi, practice);
            serde_json::to_value(&result).map_err(geo_core::errors::GeoError::Serde)
        },
        sync "ecology_rusle_simple" => "RUSLE with manual factor arrays (R, K, LS, C, P grids)" ; serde_json::json!({"type":"object","properties":{"r_factor":{"type":"array","items":{"type":"number"}},"k_factor":{"type":"array","items":{"type":"number"}},"ls_factor":{"type":"array","items":{"type":"number"}},"c_factor":{"type":"array","items":{"type":"number"}},"p_factor":{"type":"array","items":{"type":"number"}},"cells":{"type":"integer"}},"required":["r_factor","k_factor","ls_factor","c_factor","p_factor","cells"]}) => |args| -> ToolResult {
            let cells = args["cells"].as_u64().unwrap_or(1) as usize;
            let extract = |key: &str| -> Vec<f64> { args[key].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default() };
            let r = extract("r_factor");
            let k = extract("k_factor");
            let ls = extract("ls_factor");
            let c = extract("c_factor");
            let p = extract("p_factor");
            let loss = crate::rusle::compute_soil_loss(&r, &k, &ls, &c, &p, cells);
            Ok(serde_json::json!({"soil_loss_grid": loss, "mean_loss": if cells > 0 { loss.iter().sum::<f64>() / cells as f64 } else { 0.0 }}))
        },
        sync "ecology_rf_lulc" => "Random Forest LULC classification (NDVI, NDWI, NDBI → land cover class)" ; serde_json::json!({"type":"object","properties":{"features":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"num_trees":{"type":"integer","default":10},"max_depth":{"type":"integer","default":5}},"required":["features"]}) => |args| -> ToolResult {
            let num_trees = args["num_trees"].as_u64().unwrap_or(10) as usize;
            let max_depth = args["max_depth"].as_u64().unwrap_or(5) as usize;
            let features: Vec<Vec<f64>> = args["features"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).collect()))
                        .collect()
                })
                .unwrap_or_default();
            if features.is_empty() {
                return Ok(serde_json::json!({"error": "empty features"}));
            }
            let (train_samples, train_labels) = crate::lulc::default_training_data(100);
            let model = crate::lulc::RandomForest::train(&train_samples, &train_labels, num_trees, max_depth);
            let results = model.predict_batch(&features);
            let predictions: Vec<serde_json::Value> = results.iter().map(|(class, probs)| {
                serde_json::json!({
                    "class_id": class,
                    "class_name": crate::lulc::LulcClass::from_usize(*class).to_string(),
                    "probabilities": probs,
                })
            }).collect();
            Ok(serde_json::json!({"predictions": predictions}))
        },
    ]);
}
