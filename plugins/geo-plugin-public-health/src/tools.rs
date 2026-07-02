use geo_registry::register_plugin;
use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "public-health", "Environmental public health — UHI, PM2.5, disease vectors, risk index", PluginCategory::Process, [
        sync "health_uhi" => "Urban Heat Island intensity" ; serde_json::json!({"type":"object","properties":{"urban_temp_c":{"type":"number"},"rural_temp_c":{"type":"number"}},"required":["urban_temp_c","rural_temp_c"]}) => |args| -> ToolResult {
            let u=args["urban_temp_c"].as_f64().unwrap_or(38.0);let r=args["rural_temp_c"].as_f64().unwrap_or(30.0);
            let p=crate::HealthPlugin;
            serde_json::to_value(p.uhi(u,r)).map_err(geo_core::GeoError::Serde)
        },
        sync "health_pm25" => "PM2.5 exposure and AQI" ; serde_json::json!({"type":"object","properties":{"pm25_ug_m3":{"type":"number"},"breathing_rate_m3_day":{"type":"number","default":20},"days_exposed":{"type":"number","default":365}},"required":["pm25_ug_m3"]}) => |args| -> ToolResult {
            let pm=args["pm25_ug_m3"].as_f64().unwrap_or(35.0);let br=args["breathing_rate_m3_day"].as_f64().unwrap_or(20.0);let d=args["days_exposed"].as_f64().unwrap_or(365.0);
            let p=crate::HealthPlugin;
            serde_json::to_value(p.pm25(pm,br,d)).map_err(geo_core::GeoError::Serde)
        },
        sync "health_vector" => "Disease vector habitat suitability" ; serde_json::json!({"type":"object","properties":{"temp_c":{"type":"number"},"precip_mm_month":{"type":"number"},"vector_type":{"type":"string","enum":["aedes","anopheles","culex","ixodes"]}},"required":["temp_c","precip_mm_month","vector_type"]}) => |args| -> ToolResult {
            let t=args["temp_c"].as_f64().unwrap_or(25.0);let p=args["precip_mm_month"].as_f64().unwrap_or(60.0);let v=args["vector_type"].as_str().unwrap_or("aedes");
            serde_json::to_value(crate::HealthPlugin.vector(t,p,v)).map_err(geo_core::GeoError::Serde)
        },
        sync "health_risk" => "Composite environmental health risk index" ; serde_json::json!({"type":"object","properties":{"pm25_ug_m3":{"type":"number"},"max_temp_c":{"type":"number"},"water_quality_index":{"type":"number"}},"required":["pm25_ug_m3","max_temp_c","water_quality_index"]}) => |args| -> ToolResult {
            let pm=args["pm25_ug_m3"].as_f64().unwrap_or(35.0);let t=args["max_temp_c"].as_f64().unwrap_or(38.0);let wq=args["water_quality_index"].as_f64().unwrap_or(70.0);
            serde_json::to_value(crate::HealthPlugin.risk(pm,t,wq)).map_err(geo_core::GeoError::Serde)
        }
    ]);
}
