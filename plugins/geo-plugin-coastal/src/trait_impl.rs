use crate::CoastalPlugin;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{EmptyConfig, Plugin, PluginCategory, ProcessPlugin};
impl Plugin for CoastalPlugin {
    fn name(&self) -> &str {
        "coastal"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Coastal monitoring"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
    type Config = EmptyConfig;
    fn new(_config: EmptyConfig) -> Self {
        Self
    }
}
impl ProcessPlugin for CoastalPlugin {
    fn process_type(&self) -> &str {
        "coastal"
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
        serde_json::to_value(self.assess_shoreline(
            p["aoi_name"].as_str().unwrap_or(""),
            p["aoi_geojson"].as_str().unwrap_or(""),
            &mk("dem_data"),
            &mk("ndvi_old"),
            &mk("ndvi_new"),
            p["baseline_year"].as_u64().unwrap_or(2015) as u16,
            p["assessment_year"].as_u64().unwrap_or(2025) as u16,
            p["erosion_threshold_m"].as_f64().unwrap_or(1.0),
        )?)
        .map_err(GeoError::Serde)
    }
}
