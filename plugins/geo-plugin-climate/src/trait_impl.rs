use crate::config::ClimateConfig;
use crate::{drought, gcm, idf, kriging};
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

/// Climate plugin implementing Plugin + ProcessPlugin.
pub struct ClimatePlugin;

impl Plugin for ClimatePlugin {
    type Config = ClimateConfig;

    fn new(_config: ClimateConfig) -> Self {
        Self
    }

    fn name(&self) -> &str {
        "climate"
    }

    fn version(&self) -> &str {
        "0.1"
    }

    fn description(&self) -> &str {
        "Climate & meteorology: GCM downscaling, IDF curves, drought indices, Kriging interpolation"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}

impl ProcessPlugin for ClimatePlugin {
    fn process_type(&self) -> &str {
        "climate"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let command = params["command"].as_str().unwrap_or("");

        match command {
            "delta_downscale" => {
                let obs: [f64; 12] =
                    serde_json::from_value(params["obs"].clone()).map_err(GeoError::Serde)?;
                let hist: [f64; 12] =
                    serde_json::from_value(params["hist"].clone()).map_err(GeoError::Serde)?;
                let proj: [f64; 12] =
                    serde_json::from_value(params["proj"].clone()).map_err(GeoError::Serde)?;
                let variable = params["variable"].as_str().unwrap_or("tas");
                let result = gcm::delta_downscale(&obs, &hist, &proj, variable);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "quantile_mapping" => {
                let obs: Vec<f64> =
                    serde_json::from_value(params["obs"].clone()).map_err(GeoError::Serde)?;
                let hist: Vec<f64> =
                    serde_json::from_value(params["hist"].clone()).map_err(GeoError::Serde)?;
                let proj: Vec<f64> =
                    serde_json::from_value(params["proj"].clone()).map_err(GeoError::Serde)?;
                let result = gcm::quantile_mapping(&obs, &hist, &proj);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "idf_curve" => {
                let durations: Vec<f64> =
                    serde_json::from_value(params["durations"].clone()).map_err(GeoError::Serde)?;
                let p: idf::IdfParams =
                    serde_json::from_value(params["params"].clone()).map_err(GeoError::Serde)?;
                let result = idf::idf_curve(&durations, &p);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "idf_fit" => {
                let durations: Vec<f64> =
                    serde_json::from_value(params["durations"].clone()).map_err(GeoError::Serde)?;
                let intensities: Vec<f64> = serde_json::from_value(params["intensities"].clone())
                    .map_err(GeoError::Serde)?;
                let result = idf::idf_fit_params(&durations, &intensities);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "spi" => {
                let precip: Vec<f64> =
                    serde_json::from_value(params["precip"].clone()).map_err(GeoError::Serde)?;
                let scale = params["scale_months"].as_u64().unwrap_or(3) as usize;
                let result = drought::compute_spi(&precip, scale);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "spei" => {
                let precip: Vec<f64> =
                    serde_json::from_value(params["precip"].clone()).map_err(GeoError::Serde)?;
                let temp: Vec<f64> =
                    serde_json::from_value(params["temp"].clone()).map_err(GeoError::Serde)?;
                let scale = params["scale_months"].as_u64().unwrap_or(3) as usize;
                let result = drought::compute_spei(&precip, &temp, scale);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "pdsi" => {
                let temp: Vec<f64> =
                    serde_json::from_value(params["temp"].clone()).map_err(GeoError::Serde)?;
                let precip: Vec<f64> =
                    serde_json::from_value(params["precip"].clone()).map_err(GeoError::Serde)?;
                let lat = params["lat"].as_f64().unwrap_or(30.0);
                let awc = params["awc_mm"].as_f64().unwrap_or(150.0);
                let result = drought::compute_pdsi(&temp, &precip, lat, awc);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "ordinary_kriging" => {
                let points: Vec<(f64, f64, f64)> =
                    serde_json::from_value(params["points"].clone()).map_err(GeoError::Serde)?;
                let bbox: geo_core::types::BBox =
                    serde_json::from_value(params["bbox"].clone()).map_err(GeoError::Serde)?;
                let cell_size = params["cell_size"].as_f64().unwrap_or(1.0);
                let variogram: kriging::VariogramParams =
                    serde_json::from_value(params["variogram"].clone()).map_err(GeoError::Serde)?;
                let result = kriging::ordinary_kriging(&points, &bbox, cell_size, &variogram);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "semivariogram_build" => {
                let points: Vec<(f64, f64, f64)> =
                    serde_json::from_value(params["points"].clone()).map_err(GeoError::Serde)?;
                let num_bins = params["num_bins"].as_u64().unwrap_or(10) as usize;
                let (d, s) = kriging::semivariogram(&points, num_bins);
                Ok(serde_json::json!({"distances": d, "semivariances": s}))
            }
            _ => Err(GeoError::Unimplemented(format!(
                "unknown climate command: {command}"
            ))),
        }
    }
}
