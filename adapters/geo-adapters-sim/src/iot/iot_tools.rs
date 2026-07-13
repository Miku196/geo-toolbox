//! Tool registration — IoT.
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "iot", "IoT sensor adapter: validate + parse messages", PluginCategory::Adapter, [
        async "iot_ingest_message" => "Parse and validate an IoT sensor JSON message" ; serde_json::json!({"type":"object","properties":{"message":{"type":"string"}},"required":["message"]}) => |args| Box::pin(async move {
        let v: serde_json::Value = serde_json::from_str(args["message"].as_str().unwrap_or("{}")).map_err(|e| geo_core::GeoError::invalid_input("message",e.to_string()))?;
        let (lat,lng,val,st) = (v["lat"].as_f64(), v["lng"].as_f64(), v["value"].as_f64(), v["sensor_type"].as_str().unwrap_or(""));
        Ok(match (lat,lng,val,st) {
            (Some(lat),Some(lng),Some(val),st) if geo_core::types::validate_coord(lng,lat).is_ok() => {
                let vok = match st { "temperature"=>( -50.0..=60.0).contains(&val), "humidity"=>(0.0..=100.0).contains(&val), "pm25"=>(0.0..=1000.0).contains(&val), _=>true };
                serde_json::json!({"valid":vok,"lat":lat,"lng":lng,"sensor_type":st,"value":val})
            }
            _ => serde_json::json!({"valid":false,"reason":"invalid coords or missing fields"}),
        })
    })]);
}
