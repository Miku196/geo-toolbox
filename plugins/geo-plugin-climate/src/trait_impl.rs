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
            "idf_return_period" => {
                let base_params: idf::IdfParams =
                    serde_json::from_value(params["params"].clone()).map_err(GeoError::Serde)?;
                let base_return_yr = params["base_return_yr"].as_f64().unwrap_or(2.0);
                let target_return_yr = params["target_return_yr"].as_f64().unwrap_or(100.0);
                let coef_a = params["coef_a"].as_f64().unwrap_or(0.0);
                let coef_b = params["coef_b"].as_f64().unwrap_or(0.0);
                let result = idf::idf_return_period(
                    &base_params,
                    base_return_yr,
                    target_return_yr,
                    coef_a,
                    coef_b,
                );
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "simple_kriging" => {
                // simple_kriging shares its signature with ordinary_kriging:
                // points / bbox / cell_size / variogram. Kept as a sugar alias.
                let points: Vec<(f64, f64, f64)> =
                    serde_json::from_value(params["points"].clone()).map_err(GeoError::Serde)?;
                let bbox: geo_core::types::BBox =
                    serde_json::from_value(params["bbox"].clone()).map_err(GeoError::Serde)?;
                let cell_size = params["cell_size"].as_f64().unwrap_or(1.0);
                let variogram: kriging::VariogramParams =
                    serde_json::from_value(params["variogram"].clone()).map_err(GeoError::Serde)?;
                let result = kriging::simple_kriging(&points, &bbox, cell_size, &variogram);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            _ => Err(GeoError::Validation(format!(
                "unknown climate command: {command}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Wake, Waker};

    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: std::sync::Arc<Self>) {}
    }

    /// Drive a future to completion on a minimal no-op executor.
    fn block_on<F: Future>(mut fut: F) -> F::Output {
        let waker = Waker::from(std::sync::Arc::new(NoopWaker));
        let mut cx = Context::from_waker(&waker);
        let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    fn run(cmd: serde_json::Value) -> GeoResult<serde_json::Value> {
        block_on(ClimatePlugin.execute(cmd))
    }

    fn variogram_json() -> serde_json::Value {
        serde_json::json!({
            "model": {
                "Spherical": { "range": 15.0, "sill": 500.0, "nugget": 0.0 }
            }
        })
    }

    #[test]
    fn execute_simple_kriging_returns_result_grid() {
        let cmd = serde_json::json!({
            "command": "simple_kriging",
            "points": [
                [0.0, 0.0, 100.0],
                [10.0, 0.0, 50.0],
                [5.0, 10.0, 75.0]
            ],
            "bbox": { "min_x": 0.0, "min_y": 0.0, "max_x": 10.0, "max_y": 10.0 },
            "cell_size": 5.0,
            "variogram": variogram_json()
        });
        let out = run(cmd).expect("simple_kriging should succeed");
        assert!(out["grid_rows"].as_u64().unwrap() > 0);
        assert!(out["grid_cols"].as_u64().unwrap() > 0);
        assert!(out["predictions"].is_array());
        assert!(out["variances"].is_array());
    }

    #[test]
    fn execute_idf_return_period_returns_ok() {
        let cmd = serde_json::json!({
            "command": "idf_return_period",
            "params": { "a": 1000.0, "b": 10.0, "c": 0.8 },
            "base_return_yr": 2.0,
            "target_return_yr": 100.0,
            "coef_a": 0.5,
            "coef_b": 0.2
        });
        let out = run(cmd).expect("idf_return_period should succeed");
        assert!(out["a"].as_f64().unwrap() > 0.0);
        assert_eq!(out["b"].as_f64().unwrap(), 10.0);
        assert_eq!(out["c"].as_f64().unwrap(), 0.8);
    }

    #[test]
    fn execute_unknown_command_returns_validation_not_unimplemented() {
        let cmd = serde_json::json!({ "command": "unknown_xyz" });
        let err = run(cmd).expect_err("unknown command should error");
        match err {
            GeoError::Validation(msg) => {
                assert!(msg.contains("unknown climate command"));
                assert!(msg.contains("unknown_xyz"));
            }
            other => panic!("expected Validation variant, got {other:?}"),
        }
    }
}
