//! Plugin + ProcessPlugin trait impls.
use crate::UrbanPlugin;
use geo_core::plugin::{Plugin, ProcessPlugin, PluginCategory};
use geo_core::errors::GeoResult;
impl Plugin for UrbanPlugin { fn name(&self)->&str{"urban"} fn version(&self)->&str{"0.1"} fn description(&self)->&str{"Urban planning"} fn category(&self)->PluginCategory{PluginCategory::Process} }
impl ProcessPlugin for UrbanPlugin { fn process_type(&self)->&str{"urban"} async fn execute(&self,p:serde_json::Value)->GeoResult<serde_json::Value>{let t=p["total_floor_area_m2"].as_f64().unwrap_or(0.0);let s=p["site_area_m2"].as_f64().unwrap_or(0.0);Ok(serde_json::json!({"far":self.far(t,s),"density":self.building_density(t,s)}))} }
