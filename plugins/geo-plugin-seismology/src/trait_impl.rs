use crate::config::SeismologyConfig;
use crate::ground_motion::{ground_motion_assessment, response_spectrum};
use crate::psha::{psha_hazard_curve, SeismicSource};
use crate::seismicity::seismicity_analysis;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

pub struct SeismologyPlugin {
    pub config: SeismologyConfig,
}

impl SeismologyPlugin {
    pub fn new(config: SeismologyConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(SeismologyConfig::default()))
    }
}

impl Plugin for SeismologyPlugin {
    type Config = SeismologyConfig;

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

impl ProcessPlugin for SeismologyPlugin {
    fn process_type(&self) -> &str {
        "seismology"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let command = params["command"].as_str().unwrap_or("");

        match command {
            "pga" => {
                let magnitude = params["magnitude"].as_f64().unwrap_or(6.0);
                let distance_km = params["distance_km"].as_f64().unwrap_or(30.0);
                let site_class = params["site_class"].as_str().unwrap_or("II");
                let result = ground_motion_assessment(magnitude, distance_km, site_class);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "response_spectrum" => {
                let pga_g = params["pga_g"].as_f64().unwrap_or(0.1);
                let periods: Vec<f64> =
                    serde_json::from_value(params["periods"].clone()).map_err(GeoError::Serde)?;
                let damping = params["damping"].as_f64().unwrap_or(0.05);
                let result = response_spectrum(pga_g, &periods, damping);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "seismic_hazard" => {
                let sources: Vec<SeismicSource> =
                    serde_json::from_value(params["sources"].clone()).map_err(GeoError::Serde)?;
                let site_lon = params["site_lon"].as_f64().unwrap_or(0.0);
                let site_lat = params["site_lat"].as_f64().unwrap_or(0.0);
                let return_periods: Vec<f64> = serde_json::from_value(
                    params["return_periods"].clone(),
                )
                .map_err(GeoError::Serde)?;
                let site_class = params["site_class"].as_str().unwrap_or("II");
                let result =
                    psha_hazard_curve(&sources, site_lon, site_lat, &return_periods, site_class);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "seismicity" => {
                let magnitudes: Vec<f64> =
                    serde_json::from_value(params["magnitudes"].clone()).map_err(GeoError::Serde)?;
                let min_mag = params["min_mag"].as_f64().unwrap_or(3.0);
                let time_span = params["time_span_years"].as_f64().unwrap_or(50.0);
                let result = seismicity_analysis(&magnitudes, min_mag, time_span);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            _ => Err(GeoError::Validation(format!(
                "unknown seismology command: {command}"
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
        block_on(SeismologyPlugin::new(SeismologyConfig::default()).execute(cmd))
    }

    #[test]
    fn execute_pga_returns_ground_motion() {
        let cmd = serde_json::json!({
            "command": "pga",
            "magnitude": 7.0,
            "distance_km": 30.0,
            "site_class": "II"
        });
        let out = run(cmd).expect("pga should succeed");
        assert!(out["pga_g"].as_f64().unwrap() > 0.0);
        assert!(out["pgv_cm_s"].as_f64().unwrap() > 0.0);
        assert!(out["intensity"].as_u64().unwrap() >= 5);
        assert_eq!(out["magnitude"].as_f64().unwrap(), 7.0);
    }

    #[test]
    fn execute_seismicity_returns_gr_analysis() {
        let cmd = serde_json::json!({
            "command": "seismicity",
            "magnitudes": [3.1, 3.5, 4.0, 4.2, 5.0],
            "min_mag": 3.0,
            "time_span_years": 50.0
        });
        let out = run(cmd).expect("seismicity should succeed");
        assert!(out["event_count"].as_u64().unwrap() >= 5);
        assert!(out["b_value_mle"].as_f64().unwrap() > 0.0);
        assert!(out["annual_rate"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn execute_unknown_command_returns_validation() {
        let cmd = serde_json::json!({ "command": "wat" });
        let err = run(cmd).expect_err("unknown command should error");
        match err {
            GeoError::Validation(msg) => assert!(msg.contains("unknown seismology command")),
            other => panic!("expected Validation, got {other:?}"),
        }
    }
}
