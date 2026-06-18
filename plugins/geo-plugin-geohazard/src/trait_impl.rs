use crate::config::GeohazardConfig;
use crate::GeohazardPlugin;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};

impl GeohazardPlugin {
    /// Load plugin from rules.toml file.
    pub fn load_from_file(path: &std::path::Path) -> GeoResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| GeoError::Validation(format!("Failed to read {}: {e}", path.display())))?;
        let config = toml::from_str(&content).map_err(|e| {
            GeoError::Validation(format!("Failed to parse {}: {e}", path.display()))
        })?;
        Ok(Self::new(config))
    }
}

impl Plugin for GeohazardPlugin {
    type Config = GeohazardConfig;

    fn new(config: GeohazardConfig) -> Self {
        Self::new(config)
    }

    fn name(&self) -> &str {
        "geohazard"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "地质灾害插件 — 滑坡敏感性、泥石流危险性、综合风险评级"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }

    fn is_healthy(&self) -> bool {
        true
    }
}

impl ProcessPlugin for GeohazardPlugin {
    fn process_type(&self) -> &str {
        "geohazard"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let task = params
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("risk_map");

        match task {
            "landslide" => {
                let slope_deg = params["slope_deg"].as_f64().unwrap_or(15.0);
                let aspect_deg = params["aspect_deg"].as_f64().unwrap_or(180.0);
                let lithology = params["lithology_index"].as_f64().unwrap_or(0.5);
                let rainfall = params["rainfall_mm"].as_f64().unwrap_or(100.0);
                let fault = params["fault_distance_m"].as_f64().unwrap_or(500.0);
                let ndvi = params["ndvi"].as_f64().unwrap_or(0.3);

                let result = self.landslide_susceptibility(
                    slope_deg, aspect_deg, lithology, rainfall, fault, ndvi,
                );
                Ok(serde_json::to_value(&result).map_err(GeoError::Serde)?)
            }

            "debris_flow" => {
                let gradient = params["channel_gradient_deg"].as_f64().unwrap_or(10.0);
                let material = params["material_volume_per_km"].as_f64().unwrap_or(500.0);
                let rainfall = params["rainfall_24h_mm"].as_f64().unwrap_or(30.0);

                let result = self.debris_flow_hazard(gradient, material, rainfall);
                Ok(serde_json::to_value(&result).map_err(GeoError::Serde)?)
            }

            "debris_flow_runout" => {
                let area = params["watershed_area_km2"].as_f64().unwrap_or(1.0);
                let rainfall = params["rainfall_24h_mm"].as_f64().unwrap_or(100.0);
                let elevation = params["elevation_drop_m"].as_f64().unwrap_or(100.0);
                let gradient = params["channel_gradient_deg"].as_f64().unwrap_or(20.0);

                let result =
                    self.debris_flow_runout_assessment(area, rainfall, elevation, gradient)?;
                Ok(serde_json::to_value(&result).map_err(GeoError::Serde)?)
            }

            _ => {
                // Default: combined risk assessment
                let aoi_name = params
                    .get("aoi_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();

                let slope_deg = params["slope_deg"].as_f64().unwrap_or(15.0);
                let aspect_deg = params["aspect_deg"].as_f64().unwrap_or(180.0);
                let lithology = params["lithology_index"].as_f64().unwrap_or(0.5);
                let rainfall = params["rainfall_mm"].as_f64().unwrap_or(100.0);
                let fault = params["fault_distance_m"].as_f64().unwrap_or(500.0);
                let ndvi = params["ndvi"].as_f64().unwrap_or(0.3);

                let ls = self.landslide_susceptibility(
                    slope_deg, aspect_deg, lithology, rainfall, fault, ndvi,
                );

                let df = if params
                    .get("channel_gradient_deg")
                    .and_then(|v| v.as_f64())
                    .is_some()
                {
                    let gradient = params["channel_gradient_deg"].as_f64().unwrap_or(0.0);
                    let material = params["material_volume_per_km"].as_f64().unwrap_or(0.0);
                    let rf = params["rainfall_24h_mm"].as_f64().unwrap_or(0.0);
                    Some(self.debris_flow_hazard(gradient, material, rf))
                } else {
                    None
                };

                let assessment = self.overall_assessment(ls, df, &aoi_name);
                Ok(serde_json::to_value(&assessment).map_err(GeoError::Serde)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::plugin::ProcessPlugin;

    fn make_plugin() -> GeohazardPlugin {
        let config = Default::default();
        GeohazardPlugin::new(config)
    }

    #[tokio::test]
    async fn test_execute_landslide() {
        let p = make_plugin();
        let result = p
            .execute(serde_json::json!({
                "task": "landslide",
                "slope_deg": 40.0,
                "aspect_deg": 180.0,
                "lithology_index": 1.0,
                "rainfall_mm": 400.0,
                "fault_distance_m": 10.0,
                "ndvi": 0.05
            }))
            .await
            .unwrap();

        assert_eq!(result["risk_level"], "very_high");
        assert!(result["susceptibility"].as_f64().unwrap() > 0.8);
    }

    #[tokio::test]
    async fn test_execute_debris_flow() {
        let p = make_plugin();
        let result = p
            .execute(serde_json::json!({
                "task": "debris_flow",
                "channel_gradient_deg": 40.0,
                "material_volume_per_km": 20000.0,
                "rainfall_24h_mm": 200.0
            }))
            .await
            .unwrap();

        assert!(result["hazard"].as_f64().unwrap() > 0.8);
    }

    #[tokio::test]
    async fn test_execute_risk_map() {
        let p = make_plugin();
        let result = p
            .execute(serde_json::json!({
                "aoi_name": "test-area",
                "slope_deg": 40.0,
                "aspect_deg": 180.0,
                "lithology_index": 1.0,
                "rainfall_mm": 400.0,
                "fault_distance_m": 10.0,
                "ndvi": 0.05
            }))
            .await
            .unwrap();

        assert_eq!(result["overall_risk"], "very_high");
        assert!(result["landslide"]["susceptibility"].as_f64().unwrap() > 0.8);
    }

    #[tokio::test]
    async fn test_execute_risk_map_with_debris() {
        let p = make_plugin();
        let result = p
            .execute(serde_json::json!({
                "aoi_name": "test-area",
                "slope_deg": 25.0,
                "aspect_deg": 180.0,
                "lithology_index": 0.5,
                "rainfall_mm": 200.0,
                "fault_distance_m": 100.0,
                "ndvi": 0.3,
                "channel_gradient_deg": 30.0,
                "material_volume_per_km": 5000.0,
                "rainfall_24h_mm": 100.0
            }))
            .await
            .unwrap();

        assert!(result["debris_flow"].is_object());
        assert!(result["landslide"].is_object());
        assert!(result["overall_risk"].is_string());
    }
}
