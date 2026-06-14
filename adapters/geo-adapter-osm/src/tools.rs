//! Tool registration — OSM.
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "osm", "OpenStreetMap Overpass API", PluginCategory::Adapter, [
        async "osm_query_bbox" => "Query OSM features by bbox and type" ; serde_json::json!({"type":"object","properties":{"min_lon":{"type":"number"},"min_lat":{"type":"number"},"max_lon":{"type":"number"},"max_lat":{"type":"number"},"feature_type":{"type":"string","description":"highway|building|waterway|landuse|poi"}},"required":["min_lon","min_lat","max_lon","max_lat","feature_type"]}) => |args| Box::pin(async move {
        use crate::client::OsmFeature;
        let ft = match args["feature_type"].as_str().unwrap_or("highway") { "building"=>OsmFeature::Building, "waterway"=>OsmFeature::Waterway, "landuse"=>OsmFeature::Landuse, "poi"=>OsmFeature::Poi, _=>OsmFeature::Highway };
        let client = crate::OsmClient::new();
        let elements = client.query_bbox(args["min_lon"].as_f64().unwrap_or(0.0),args["min_lat"].as_f64().unwrap_or(0.0),args["max_lon"].as_f64().unwrap_or(0.0),args["max_lat"].as_f64().unwrap_or(0.0),ft).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(crate::OsmClient::to_geojson(&elements))
    })]);
}
