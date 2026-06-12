//! Tool registration — OSM.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::ToolDef;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "osm".into(), version: "0.1.0".into(), description: "OpenStreetMap Overpass API".into(), category: PluginCategory::Adapter, healthy: true, extra: serde_json::json!({"endpoint":"https://overpass-api.de/api/interpreter"}) });
    registry.register_tool_async("osm", ToolDef { name: "osm_query_bbox".into(), description: "Query OSM features by bbox and type".into(), input_schema: serde_json::json!({"type":"object","properties":{"min_lon":{"type":"number"},"min_lat":{"type":"number"},"max_lon":{"type":"number"},"max_lat":{"type":"number"},"feature_type":{"type":"string","description":"highway|building|waterway|landuse|poi"}},"required":["min_lon","min_lat","max_lon","max_lat","feature_type"]}) }, |args| Box::pin(async move {
        use crate::client::OsmFeature;
        let ft = match args["feature_type"].as_str().unwrap_or("highway") { "building"=>OsmFeature::Building, "waterway"=>OsmFeature::Waterway, "landuse"=>OsmFeature::Landuse, "poi"=>OsmFeature::Poi, _=>OsmFeature::Highway };
        let client = crate::OsmClient::new();
        let elements = client.query_bbox(args["min_lon"].as_f64().unwrap_or(0.0),args["min_lat"].as_f64().unwrap_or(0.0),args["max_lon"].as_f64().unwrap_or(0.0),args["max_lat"].as_f64().unwrap_or(0.0),ft).await.map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
        Ok(crate::OsmClient::to_geojson(&elements))
    }));
}
