use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;
use serde_json::json;

use crate::drought;
use crate::gcm;
use crate::idf;
use crate::kriging;

/// Register climate plugin tools.
pub fn register_tools(registry: &mut PluginRegistry) {
    geo_registry::register_plugin!(registry, "climate",
        "Climate & meteorology: GCM downscaling, IDF curves, drought indices, Kriging interpolation",
        PluginCategory::Process, [
        sync "climate_gcm_downscale" => "Delta-method GCM statistical downscaling"
            ; json!({"type":"object","properties":{"obs":{"type":"array","items":{"type":"number"},"minItems":12,"maxItems":12},"hist":{"type":"array","items":{"type":"number"},"minItems":12,"maxItems":12},"proj":{"type":"array","items":{"type":"number"},"minItems":12,"maxItems":12},"variable":{"type":"string","enum":["tas","pr"]}},"required":["obs","hist","proj"]})
            => |args| -> ToolResult {
                let obs: [f64; 12] = serde_json::from_value(args["obs"].clone())?;
                let hist: [f64; 12] = serde_json::from_value(args["hist"].clone())?;
                let proj: [f64; 12] = serde_json::from_value(args["proj"].clone())?;
                let variable = args["variable"].as_str().unwrap_or("tas");
                let result = gcm::delta_downscale(&obs, &hist, &proj, variable);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_quantile_mapping" => "Quantile mapping bias correction"
            ; json!({"type":"object","properties":{"obs":{"type":"array","items":{"type":"number"}},"hist":{"type":"array","items":{"type":"number"}},"proj":{"type":"array","items":{"type":"number"}}},"required":["obs","hist","proj"]})
            => |args| -> ToolResult {
                let obs: Vec<f64> = serde_json::from_value(args["obs"].clone())?;
                let hist: Vec<f64> = serde_json::from_value(args["hist"].clone())?;
                let proj: Vec<f64> = serde_json::from_value(args["proj"].clone())?;
                let result = gcm::quantile_mapping(&obs, &hist, &proj);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_idf_curve" => "IDF rainfall curve from Sherman parameters"
            ; json!({"type":"object","properties":{"durations":{"type":"array","items":{"type":"number"}},"params":{"type":"object","properties":{"a":{"type":"number"},"b":{"type":"number"},"c":{"type":"number"}}}},"required":["durations","params"]})
            => |args| -> ToolResult {
                let durations: Vec<f64> = serde_json::from_value(args["durations"].clone())?;
                let params: idf::IdfParams = serde_json::from_value(args["params"].clone())?;
                let result = idf::idf_curve(&durations, &params);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_idf_fit" => "Fit IDF Sherman parameters from duration-intensity data"
            ; json!({"type":"object","properties":{"durations":{"type":"array","items":{"type":"number"}},"intensities":{"type":"array","items":{"type":"number"}}},"required":["durations","intensities"]})
            => |args| -> ToolResult {
                let durations: Vec<f64> = serde_json::from_value(args["durations"].clone())?;
                let intensities: Vec<f64> = serde_json::from_value(args["intensities"].clone())?;
                let result = idf::idf_fit_params(&durations, &intensities);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_idf_return_period" => "Scale IDF Sherman parameters to a new return period"
            ; json!({"type":"object","properties":{"params":{"type":"object","properties":{"a":{"type":"number"},"b":{"type":"number"},"c":{"type":"number"}}},"base_return_yr":{"type":"number","default":2.0},"target_return_yr":{"type":"number","default":100.0},"coef_a":{"type":"number","default":0.0},"coef_b":{"type":"number","default":0.0}},"required":["params"]})
            => |args| -> ToolResult {
                let base_params: idf::IdfParams = serde_json::from_value(args["params"].clone())?;
                let base_return_yr = args["base_return_yr"].as_f64().unwrap_or(2.0);
                let target_return_yr = args["target_return_yr"].as_f64().unwrap_or(100.0);
                let coef_a = args["coef_a"].as_f64().unwrap_or(0.0);
                let coef_b = args["coef_b"].as_f64().unwrap_or(0.0);
                let result = idf::idf_return_period(&base_params, base_return_yr, target_return_yr, coef_a, coef_b);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_spi" => "Standardized Precipitation Index"
            ; json!({"type":"object","properties":{"precip":{"type":"array","items":{"type":"number"}},"scale_months":{"type":"integer","default":3}},"required":["precip"]})
            => |args| -> ToolResult {
                let precip: Vec<f64> = serde_json::from_value(args["precip"].clone())?;
                let scale = args["scale_months"].as_u64().unwrap_or(3) as usize;
                let result = drought::compute_spi(&precip, scale);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_spei" => "Standardized Precipitation-Evapotranspiration Index"
            ; json!({"type":"object","properties":{"precip":{"type":"array","items":{"type":"number"}},"temp":{"type":"array","items":{"type":"number"}},"scale_months":{"type":"integer","default":3}},"required":["precip","temp"]})
            => |args| -> ToolResult {
                let precip: Vec<f64> = serde_json::from_value(args["precip"].clone())?;
                let temp: Vec<f64> = serde_json::from_value(args["temp"].clone())?;
                let scale = args["scale_months"].as_u64().unwrap_or(3) as usize;
                let result = drought::compute_spei(&precip, &temp, scale);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_pdsi" => "Palmer Drought Severity Index"
            ; json!({"type":"object","properties":{"temp":{"type":"array","items":{"type":"number"}},"precip":{"type":"array","items":{"type":"number"}},"lat":{"type":"number","default":30.0},"awc_mm":{"type":"number","default":150.0}},"required":["temp","precip"]})
            => |args| -> ToolResult {
                let temp: Vec<f64> = serde_json::from_value(args["temp"].clone())?;
                let precip: Vec<f64> = serde_json::from_value(args["precip"].clone())?;
                let lat = args["lat"].as_f64().unwrap_or(30.0);
                let awc = args["awc_mm"].as_f64().unwrap_or(150.0);
                let result = drought::compute_pdsi(&temp, &precip, lat, awc);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_kriging" => "Ordinary Kriging spatial interpolation"
            ; json!({"type":"object","properties":{"points":{"type":"array","items":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"number"}}},"bbox":{"type":"object","properties":{"min_x":{"type":"number"},"min_y":{"type":"number"},"max_x":{"type":"number"},"max_y":{"type":"number"}}},"cell_size":{"type":"number","default":1.0},"variogram":{"type":"object"}},"required":["points","bbox","variogram"]})
            => |args| -> ToolResult {
                let points: Vec<(f64, f64, f64)> = serde_json::from_value(args["points"].clone())?;
                let bbox: geo_core::types::BBox = serde_json::from_value(args["bbox"].clone())?;
                let cell_size = args["cell_size"].as_f64().unwrap_or(1.0);
                let variogram: kriging::VariogramParams = serde_json::from_value(args["variogram"].clone())?;
                let result = kriging::ordinary_kriging(&points, &bbox, cell_size, &variogram);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_simple_kriging" => "Simple Kriging spatial interpolation (known mean 0)"
            ; json!({"type":"object","properties":{"points":{"type":"array","items":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"number"}}},"bbox":{"type":"object","properties":{"min_x":{"type":"number"},"min_y":{"type":"number"},"max_x":{"type":"number"},"max_y":{"type":"number"}}},"cell_size":{"type":"number","default":1.0},"variogram":{"type":"object"}},"required":["points","bbox","variogram"]})
            => |args| -> ToolResult {
                let points: Vec<(f64, f64, f64)> = serde_json::from_value(args["points"].clone())?;
                let bbox: geo_core::types::BBox = serde_json::from_value(args["bbox"].clone())?;
                let cell_size = args["cell_size"].as_f64().unwrap_or(1.0);
                let variogram: kriging::VariogramParams = serde_json::from_value(args["variogram"].clone())?;
                let result = kriging::simple_kriging(&points, &bbox, cell_size, &variogram);
                Ok(serde_json::to_value(result)?)
            },
        sync "climate_variogram" => "Empirical semivariogram from point data"
            ; json!({"type":"object","properties":{"points":{"type":"array","items":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"number"}}},"num_bins":{"type":"integer","default":10}},"required":["points"]})
            => |args| -> ToolResult {
                let points: Vec<(f64, f64, f64)> = serde_json::from_value(args["points"].clone())?;
                let num_bins = args["num_bins"].as_u64().unwrap_or(10) as usize;
                let (d, s) = kriging::semivariogram(&points, num_bins);
                Ok(json!({"distances": d, "semivariances": s}))
            },
    ]);
}
