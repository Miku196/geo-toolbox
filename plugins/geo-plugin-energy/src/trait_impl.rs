use crate::EnergyPlugin;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{Plugin, PluginCategory, ProcessPlugin};
impl Plugin for EnergyPlugin {
    fn name(&self) -> &str {
        "energy"
    }
    fn version(&self) -> &str {
        "0.1"
    }
    fn description(&self) -> &str {
        "Solar/wind site suitability"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Process
    }
}
impl ProcessPlugin for EnergyPlugin {
    fn process_type(&self) -> &str {
        "energy"
    }
    async fn execute(&self, p: serde_json::Value) -> GeoResult<serde_json::Value> {
        use geo_raster::RasterBand;
        let nd = p["nodata"].as_f64().unwrap_or(-999.0);
        let c = p["cols"].as_u64().unwrap_or(1) as usize;
        let r = p["rows"].as_u64().unwrap_or(1) as usize;
        let mk = |k: &str| {
            let v: Vec<f64> = p[k]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|x| x.as_f64())
                .collect();
            RasterBand::new(k, c, r, v, nd)
        };
        Ok(serde_json::to_value(self.assess_solar(
            p["aoi_name"].as_str().unwrap_or(""),
            p["aoi_geojson"].as_str().unwrap_or(""),
            &mk("dem_data"),
            &mk("radiation_data"),
        )?)
        .map_err(GeoError::Serde)?)
    }
}
