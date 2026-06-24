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
            p["baseline_area_ha"].as_f64().unwrap_or(100.0),
            p["baseline_volume_m3_ha"].as_f64().unwrap_or(200.0),
        )?)
        .map_err(GeoError::Serde)
    }
}
