//! Tool registration — IoT.
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;
use geo_registry::registry::ToolDef;
pub fn register_tools(registry: &mut PluginRegistry) {
    registry.register(geo_core::plugin::PluginMeta { name: "iot".into(), version: "0.1.0".into(), description: "IoT sensor adapter: validate + parse messages".into(), category: PluginCategory::Adapter, healthy: true, extra: serde_json::json!({"endpoint":"mqtt://localhost:1883"}) });
    registry.register_tool_async("iot", ToolDef { name: "iot_ingest_message".into(), description: "Parse and validate an IoT sensor JSON message".into(), input_schema: serde_json::json!({"type":"object","properties":{"message":{"type":"string"}},"required":["message"]}) }, |args| Box::pin(async move {
        let v: serde_json::Value = serde_json::from_str(args["message"].as_str().unwrap_or("{}")).map_err(|e| geo_core::GeoError::invalid_input("message",e.to_string()))?;
        let (lat,lng,val,st) = (v["lat"].as_f64(), v["lng"].as_f64(), v["value"].as_f64(), v["sensor_type"].as_str().unwrap_or(""));
        Ok(match (lat,lng,val,st) {
            (Some(lat),Some(lng),Some(val),st) if geo_core::types::validate_coord(lng,lat).is_ok() => {
                let vok = match st { "temperature"=>( -50.0..=60.0).contains(&val), "humidity"=>(0.0..=100.0).contains(&val), "pm25"=>(0.0..=1000.0).contains(&val), _=>true };
                serde_json::json!({"valid":vok,"lat":lat,"lng":lng,"sensor_type":st,"value":val})
            }
            _ => serde_json::json!({"valid":false,"reason":"invalid coords or missing fields"}),
        })
    }));
}
