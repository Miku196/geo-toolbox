//! Tool registration — Coastal plugin.
use crate::blue_carbon::BlueCarbonResult;
use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "coastal", "Coastal change monitoring: erosion + inundation + storm surge", PluginCategory::Process, [
        sync "coastal_shoreline" => "Assess shoreline erosion and inundation between two periods" ; serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"dem_data":{"type":"array","items":{"type":"number"}},"ndvi_old":{"type":"array","items":{"type":"number"}},"ndvi_new":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"baseline_year":{"type":"integer"},"assessment_year":{"type":"integer"},"erosion_threshold_m":{"type":"number"},"nodata":{"type":"number"}},"required":["aoi_name","dem_data","ndvi_old","ndvi_new","cols","rows","baseline_year","assessment_year"]}) => |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0);let c=args["cols"].as_u64().unwrap_or(1) as usize;let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
        let report=crate::CoastalPlugin::new().assess_shoreline(args["aoi_name"].as_str().unwrap_or(""),args["aoi_geojson"].as_str().unwrap_or(""),&mk("dem_data","dem"),&mk("ndvi_old","o"),&mk("ndvi_new","n"),args["baseline_year"].as_u64().unwrap_or(2015) as u16,args["assessment_year"].as_u64().unwrap_or(2025) as u16,args["erosion_threshold_m"].as_f64().unwrap_or(1.0))?;
        Ok(serde_json::to_value(report).map_err(geo_core::GeoError::Serde)?)
    },
        sync "coastal_storm_surge" => "SLOSH simplified storm surge model: Holland wind + wind setup + inundation" ; serde_json::json!({"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"central_pressure_hpa":{"type":"number"},"rmax_km":{"type":"number"},"forward_speed_m_s":{"type":"number"},"forward_bearing_deg":{"type":"number"},"dem":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"cell_size_m":{"type":"number"},"land_mask":{"type":"array","items":{"type":"boolean"}}},"required":["dem","cols","rows","cell_size_m","land_mask"]}) => |args| -> ToolResult {
        let params=crate::storm_surge::StormParams{
            lat:args["lat"].as_f64().unwrap_or(30.0),
            lon:args["lon"].as_f64().unwrap_or(122.0),
            central_pressure_hpa:args["central_pressure_hpa"].as_f64().unwrap_or(955.0),
            ambient_pressure_hpa:args["ambient_pressure_hpa"].as_f64().unwrap_or(1013.0),
            rmax_km:args["rmax_km"].as_f64().unwrap_or(40.0),
            forward_speed_m_s:args["forward_speed_m_s"].as_f64().unwrap_or(5.0),
            forward_bearing_deg:args["forward_bearing_deg"].as_f64().unwrap_or(0.0),
            holland_b:args["holland_b"].as_f64().unwrap_or(1.3),
        };
        let dem:Vec<f64>=args["dem"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let rows=args["rows"].as_u64().unwrap_or(1) as usize;
        let cols=args["cols"].as_u64().unwrap_or(1) as usize;
        let cell_size_m=args["cell_size_m"].as_f64().unwrap_or(1000.0);
        let land_mask:Vec<bool>=args["land_mask"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_bool()).collect();
        let result=crate::CoastalPlugin::new().storm_surge(&params,&dem,rows,cols,cell_size_m,&land_mask)?;
        Ok(serde_json::to_value(result).map_err(geo_core::GeoError::Serde)?)
    },
        sync "coastal_blue_carbon" => "Blue carbon stock & sequestration: mangrove/salt-marsh/seagrass" ; serde_json::json!({"type":"object","properties":{"ecosystem":{"type":"string","enum":["mangrove","salt_marsh","seagrass"]},"area_ha":{"type":"number"},"soil_factor":{"type":"number"}},"required":["ecosystem","area_ha"]}) => |args| -> ToolResult {
        let eco=args["ecosystem"].as_str().unwrap_or("mangrove");
        let area=args["area_ha"].as_f64().unwrap_or(1.0);
        let sf=args["soil_factor"].as_f64().unwrap_or(1.0);
        let r=crate::CoastalPlugin::new().assess_blue_carbon(eco,area,sf)?;
        Ok(serde_json::to_value(r).map_err(geo_core::GeoError::Serde)?)
    },
        sync "coastal_blue_carbon_aggregate" => "Aggregate multiple blue carbon assessments" ; serde_json::json!({"type":"object","properties":{"items":{"type":"array","items":{"type":"object","properties":{"ecosystem":{"type":"string","enum":["mangrove","salt_marsh","seagrass"]},"area_ha":{"type":"number"},"soil_factor":{"type":"number"}},"required":["ecosystem","area_ha"]}}},"required":["items"]}) => |args| -> ToolResult {
        let items: Vec<BlueCarbonResult> = args["items"].as_array().unwrap_or(&vec![]).iter().map(|v| {
            let eco=v["ecosystem"].as_str().unwrap_or("mangrove");
            let area=v["area_ha"].as_f64().unwrap_or(1.0);
            let sf=v["soil_factor"].as_f64().unwrap_or(1.0);
            crate::CoastalPlugin::new().assess_blue_carbon(eco,area,sf)
        }).filter_map(|r| r.ok()).collect();
        let agg=crate::CoastalPlugin::new().aggregate_blue_carbon(items);
        Ok(serde_json::to_value(agg).map_err(geo_core::GeoError::Serde)?)
    },
        sync "coastal_storm_surge_1d" => "1D storm surge profile: quick max surge along transect" ; serde_json::json!({"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"central_pressure_hpa":{"type":"number"},"rmax_km":{"type":"number"},"forward_speed_m_s":{"type":"number"},"coast_distance_km":{"type":"array","items":{"type":"number"}},"bathymetry_m":{"type":"array","items":{"type":"number"}}},"required":["coast_distance_km","bathymetry_m"]}) => |args| -> ToolResult {
        let params=crate::storm_surge::StormParams{
            lat:args["lat"].as_f64().unwrap_or(30.0),
            lon:args["lon"].as_f64().unwrap_or(122.0),
            central_pressure_hpa:args["central_pressure_hpa"].as_f64().unwrap_or(955.0),
            ambient_pressure_hpa:args["ambient_pressure_hpa"].as_f64().unwrap_or(1013.0),
            rmax_km:args["rmax_km"].as_f64().unwrap_or(40.0),
            forward_speed_m_s:args["forward_speed_m_s"].as_f64().unwrap_or(5.0),
            forward_bearing_deg:args["forward_bearing_deg"].as_f64().unwrap_or(0.0),
            holland_b:args["holland_b"].as_f64().unwrap_or(1.3),
        };
        let dist:Vec<f64>=args["coast_distance_km"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let bathy:Vec<f64>=args["bathymetry_m"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let surge=crate::CoastalPlugin::new().storm_surge_1d(&params,&dist,&bathy)?;
        Ok(serde_json::json!({ "max_surge_m": surge }))
    }]);
}
