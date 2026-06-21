//! Tool registration — Coastal plugin.
use crate::blue_carbon::BlueCarbonResult;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "coastal", "Coastal change monitoring: erosion + inundation + storm surge + SLR + CVI", PluginCategory::Process, [
        sync "coastal_shoreline" => "Assess shoreline erosion and inundation between two periods" ; serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"dem_data":{"type":"array","items":{"type":"number"}},"ndvi_old":{"type":"array","items":{"type":"number"}},"ndvi_new":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"baseline_year":{"type":"integer"},"assessment_year":{"type":"integer"},"erosion_threshold_m":{"type":"number"},"nodata":{"type":"number"}},"required":["aoi_name","dem_data","ndvi_old","ndvi_new","cols","rows","baseline_year","assessment_year"]}) => |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0);let c=args["cols"].as_u64().unwrap_or(1) as usize;let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
        let report=crate::CoastalPlugin::new().assess_shoreline(args["aoi_name"].as_str().unwrap_or(""),args["aoi_geojson"].as_str().unwrap_or(""),&mk("dem_data","dem"),&mk("ndvi_old","o"),&mk("ndvi_new","n"),args["baseline_year"].as_u64().unwrap_or(2015) as u16,args["assessment_year"].as_u64().unwrap_or(2025) as u16,args["erosion_threshold_m"].as_f64().unwrap_or(1.0))?;
        serde_json::to_value(report).map_err(geo_core::GeoError::Serde)
    },
        sync "coastal_storm_surge" => "SLOSH simplified storm surge model: Holland wind + wind setup + inundation" ; serde_json::json!({"type":"object","properties":{"lat":{"type":"number"},"lon":{"type":"number"},"ul_lat":{"type":"number","description":"Grid upper-left corner latitude (°)"},"ul_lon":{"type":"number","description":"Grid upper-left corner longitude (°)"},"central_pressure_hpa":{"type":"number"},"rmax_km":{"type":"number"},"forward_speed_m_s":{"type":"number"},"forward_bearing_deg":{"type":"number"},"dem":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"cell_size_m":{"type":"number"},"land_mask":{"type":"array","items":{"type":"boolean"}}},"required":["dem","cols","rows","cell_size_m","land_mask","ul_lat","ul_lon"]}) => |args| -> ToolResult {
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
        let ul_lat=args["ul_lat"].as_f64().unwrap_or(30.0);
        let ul_lon=args["ul_lon"].as_f64().unwrap_or(120.0);
        let land_mask:Vec<bool>=args["land_mask"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_bool()).collect();
        let result=crate::CoastalPlugin::new().storm_surge(&params,&dem,rows,cols,cell_size_m,&land_mask,ul_lat,ul_lon)?;
        serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
    },
        sync "coastal_blue_carbon" => "Blue carbon stock & sequestration: mangrove/salt-marsh/seagrass" ; serde_json::json!({"type":"object","properties":{"ecosystem":{"type":"string","enum":["mangrove","salt_marsh","seagrass"]},"area_ha":{"type":"number"},"soil_factor":{"type":"number"}},"required":["ecosystem","area_ha"]}) => |args| -> ToolResult {
        let eco=args["ecosystem"].as_str().unwrap_or("mangrove");
        let area=args["area_ha"].as_f64().unwrap_or(1.0);
        let sf=args["soil_factor"].as_f64().unwrap_or(1.0);
        let r=crate::CoastalPlugin::new().assess_blue_carbon(eco,area,sf)?;
        serde_json::to_value(r).map_err(geo_core::GeoError::Serde)
    },
        sync "coastal_blue_carbon_aggregate" => "Aggregate multiple blue carbon assessments" ; serde_json::json!({"type":"object","properties":{"items":{"type":"array","items":{"type":"object","properties":{"ecosystem":{"type":"string","enum":["mangrove","salt_marsh","seagrass"]},"area_ha":{"type":"number"},"soil_factor":{"type":"number"}},"required":["ecosystem","area_ha"]}}},"required":["items"]}) => |args| -> ToolResult {
        let items: Vec<BlueCarbonResult> = args["items"].as_array().unwrap_or(&vec![]).iter().map(|v| {
            let eco=v["ecosystem"].as_str().unwrap_or("mangrove");
            let area=v["area_ha"].as_f64().unwrap_or(1.0);
            let sf=v["soil_factor"].as_f64().unwrap_or(1.0);
            crate::CoastalPlugin::new().assess_blue_carbon(eco,area,sf)
        }).filter_map(|r| r.ok()).collect();
        let agg=crate::CoastalPlugin::new().aggregate_blue_carbon(items);
        serde_json::to_value(agg).map_err(geo_core::GeoError::Serde)
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
    },
        sync "coastal_slr_scenario" => "IPCC AR6 SLR level for a given scenario and year" ; serde_json::json!({"type":"object","properties":{"scenario":{"type":"string","enum":["SSP1-1.9","SSP1-2.6","SSP2-4.5","SSP3-7.0","SSP5-8.5"]},"year":{"type":"integer","minimum":2020,"maximum":2150}},"required":["scenario","year"]}) => |args| -> ToolResult {
        let sc=args["scenario"].as_str().unwrap_or("SSP2-4.5");
        let yr=args["year"].as_u64().unwrap_or(2100) as u16;
        let level=crate::slr::slr_scenario_level(sc,yr);
        Ok(serde_json::json!({"scenario":sc,"year":yr,"sea_level_rise_m":(level*100.0).round()/100.0}))
    },
        sync "coastal_slr_inundation" => "Bathtub inundation from SLR and DEM" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"cell_size_m":{"type":"number"},"slr_m":{"type":"number"},"tidal_range_m":{"type":"number"}},"required":["dem","cols","rows","cell_size_m","slr_m"]}) => |args| -> ToolResult {
        let dem:Vec<f64>=args["dem"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let c=args["cols"].as_u64().unwrap_or(1) as usize;
        let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let cs=args["cell_size_m"].as_f64().unwrap_or(30.0);
        let slr=args["slr_m"].as_f64().unwrap_or(0.5);
        let tr=args["tidal_range_m"].as_f64().unwrap_or(0.0);
        let result=crate::slr::slr_inundation_area(&dem,c,r,cs,slr,tr);
        Ok(result)
    },
        sync "coastal_slr_impact" => "Comprehensive SLR impact assessment" ; serde_json::json!({"type":"object","properties":{"dem":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"cell_size_m":{"type":"number"},"scenario":{"type":"string"},"year":{"type":"integer"}},"required":["dem","cols","rows","cell_size_m","scenario","year"]}) => |args| -> ToolResult {
        let dem:Vec<f64>=args["dem"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let c=args["cols"].as_u64().unwrap_or(1) as usize;
        let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let cs=args["cell_size_m"].as_f64().unwrap_or(30.0);
        let sc=args["scenario"].as_str().unwrap_or("SSP2-4.5");
        let yr=args["year"].as_u64().unwrap_or(2100) as u16;
        let result=crate::slr::slr_coastal_impact(&dem,c,r,cs,sc,yr);
        Ok(result)
    },
        sync "coastal_slr_erosion" => "Bruun Rule erosion from sea level rise" ; serde_json::json!({"type":"object","properties":{"slr_m":{"type":"number"},"shoreline_length_km":{"type":"number"},"beach_slope_pct":{"type":"number"},"closure_depth_m":{"type":"number"}},"required":["slr_m","shoreline_length_km","beach_slope_pct","closure_depth_m"]}) => |args| -> ToolResult {
        let slr=args["slr_m"].as_f64().unwrap_or(0.5);
        let len=args["shoreline_length_km"].as_f64().unwrap_or(10.0);
        let slope=args["beach_slope_pct"].as_f64().unwrap_or(3.0);
        let cd=args["closure_depth_m"].as_f64().unwrap_or(10.0);
        let result=crate::slr::slr_erosion_impact(slr,len,slope,cd);
        Ok(result)
    },
        sync "coastal_cvi_calculate" => "Coastal Vulnerability Index (Gornitz 1991)" ; serde_json::json!({"type":"object","properties":{"geomorphology":{"type":"string","enum":["rocky","medium_cliff","low_cliff","cobble_beach","sandy_beach"]},"shoreline_change_m_yr":{"type":"number"},"coastal_slope_pct":{"type":"number"},"slr_mm_yr":{"type":"number"},"wave_height_m":{"type":"number"},"tidal_range_m":{"type":"number"}},"required":["geomorphology","shoreline_change_m_yr","coastal_slope_pct","slr_mm_yr","wave_height_m","tidal_range_m"]}) => |args| -> ToolResult {
        let geo=args["geomorphology"].as_str().unwrap_or("sandy_beach");
        let sh=args["shoreline_change_m_yr"].as_f64().unwrap_or(0.0);
        let sl=args["coastal_slope_pct"].as_f64().unwrap_or(5.0);
        let sr=args["slr_mm_yr"].as_f64().unwrap_or(3.0);
        let wh=args["wave_height_m"].as_f64().unwrap_or(1.0);
        let tr=args["tidal_range_m"].as_f64().unwrap_or(2.0);
        let result=crate::cvi::cvi_calculate(geo,sh,sl,sr,wh,tr);
        Ok(result)
    }]);
}
