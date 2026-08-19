use crate::ForestryConfig;
use crate::ForestryPlugin;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};
impl Plugin for ForestryPlugin {
    type Config = ForestryConfig;
    fn new(config: ForestryConfig) -> Self {
        Self::new(config)
    }
    fn name(&self) -> &str {
        "forestry"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Forest carbon stock"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Carbon
    }
}
impl ProcessPlugin for ForestryPlugin {
    fn process_type(&self) -> &str {
        "forestry"
    }
    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        use geo_raster::RasterBand;
        let nd = p["nodata"].as_f64().unwrap_or(-999.0);
        let c = p["cols"].as_u64().unwrap_or(1) as usize;
        let r = p["rows"].as_u64().unwrap_or(1) as usize;
        let mk = |k: &str| {
            let v: Vec<f64> = p[k]
                .as_array()
                .map(|a| a.as_slice())
                .unwrap_or(&[])
                .iter()
                .filter_map(|x| x.as_f64())
                .collect();
            RasterBand::new(k, c, r, v, nd)
        };
        serde_json::to_value(self.assess_carbon_stock(
            p["aoi_name"].as_str().unwrap_or(""),
            p["aoi_geojson"].as_str().unwrap_or(""),
            &mk("red_old"),
            &mk("nir_old"),
            &mk("red_new"),
            &mk("nir_new"),
            p["year_old"].as_u64().unwrap_or(2020) as u16,
            p["year_new"].as_u64().unwrap_or(2025) as u16,
            p["baseline_volume_m3_ha"].as_f64().unwrap_or(200.0),
            p["baseline_area_ha"].as_f64().unwrap_or(100.0),
        )?)
        .map_err(GeoError::Serde)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::plugin::ProcessPlugin;

    fn make_params() -> serde_json::Value {
        // Non-symmetric inputs: small area (10 ha) + large volume (10000 m3/ha).
        // Correct semantics: ccer_applicable must be false (area 10 < 100).
        // The param-swap bug passes baseline_volume_m3_ha (10000) into forest_area_ha,
        // which makes ccer_applicable true - so this test reproduces the swap.
        serde_json::json!({
            "aoi_name": "测试林场",
            "aoi_geojson": "{\"type\":\"FeatureCollection\",\"features\":[{\"type\":\"Feature\",\"properties\":{},\"geometry\":{\"type\":\"Polygon\",\"coordinates\":[[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]}}]}",
            "red_old": [0.15, 0.16],
            "nir_old": [0.40, 0.42],
            "red_new": [0.10, 0.11],
            "nir_new": [0.55, 0.58],
            "cols": 2,
            "rows": 1,
            "year_old": 2020,
            "year_new": 2025,
            "baseline_area_ha": 10.0,
            "baseline_volume_m3_ha": 10000.0
        })
    }

    #[tokio::test]
    async fn test_execute_carbon_stock_respects_area_not_volume_swap() {
        let plugin = ForestryPlugin::new(ForestryConfig::default());
        let result = plugin.execute(make_params()).await.unwrap();
        // area=10 < 100 => ccer_applicable must be false. Bug version uses 10000 as
        // forest_area_ha and returns true.
        assert_eq!(result["ccer_applicable"], false);
    }
}

