//! Plugin trait impl — SocioeconomicPlugin
use crate::accessibility::multi_city_accessibility;
use crate::config::SocioeconomicConfig;
use crate::landuse_change::transition_probability;
use crate::population::full_population_pipeline;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

pub struct SocioeconomicPlugin {
    pub config: SocioeconomicConfig,
}

impl SocioeconomicPlugin {
    pub fn new(config: SocioeconomicConfig) -> Self {
        Self { config }
    }

    pub fn load(_path: &std::path::Path) -> GeoResult<Self> {
        Ok(Self::new(SocioeconomicConfig::default()))
    }
}

impl Plugin for SocioeconomicPlugin {
    type Config = SocioeconomicConfig;

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

impl ProcessPlugin for SocioeconomicPlugin {
    fn process_type(&self) -> &str {
        "socioeconomic"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let command = params["command"].as_str().unwrap_or("");

        match command {
            "population" => {
                let admin_pop = params["admin_pop"].as_f64().unwrap_or(0.0);
                let landcover: Vec<f64> = serde_json::from_value(params["landcover_weights"].clone())
                    .map_err(GeoError::Serde)?;
                let cell_area = params["cell_area_km2"].as_f64().unwrap_or(0.01);
                let ntl: Option<Vec<f64>> =
                    serde_json::from_value(params["ntl"].clone()).map_err(GeoError::Serde)?;
                let calibration = params["calibration"].as_f64().unwrap_or(0.5);
                let building: Option<Vec<f64>> = serde_json::from_value(
                    params["building_density"].clone(),
                )
                .map_err(GeoError::Serde)?;
                let road: Option<Vec<f64>> =
                    serde_json::from_value(params["road_density"].clone())
                        .map_err(GeoError::Serde)?;
                let result = full_population_pipeline(
                    admin_pop,
                    &landcover,
                    cell_area,
                    ntl.as_deref(),
                    calibration,
                    building.as_deref(),
                    road.as_deref(),
                );
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "transition_matrix" => {
                let from: Vec<u8> = serde_json::from_value(params["from_lulc"].clone())
                    .map_err(GeoError::Serde)?;
                let to: Vec<u8> =
                    serde_json::from_value(params["to_lulc"].clone()).map_err(GeoError::Serde)?;
                let n_classes = params["n_classes"].as_u64().unwrap_or(3) as u8;
                let result = transition_probability(&from, &to, n_classes);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            "accessibility" => {
                let origins: Vec<usize> =
                    serde_json::from_value(params["origins"].clone()).map_err(GeoError::Serde)?;
                let cost: Vec<f64> = serde_json::from_value(params["cost_surface"].clone())
                    .map_err(GeoError::Serde)?;
                let max_cost = params["max_cost"].as_f64().unwrap_or(120.0);
                let cols = params["cols"].as_u64().unwrap_or(1) as usize;
                let decay = params["decay"].as_f64().unwrap_or(0.05);
                let city_pop: Vec<f64> = serde_json::from_value(params["city_populations"].clone())
                    .map_err(GeoError::Serde)?;
                let result =
                    multi_city_accessibility(&origins, &cost, max_cost, cols, decay, &city_pop);
                serde_json::to_value(result).map_err(GeoError::Serde)
            }
            _ => Err(GeoError::Validation(format!(
                "unknown socioeconomic command: {command}"
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
        block_on(SocioeconomicPlugin::new(SocioeconomicConfig::default()).execute(cmd))
    }

    #[test]
    fn execute_population_returns_grid() {
        let cmd = serde_json::json!({
            "command": "population",
            "admin_pop": 3000.0,
            "landcover_weights": [1.0, 1.0, 1.0],
            "cell_area_km2": 0.01,
            "ntl": [10.0, 20.0, 30.0],
            "calibration": 0.5
        });
        let out = run(cmd).expect("population should succeed");
        assert_eq!(out["population_grid"].as_array().unwrap().len(), 3);
        assert!((out["total_population"].as_f64().unwrap() - 3000.0).abs() < 1e-6);
        assert!(out["gdp_grid"].is_array());
        assert!(out["total_gdp"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn execute_transition_matrix_returns_rows() {
        let cmd = serde_json::json!({
            "command": "transition_matrix",
            "from_lulc": [0, 0, 1, 1, 2],
            "to_lulc": [0, 1, 1, 2, 2],
            "n_classes": 3
        });
        let out = run(cmd).expect("transition_matrix should succeed");
        let rows = out.as_array().unwrap();
        assert_eq!(rows.len(), 3);
        for row in rows {
            assert_eq!(row.as_array().unwrap().len(), 3);
        }
    }

    #[test]
    fn execute_unknown_command_returns_validation() {
        let cmd = serde_json::json!({ "command": "huh" });
        let err = run(cmd).expect_err("unknown command should error");
        match err {
            GeoError::Validation(msg) => assert!(msg.contains("unknown socioeconomic command")),
            other => panic!("expected Validation, got {other:?}"),
        }
    }
}
