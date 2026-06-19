use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

/// Register geospatial statistics tools.
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "stats", "Spatial statistics: zonal, Moran's I, Hotspot Gi*, IDW interpolation, classification", PluginCategory::Process, [

        sync "zonal_stats" => "Compute zonal statistics for raster data within bboxes" ; serde_json::json!({"type":"object","properties":{"zones":{"type":"array"},"raster_data":{"type":"array","items":{"type":"number"}},"raster_cols":{"type":"integer"},"raster_min_x":{"type":"number"},"raster_min_y":{"type":"number"},"raster_max_x":{"type":"number"},"raster_max_y":{"type":"number"},"nodata":{"type":"number"}},"required":["zones","raster_data","raster_cols"]}) => |args| -> ToolResult {
        let empty = vec![];
        let zones_json = args["zones"].as_array().unwrap_or(&empty);
        let data: Vec<f64> = args["raster_data"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_f64()).collect();
        let cols = args["raster_cols"].as_u64().unwrap_or(1) as usize;
        let rows = data.len()/cols.max(1);
        let nodata = args["nodata"].as_f64().unwrap_or(-999.0);
        let rb = geo_core::types::BBox{min_x:args["raster_min_x"].as_f64().unwrap_or(0.0),min_y:args["raster_min_y"].as_f64().unwrap_or(0.0),max_x:args["raster_max_x"].as_f64().unwrap_or(0.0),max_y:args["raster_max_y"].as_f64().unwrap_or(0.0)};
        let mut results = Vec::new();
        for z in zones_json {
            let zn = z["name"].as_str().unwrap_or("zone");
            let zb = geo_core::types::BBox{min_x:z["min_x"].as_f64().unwrap_or(0.0),min_y:z["min_y"].as_f64().unwrap_or(0.0),max_x:z["max_x"].as_f64().unwrap_or(0.0),max_y:z["max_y"].as_f64().unwrap_or(0.0)};
            let zr = crate::zonal_stats(&data,rows,cols,nodata,rb,&zb,zn).map_err(|e| geo_core::GeoError::Validation(e.to_string()))?;
            results.push(serde_json::json!({"zone":zn,"pixel_count":zr.pixel_count,"mean":zr.mean,"min":zr.min,"max":zr.max,"sum":zr.sum}));
        }
        Ok(serde_json::json!(results))
    },
        sync "morans_i" => "Moran's I spatial autocorrelation (grid-based)" ; serde_json::json!({"type":"object","properties":{"values":{"type":"array","items":{"type":"number"}},"nrows":{"type":"integer"},"ncols":{"type":"integer"},"contiguity":{"type":"string","enum":["rook","queen"],"default":"rook"}},"required":["values","nrows","ncols"]}) => |args| -> ToolResult {
        let values: Vec<f64> = args["values"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let nrows = args["nrows"].as_u64().unwrap_or(1) as usize;
        let ncols = args["ncols"].as_u64().unwrap_or(1) as usize;
        let rook = args["contiguity"].as_str().unwrap_or("rook") == "rook";
        let weights = if rook { crate::moran::rook_weights(nrows, ncols) } else { crate::moran::queen_weights(nrows, ncols) };
        match crate::moran::morans_i(&values, &weights) {
            Some(result) => Ok(serde_json::json!({"i":result.i,"expected_i":result.expected_i,"z_score":result.z_score,"p_value":result.p_value})),
            None => Err(geo_core::GeoError::Validation("Moran's I computation failed (check data variance)".into())),
        }
    },
        sync "gistar" => "Getis-Ord Gi* hotspot analysis (grid-based)" ; serde_json::json!({"type":"object","properties":{"values":{"type":"array","items":{"type":"number"}},"nrows":{"type":"integer"},"ncols":{"type":"integer"},"confidence":{"type":"number","default":0.05}},"required":["values","nrows","ncols"]}) => |args| -> ToolResult {
        let values: Vec<f64> = args["values"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let nrows = args["nrows"].as_u64().unwrap_or(1) as usize;
        let ncols = args["ncols"].as_u64().unwrap_or(1) as usize;
        let confidence = args["confidence"].as_f64().unwrap_or(0.05);
        let weights = crate::hotspot::queen_weights_self(nrows, ncols);
        let hotspots = crate::hotspot::gistar(&values, &weights, confidence).ok_or_else(|| geo_core::GeoError::Validation("Gi* computation failed".into()))?;
        Ok(serde_json::json!({"results": hotspots.iter().map(|h| serde_json::json!({"index":h.index,"z_score":h.z_score,"p_value":h.p_value,"is_hotspot":h.is_hotspot,"is_coldspot":h.is_coldspot})).collect::<Vec<_>>()}))
    },
        sync "idw_grid" => "IDW spatial interpolation (regular grid)" ; serde_json::json!({"type":"object","properties":{"x_src":{"type":"array","items":{"type":"number"}},"y_src":{"type":"array","items":{"type":"number"}},"values_src":{"type":"array","items":{"type":"number"}},"ncols":{"type":"integer"},"nrows":{"type":"integer"},"bbox":{"type":"object","properties":{"min_x":{"type":"number"},"min_y":{"type":"number"},"max_x":{"type":"number"},"max_y":{"type":"number"}},"required":["min_x","min_y","max_x","max_y"]},"power":{"type":"number","default":2},"max_radius":{"type":"number","default":0},"min_neighbors":{"type":"integer","default":1}},"required":["x_src","y_src","values_src","ncols","nrows","bbox"]}) => |args| -> ToolResult {
        let x_src: Vec<f64> = args["x_src"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let y_src: Vec<f64> = args["y_src"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let values_src: Vec<f64> = args["values_src"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let ncols = args["ncols"].as_u64().unwrap_or(10) as usize;
        let nrows = args["nrows"].as_u64().unwrap_or(10) as usize;
        let bbox_json = &args["bbox"];
        let bbox = geo_core::types::BBox {
            min_x: bbox_json["min_x"].as_f64().unwrap_or(0.0),
            min_y: bbox_json["min_y"].as_f64().unwrap_or(0.0),
            max_x: bbox_json["max_x"].as_f64().unwrap_or(1.0),
            max_y: bbox_json["max_y"].as_f64().unwrap_or(1.0),
        };
        let power = args["power"].as_f64().unwrap_or(2.0);
        let max_radius = args["max_radius"].as_f64().unwrap_or(0.0);
        let min_neighbors = args["min_neighbors"].as_u64().unwrap_or(1) as usize;
        let (grid, _meta) = crate::idw::idw_grid(&bbox, ncols, nrows, &x_src, &y_src, &values_src, power, max_radius, min_neighbors);
        Ok(serde_json::json!({"grid":grid,"ncols":ncols,"nrows":nrows}))
    },
        sync "jenks_classify" => "Jenks Natural Breaks classification" ; serde_json::json!({"type":"object","properties":{"values":{"type":"array","items":{"type":"number"}},"k":{"type":"integer","description":"number of classes 2-10"}},"required":["values","k"]}) => |args| -> ToolResult {
        let values: Vec<f64> = args["values"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let k = args["k"].as_u64().unwrap_or(3) as usize;
        match crate::classify::jenks(&values, k) {
            Some(result) => Ok(serde_json::json!({"breaks":result.breaks,"gvf":result.gvf,"classes":result.classes})),
            None => Err(geo_core::GeoError::Validation("Jenks classification failed".into())),
        }
    },
    ]);
}
