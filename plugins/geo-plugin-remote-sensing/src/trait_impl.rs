//! Plugin trait impl — RemoteSensingPlugin
use crate::config::RemoteSensingConfig;
use crate::insar::full_insar_pipeline;
use crate::radiometric::{full_radiometric_pipeline, toa_radiance};
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

pub struct RemoteSensingPlugin {
    pub config: RemoteSensingConfig,
}

impl RemoteSensingPlugin {
    pub fn new(config: RemoteSensingConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(RemoteSensingConfig::default()))
    }
}

impl Plugin for RemoteSensingPlugin {
    type Config = RemoteSensingConfig;

    fn new(config: Self::Config) -> Self {
        Self { config }
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

impl ProcessPlugin for RemoteSensingPlugin {
    fn process_type(&self) -> &str {
        "remote-sensing"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let command = params["command"].as_str().unwrap_or("");

        match command {
            "toa_radiance" => {
                let dn: Vec<Vec<f64>> =
                    serde_json::from_value(params["dn_bands"].clone()).map_err(GeoError::Serde)?;
                let gain: Vec<f64> =
                    serde_json::from_value(params["gain"].clone()).map_err(GeoError::Serde)?;
                let bias: Vec<f64> =
                    serde_json::from_value(params["bias"].clone()).map_err(GeoError::Serde)?;
                let result = toa_radiance(&dn, &gain, &bias);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "radiometric" => {
                let dn: Vec<Vec<f64>> =
                    serde_json::from_value(params["dn_bands"].clone()).map_err(GeoError::Serde)?;
                let gain: Vec<f64> =
                    serde_json::from_value(params["gain"].clone()).map_err(GeoError::Serde)?;
                let bias: Vec<f64> =
                    serde_json::from_value(params["bias"].clone()).map_err(GeoError::Serde)?;
                let sun_el = params["sun_elevation_deg"].as_f64().unwrap_or(50.0);
                let sun_dist = params["sun_earth_distance_au"].as_f64().unwrap_or(1.0);
                let dark_pct = params["dark_pct"].as_f64().unwrap_or(0.02);
                let cloud_ndvi = params["cloud_ndvi_threshold"].as_f64().unwrap_or(0.2);
                let red_idx = params["red_band_idx"].as_u64().unwrap_or(3) as usize;
                let nir_idx = params["nir_band_idx"].as_u64().unwrap_or(4) as usize;
                let result = full_radiometric_pipeline(
                    &dn,
                    &gain,
                    &bias,
                    sun_el,
                    sun_dist,
                    dark_pct,
                    cloud_ndvi,
                    red_idx,
                    nir_idx,
                );
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "insar" => {
                let master: Vec<f64> =
                    serde_json::from_value(params["master"].clone()).map_err(GeoError::Serde)?;
                let slave: Vec<f64> =
                    serde_json::from_value(params["slave"].clone()).map_err(GeoError::Serde)?;
                let window = params["window"].as_u64().unwrap_or(5) as usize;
                let cols = params["cols"].as_u64().unwrap_or(0) as usize;
                let coh_thresh = params["coherence_threshold"].as_f64().unwrap_or(0.3);
                let wavelength = params["wavelength_cm"].as_f64().unwrap_or(5.6);
                let phase_diff: Option<Vec<f64>> =
                    serde_json::from_value(params["phase_diff"].clone())
                        .map_err(GeoError::Serde)?;
                let result = full_insar_pipeline(
                    &master,
                    &slave,
                    window,
                    cols,
                    coh_thresh,
                    wavelength,
                    phase_diff.as_deref(),
                );
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            _ => Err(GeoError::Validation(format!(
                "unknown remote-sensing command: {command}"
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
        block_on(RemoteSensingPlugin::new(RemoteSensingConfig::default()).execute(cmd))
    }

    #[test]
    fn execute_radiometric_returns_pipeline_result() {
        let cmd = serde_json::json!({
            "command": "radiometric",
            "dn_bands": [[100.0, 120.0, 90.0], [200.0, 210.0, 180.0]],
            "gain": [0.1, 0.1],
            "bias": [0.0, 0.0],
            "sun_elevation_deg": 50.0,
            "sun_earth_distance_au": 1.0,
            "red_band_idx": 0,
            "nir_band_idx": 1
        });
        let out = run(cmd).expect("radiometric should succeed");
        assert_eq!(out["bands"].as_u64().unwrap(), 2);
        assert!(out["toa_radiance"].as_array().unwrap().len() == 2);
        assert!(out["toa_reflectance"].is_array());
        assert!(out["cloud_mask"].is_array());
    }

    #[test]
    fn execute_insar_returns_displacement() {
        let cmd = serde_json::json!({
            "command": "insar",
            "master": [1.0, 2.0, 3.0, 4.0],
            "slave": [1.1, 1.9, 3.2, 3.8],
            "cols": 2,
            "window": 3,
            "coherence_threshold": 0.3,
            "wavelength_cm": 5.6
        });
        let out = run(cmd).expect("insar should succeed");
        assert!(out["coherence"].as_array().unwrap().len() == 4);
        assert!(out["wrapped_phase"].as_array().unwrap().len() == 4);
        assert!(out["mean_coherence"].as_f64().unwrap() >= 0.0);
    }

    #[test]
    fn execute_unknown_command_returns_validation() {
        let cmd = serde_json::json!({ "command": "bogus" });
        let err = run(cmd).expect_err("unknown command should error");
        match err {
            GeoError::Validation(msg) => assert!(msg.contains("unknown remote-sensing command")),
            other => panic!("expected Validation, got {other:?}"),
        }
    }
}
