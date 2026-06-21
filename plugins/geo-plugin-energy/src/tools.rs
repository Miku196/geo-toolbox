//! Tool registration — Energy plugin.
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "energy", "Solar/wind/geothermal/transmission/PV site assessment", PluginCategory::Process, [
        sync "energy_solar_suitability" => "Assess solar site suitability from DEM + radiation" ; serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"dem_data":{"type":"array","items":{"type":"number"}},"radiation_data":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"nodata":{"type":"number"}},"required":["aoi_name","aoi_geojson","dem_data","radiation_data","cols","rows"]}) => |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0); let c=args["cols"].as_u64().unwrap_or(1) as usize; let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
        let result=crate::EnergyPlugin::new(Default::default()).assess_solar(args["aoi_name"].as_str().unwrap_or(""),args["aoi_geojson"].as_str().unwrap_or(""),&mk("dem_data","dem"),&mk("radiation_data","rad"))?;
        serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
    },
        sync "energy_geothermal" => "Geothermal power potential: heat flux → MW, LCOE" ; serde_json::json!({"type":"object","properties":{"name":{"type":"string"},"heat_flux_mw_m2":{"type":"number"},"area_km2":{"type":"number"},"surface_temp_c":{"type":"number"}},"required":["name","heat_flux_mw_m2","area_km2"]}) => |args| -> ToolResult {
        let hf=args["heat_flux_mw_m2"].as_f64().unwrap_or(80.0);
        let area=args["area_km2"].as_f64().unwrap_or(1.0);
        let st=args["surface_temp_c"].as_f64().unwrap_or(15.0);
        let result=crate::geothermal::GeothermalAssessment::from_heat_flux(args["name"].as_str().unwrap_or("site"),hf,area,st);
        serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
    },
        sync "energy_geothermal_gradient" => "Geothermal from temperature gradient + conductivity" ; serde_json::json!({"type":"object","properties":{"name":{"type":"string"},"gradient_c_per_km":{"type":"number"},"conductivity":{"type":"number"},"area_km2":{"type":"number"},"surface_temp_c":{"type":"number"}},"required":["name","gradient_c_per_km","area_km2"]}) => |args| -> ToolResult {
        let grad=args["gradient_c_per_km"].as_f64().unwrap_or(30.0);
        let cond=args["conductivity"].as_f64().unwrap_or(2.5);
        let area=args["area_km2"].as_f64().unwrap_or(1.0);
        let st=args["surface_temp_c"].as_f64().unwrap_or(15.0);
        let result=crate::geothermal::GeothermalAssessment::from_gradient(args["name"].as_str().unwrap_or("site"),grad,cond,area,st);
        serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
    },
        sync "energy_transmission_corridor" => "Least-cost path for power transmission corridor" ; serde_json::json!({"type":"object","properties":{"name":{"type":"string"},"source_name":{"type":"string"},"sink_name":{"type":"string"},"cost_surface":{"type":"array","items":{"type":"number"}},"nrows":{"type":"integer"},"ncols":{"type":"integer"},"start_idx":{"type":"integer"},"end_idx":{"type":"integer"},"cell_size_m":{"type":"number"},"corridor_width_m":{"type":"number"}},"required":["name","cost_surface","nrows","ncols","start_idx","end_idx"]}) => |args| -> ToolResult {
        let cs:Vec<f64>=args["cost_surface"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let nr=args["nrows"].as_u64().unwrap_or(1) as usize;
        let nc=args["ncols"].as_u64().unwrap_or(1) as usize;
        let si=args["start_idx"].as_u64().unwrap_or(0) as usize;
        let ei=args["end_idx"].as_u64().unwrap_or(0) as usize;
        let csm=args["cell_size_m"].as_f64().unwrap_or(1000.0);
        let cw=args["corridor_width_m"].as_f64().unwrap_or(100.0);
        let result=crate::transmission::assess_corridor(
            args["name"].as_str().unwrap_or("corridor"),
            args["source_name"].as_str().unwrap_or("source"),
            args["sink_name"].as_str().unwrap_or("sink"),
            &cs,nr,nc,si,ei,csm,cw,crate::transmission::DEFAULT_COST_PER_KM
        ).ok_or_else(|| geo_core::GeoError::Validation("不可达或无效路径".into()))?;
        serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
    },
        sync "energy_pvwatts_annual" => "PVWatts v5 annual energy estimation" ; serde_json::json!({"type":"object","properties":{"monthly_poa":{"type":"array","items":{"type":"number"}},"module_capacity_kw":{"type":"number"},"monthly_temp":{"type":"array","items":{"type":"number"}},"monthly_wind":{"type":"array","items":{"type":"number"}},"mounting":{"type":"string","enum":["open_rack","roof_mount","insulated"]},"inverter_eff":{"type":"number"},"dc_ac_ratio":{"type":"number"},"temp_coeff":{"type":"number"},"losses_pct":{"type":"number"}},"required":["monthly_poa","module_capacity_kw"]}) => |args| -> ToolResult {
        let poa:Vec<f64>=args["monthly_poa"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let cap=args["module_capacity_kw"].as_f64().unwrap_or(1.0);
        let tmp:Vec<f64>=args["monthly_temp"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let wnd:Vec<f64>=args["monthly_wind"].as_array().unwrap_or(&vec![]).iter().filter_map(|x|x.as_f64()).collect();
        let mt=args["mounting"].as_str().unwrap_or("open_rack");
        let ie=args["inverter_eff"].as_f64().unwrap_or(0.96);
        let dar=args["dc_ac_ratio"].as_f64().unwrap_or(1.1);
        let tc=args["temp_coeff"].as_f64().unwrap_or(-0.35);
        let lp=args["losses_pct"].as_f64().unwrap_or(14.0);
        let result=crate::pvwatts::pvwatts_annual_energy(&poa,cap,&tmp,&wnd,mt,ie,dar,tc,lp);
        Ok(result)
    },
        sync "energy_pvwatts_cell_temp" => "PV cell temperature from Sandia model" ; serde_json::json!({"type":"object","properties":{"poa_irradiance_w_m2":{"type":"number"},"ambient_temp_c":{"type":"number"},"wind_speed_m_s":{"type":"number"},"mounting":{"type":"string"}},"required":["poa_irradiance_w_m2","ambient_temp_c"]}) => |args| -> ToolResult {
        let g=args["poa_irradiance_w_m2"].as_f64().unwrap_or(1000.0);
        let ta=args["ambient_temp_c"].as_f64().unwrap_or(25.0);
        let ws=args["wind_speed_m_s"].as_f64().unwrap_or(0.0);
        let mt=args["mounting"].as_str().unwrap_or("open_rack");
        let tc=crate::pvwatts::pvwatts_cell_temperature(g,ta,ws,mt);
        Ok(serde_json::json!({"cell_temperature_c":(tc*100.0).round()/100.0}))
    }]);
}
