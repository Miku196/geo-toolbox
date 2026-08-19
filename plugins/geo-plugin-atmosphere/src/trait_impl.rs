use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

use crate::boundary_layer::StabilityClass;
use crate::config::AtmosphereConfig;
use crate::{aod_pm25_pipeline, boundary_layer_assessment, dispersion_assessment};

pub struct AtmospherePlugin {
    pub config: AtmosphereConfig,
}

impl AtmospherePlugin {
    pub fn new(config: AtmosphereConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(AtmosphereConfig::default()))
    }
}

impl Plugin for AtmospherePlugin {
    type Config = AtmosphereConfig;

    fn new(config: Self::Config) -> Self {
        Self::new(config)
    }

    fn name(&self) -> &str {
        &self.config.plugin.name
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        &self.config.plugin.description
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}

impl ProcessPlugin for AtmospherePlugin {
    fn process_type(&self) -> &str {
        "atmosphere"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let command = params["command"].as_str().unwrap_or("");

        match command {
            "aod_pm25" => {
                let aod_values: Vec<f64> = serde_json::from_value(params["aod_values"].clone())
                    .map_err(GeoError::Serde)?;
                let ratio = params["aod550_pm25_ratio"]
                    .as_f64()
                    .unwrap_or(self.config.aod550_pm25_ratio);
                let rh = params["rh_correction"]
                    .as_f64()
                    .unwrap_or(self.config.rh_correction_factor);
                let season = params["season"].as_str().unwrap_or("annual");
                let result = aod_pm25_pipeline(&aod_values, ratio, rh, season);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "boundary_layer" => {
                let temp_profile: Vec<f64> = serde_json::from_value(params["temp_profile"].clone())
                    .map_err(GeoError::Serde)?;
                let wind_profile: Vec<f64> = serde_json::from_value(params["wind_profile"].clone())
                    .map_err(GeoError::Serde)?;
                let roughness = params["roughness_m"]
                    .as_f64()
                    .unwrap_or(self.config.roughness_default);
                let coriolis = params["coriolis_param"]
                    .as_f64()
                    .unwrap_or(self.config.coriolis_default);
                let result =
                    boundary_layer_assessment(&temp_profile, &wind_profile, roughness, coriolis);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "dispersion" => {
                let emission = params["emission_rate_g_s"].as_f64().unwrap_or(100.0);
                let wind = params["wind_speed_m_s"].as_f64().unwrap_or(5.0);
                let stability = params["stability"]
                    .as_str()
                    .and_then(|s| s.chars().next())
                    .and_then(StabilityClass::from_char)
                    .unwrap_or(StabilityClass::D);
                let src_h = params["source_height_m"].as_f64().unwrap_or(10.0);
                let result = dispersion_assessment(emission, wind, stability, src_h);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            _ => Err(GeoError::Validation(format!(
                "unknown atmosphere command: {command}"
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
        block_on(AtmospherePlugin::new(AtmosphereConfig::default()).execute(cmd))
    }

    #[test]
    fn execute_boundary_layer_returns_assessment() {
        let cmd = serde_json::json!({
            "command": "boundary_layer",
            "temp_profile": [20.0, 18.0, 16.0, 14.0],
            "wind_profile": [2.0, 5.0, 8.0, 10.0],
            "roughness_m": 0.1,
            "coriolis_param": 1.0e-4
        });
        let out = run(cmd).expect("boundary_layer should succeed");
        assert!(out["abl_height_m"].as_f64().unwrap() > 0.0);
        assert!(out["u_star_m_s"].as_f64().unwrap() >= 0.0);
        assert!(out["stability"].is_string());
    }

    #[test]
    fn execute_dispersion_returns_plume_summary() {
        let cmd = serde_json::json!({
            "command": "dispersion",
            "emission_rate_g_s": 200.0,
            "wind_speed_m_s": 5.0,
            "stability": "D",
            "source_height_m": 20.0
        });
        let out = run(cmd).expect("dispersion should succeed");
        assert!(out["plume"]["max_ground_conc_ug_m3"].as_f64().unwrap() > 0.0);
        assert!(out["centerline"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn execute_unknown_command_returns_validation() {
        let cmd = serde_json::json!({ "command": "nope" });
        let err = run(cmd).expect_err("unknown command should error");
        match err {
            GeoError::Validation(msg) => assert!(msg.contains("unknown atmosphere command")),
            other => panic!("expected Validation, got {other:?}"),
        }
    }
}
