use geo_registry::register_plugin;
use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "geomorph", "Geomorphology — D8 flow direction, accumulation, Strahler order, valley cross-section", PluginCategory::Process, [
        sync "geomorph_d8_flow_dir" => "D8 flow direction from DEM" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"rows":{"type":"integer"},"cols":{"type":"integer"}},"required":["dem","rows","cols"]}) => |args| -> ToolResult {
            let dem:Vec<f64>=args["dem"].as_array().map(|a|a.iter().filter_map(|v|v.as_f64()).collect()).unwrap_or_default();
            let rows=args["rows"].as_u64().unwrap_or(1) as usize;let cols=args["cols"].as_u64().unwrap_or(1) as usize;
            let result=crate::d8_flow_direction(&dem,rows,cols);
            serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
        },
        sync "geomorph_d8_flow_dir_filled" => "D8 flow direction (pit-filled)" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"rows":{"type":"integer"},"cols":{"type":"integer"}},"required":["dem","rows","cols"]}) => |args| -> ToolResult {
            let dem:Vec<f64>=args["dem"].as_array().map(|a|a.iter().filter_map(|v|v.as_f64()).collect()).unwrap_or_default();
            let rows=args["rows"].as_u64().unwrap_or(1) as usize;let cols=args["cols"].as_u64().unwrap_or(1) as usize;
            let result=crate::d8_flow_direction_filled(&dem,rows,cols);
            serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
        },
        sync "geomorph_d8_flow_acc" => "D8 flow accumulation" ; serde_json::json!({"type":"object","properties":{"flow_dir":{"type":"array","items":{"type":"integer","minimum":0,"maximum":255}},"rows":{"type":"integer"},"cols":{"type":"integer"}},"required":["flow_dir","rows","cols"]}) => |args| -> ToolResult {
            let fd:Vec<u8>=args["flow_dir"].as_array().map(|a|a.iter().filter_map(|v|v.as_u64().map(|x|x as u8)).collect()).unwrap_or_default();
            let rows=args["rows"].as_u64().unwrap_or(1) as usize;let cols=args["cols"].as_u64().unwrap_or(1) as usize;
            let result=crate::d8_flow_accumulation_fast(&fd,rows,cols);
            serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
        },
        sync "geomorph_extract_streams" => "Extract stream network from flow accumulation" ; serde_json::json!({"type":"object","properties":{"flow_acc":{"type":"array","items":{"type":"integer"}},"rows":{"type":"integer"},"cols":{"type":"integer"},"threshold":{"type":"integer","default":100}},"required":["flow_acc","rows","cols"]}) => |args| -> ToolResult {
            let fa:Vec<u32>=args["flow_acc"].as_array().map(|a|a.iter().filter_map(|v|v.as_u64().map(|x|x as u32)).collect()).unwrap_or_default();
            let rows=args["rows"].as_u64().unwrap_or(1) as usize;let cols=args["cols"].as_u64().unwrap_or(1) as usize;
            let t=args["threshold"].as_u64().unwrap_or(100) as u32;
            let result=crate::extract_streams(&fa,rows,cols,t);
            serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
        },
        sync "geomorph_strahler" => "Strahler stream order" ; serde_json::json!({"type":"object","properties":{"flow_dir":{"type":"array","items":{"type":"integer","minimum":0,"maximum":255}},"stream_mask":{"type":"array","items":{"type":"boolean"}},"rows":{"type":"integer"},"cols":{"type":"integer"}},"required":["flow_dir","stream_mask","rows","cols"]}) => |args| -> ToolResult {
            let fd:Vec<u8>=args["flow_dir"].as_array().map(|a|a.iter().filter_map(|v|v.as_u64().map(|x|x as u8)).collect()).unwrap_or_default();
            let sm:Vec<bool>=args["stream_mask"].as_array().map(|a|a.iter().filter_map(|v|v.as_bool()).collect()).unwrap_or_default();
            let rows=args["rows"].as_u64().unwrap_or(1) as usize;let cols=args["cols"].as_u64().unwrap_or(1) as usize;
            let result=crate::strahler_order(&fd,&sm,rows,cols);
            serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
        },
        sync "geomorph_valley_cs" => "Valley cross-section from DEM at channel cell" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"dem_rows":{"type":"integer"},"dem_cols":{"type":"integer"},"channel_row":{"type":"integer"},"channel_col":{"type":"integer"},"half_width":{"type":"integer","default":10}},"required":["dem","dem_rows","dem_cols","channel_row","channel_col"]}) => |args| -> ToolResult {
            let dem:Vec<f64>=args["dem"].as_array().map(|a|a.iter().filter_map(|v|v.as_f64()).collect()).unwrap_or_default();
            let dr=args["dem_rows"].as_u64().unwrap_or(1) as usize;let dc=args["dem_cols"].as_u64().unwrap_or(1) as usize;
            let cr=args["channel_row"].as_u64().unwrap_or(0) as usize;let cc=args["channel_col"].as_u64().unwrap_or(0) as usize;
            let hw=args["half_width"].as_u64().unwrap_or(10) as usize;
            let result=crate::valley_cross_section(&dem,dr,dc,cr,cc,hw);
            serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
        },
        sync "geomorph_stream_segments" => "Extract individual stream segments" ; serde_json::json!({"type":"object","properties":{"flow_dir":{"type":"array","items":{"type":"integer","minimum":0,"maximum":255}},"stream_mask":{"type":"array","items":{"type":"boolean"}},"order":{"type":"array","items":{"type":"integer"}},"rows":{"type":"integer"},"cols":{"type":"integer"}},"required":["flow_dir","stream_mask","order","rows","cols"]}) => |args| -> ToolResult {
            let fd:Vec<u8>=args["flow_dir"].as_array().map(|a|a.iter().filter_map(|v|v.as_u64().map(|x|x as u8)).collect()).unwrap_or_default();
            let sm:Vec<bool>=args["stream_mask"].as_array().map(|a|a.iter().filter_map(|v|v.as_bool()).collect()).unwrap_or_default();
            let order:Vec<u8>=args["order"].as_array().map(|a|a.iter().filter_map(|v|v.as_u64().map(|x|x as u8)).collect()).unwrap_or_default();
            let rows=args["rows"].as_u64().unwrap_or(1) as usize;let cols=args["cols"].as_u64().unwrap_or(1) as usize;
            let result=crate::extract_stream_segments(&fd,&sm,&order,rows,cols);
            serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
        }
    ]);
}
