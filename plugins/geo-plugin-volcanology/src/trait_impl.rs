use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

use crate::ash_dispersion::ash_dispersion_assessment;
use crate::config::VolcanologyConfig;
use crate::hazard_zoning::{hazard_zone_classification, volcanic_hazard_zoning};
use crate::lava_flow::lava_flow_path;

pub struct VolcanologyPlugin {
    pub config: VolcanologyConfig,
}

impl VolcanologyPlugin {
    pub fn new(config: VolcanologyConfig) -> Self {
        Self { config }
    }
    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(VolcanologyConfig::default()))
    }
}

impl Plugin for VolcanologyPlugin {
    type Config = VolcanologyConfig;
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

impl ProcessPlugin for VolcanologyPlugin {
    fn process_type(&self) -> &str {
        "volcanology"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let command = params["command"].as_str().unwrap_or("");

        match command {
            "ash_dispersion" => {
                let emission = params["emission_rate_kg_s"].as_f64().unwrap_or(1000.0);
                let wind = params["wind_speed_m_s"].as_f64().unwrap_or(10.0);
                let plume_h = params["plume_height_m"].as_f64().unwrap_or(5000.0);
                // Accept particle diameter in millimetres (matches the tool schema) and
                // convert to metres for the Stokes settling calculation.
                let diameter_mm = params["particle_diameter_mm"].as_f64().unwrap_or(0.5);
                let diameter_m = diameter_mm / 1000.0;
                let density = params["particle_density_kgm3"].as_f64().unwrap_or(2500.0);
                let stability = params["stability"].as_str().unwrap_or("D");
                let n_points = params["n_points"].as_u64().unwrap_or(20) as usize;
                let result = ash_dispersion_assessment(
                    emission, wind, plume_h, diameter_m, density, stability, n_points,
                );
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "hazard_zone" => {
                let ash_grid: Vec<f64> =
                    serde_json::from_value(params["ash_grid"].clone()).map_err(GeoError::Serde)?;
                let lava_grid: Vec<u8> =
                    serde_json::from_value(params["lava_grid"].clone()).map_err(GeoError::Serde)?;
                let dist_grid: Vec<f64> =
                    serde_json::from_value(params["dist_grid"].clone()).map_err(GeoError::Serde)?;
                let slope_grid: Vec<f64> = serde_json::from_value(params["slope_grid"].clone())
                    .map_err(GeoError::Serde)?;
                let n = params["n"].as_u64().unwrap_or(ash_grid.len() as u64) as usize;
                let src_row = params["source_row"].as_u64().unwrap_or(0) as usize;
                let src_col = params["source_col"].as_u64().unwrap_or(0) as usize;
                let result = volcanic_hazard_zoning(
                    &ash_grid,
                    &lava_grid,
                    &dist_grid,
                    &slope_grid,
                    n,
                    src_row,
                    src_col,
                );
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "hazard_classification" => {
                let ash = params["ash_thickness_mm"].as_f64().unwrap_or(0.0);
                let on_lava = params["on_lava_path"].as_bool().unwrap_or(false);
                let dist = params["distance_km"].as_f64().unwrap_or(10.0);
                let level = hazard_zone_classification(ash, on_lava, dist);
                serde_json::to_value(level).map_err(GeoError::Serde)
            }
            "lava_flow" => {
                let dem: Vec<f64> =
                    serde_json::from_value(params["dem"].clone()).map_err(GeoError::Serde)?;
                let vent_row = params["vent_row"].as_u64().unwrap_or(0) as usize;
                let vent_col = params["vent_col"].as_u64().unwrap_or(0) as usize;
                let effusion = params["effusion_rate_m3s"].as_f64().unwrap_or(500.0);
                let viscosity = params["viscosity_Pa_s"].as_f64().unwrap_or(5000.0);
                let rows = params["rows"].as_u64().unwrap_or(0) as usize;
                let cols = params["cols"].as_u64().unwrap_or(0) as usize;
                let result =
                    lava_flow_path(&dem, vent_row, vent_col, effusion, viscosity, rows, cols);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            _ => Err(GeoError::Validation(format!(
                "unknown volcanology command: {command}"
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
        block_on(VolcanologyPlugin::new(VolcanologyConfig::default()).execute(cmd))
    }

    #[test]
    fn execute_ash_dispersion_returns_profile() {
        let cmd = serde_json::json!({
            "command": "ash_dispersion",
            "emission_rate_kg_s": 1000.0,
            "wind_speed_m_s": 10.0,
            "plume_height_m": 5000.0,
            "particle_diameter_mm": 0.5,
            "particle_density_kgm3": 2500.0,
            "stability": "D",
            "n_points": 20
        });
        let out = run(cmd).expect("ash_dispersion should succeed");
        assert!(out["downwind_distances_km"].as_array().unwrap().len() == 20);
        assert!(out["plume_height_m"].as_f64().unwrap() > 0.0);
        assert!(out["total_emission_kg"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn execute_hazard_classification_returns_level() {
        let cmd = serde_json::json!({
            "command": "hazard_classification",
            "ash_thickness_mm": 200.0,
            "on_lava_path": true,
            "distance_km": 0.5
        });
        let out = run(cmd).expect("hazard_classification should succeed");
        assert!(out.is_string());
    }

    #[test]
    fn execute_unknown_command_returns_validation() {
        let cmd = serde_json::json!({ "command": "n/a" });
        let err = run(cmd).expect_err("unknown command should error");
        match err {
            GeoError::Validation(msg) => assert!(msg.contains("unknown volcanology command")),
            other => panic!("expected Validation, got {other:?}"),
        }
    }
}
