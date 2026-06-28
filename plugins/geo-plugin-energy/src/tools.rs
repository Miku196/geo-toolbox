//! Tool registration — Energy plugin.
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};
pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "energy", "Solar/wind/geothermal/transmission/PV site assessment", PluginCategory::Process, [
        sync "energy_solar_suitability" => "Assess solar site suitability from DEM + radiation" ; serde_json::json!({"type":"object","properties":{"aoi_name":{"type":"string"},"aoi_geojson":{"type":"string"},"dem_data":{"type":"array","items":{"type":"number"}},"radiation_data":{"type":"array","items":{"type":"number"}},"cols":{"type":"integer"},"rows":{"type":"integer"},"nodata":{"type":"number"}},"required":["aoi_name","aoi_geojson","dem_data","radiation_data","cols","rows"]}) => |args| -> ToolResult {
        use geo_raster::RasterBand;
        let nd=args["nodata"].as_f64().unwrap_or(-999.0); let c=args["cols"].as_u64().unwrap_or(1) as usize; let r=args["rows"].as_u64().unwrap_or(1) as usize;
        let mk=|k:&str,l:&str|{let v:Vec<f64>=args[k].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|x|x.as_f64()).collect();RasterBand::new(l,c,r,v,nd)};
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
        let cs:Vec<f64>=args["cost_surface"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|x|x.as_f64()).collect();
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
        let poa:Vec<f64>=args["monthly_poa"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|x|x.as_f64()).collect();
        let cap=args["module_capacity_kw"].as_f64().unwrap_or(1.0);
        let tmp:Vec<f64>=args["monthly_temp"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|x|x.as_f64()).collect();
        let wnd:Vec<f64>=args["monthly_wind"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().filter_map(|x|x.as_f64()).collect();
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
    },
        sync "energy_turbine_power" => "Turbine power output at given wind speed with air density" ; serde_json::json!({"type":"object","properties":{"wind_speed_ms":{"type":"number"},"altitude_m":{"type":"number","default":0},"turbine":{"type":"string","default":"V80","enum":["V80","V164","G114"]}},"required":["wind_speed_ms"]}) => |args| -> ToolResult {
        let ws=args["wind_speed_ms"].as_f64().unwrap_or(0.0);
        let alt=args["altitude_m"].as_f64().unwrap_or(0.0);
        let rho=crate::turbine::air_density(alt);
        let params=match args["turbine"].as_str().unwrap_or("V80") {
            "V164" => crate::turbine::TurbineParams::vestas_v164(),
            "G114" => crate::turbine::TurbineParams::gamesa_g114(),
            _ => crate::turbine::TurbineParams::vestas_v80(),
        };
        let pc=crate::turbine::power_curve(&params,ws,rho);
        Ok(serde_json::json!({"actual_power_w":(pc.actual_power_w*100.0).round()/100.0,"betz_power_w":(pc.betz_power_w*100.0).round()/100.0,"capacity_factor":(pc.capacity_factor*10000.0).round()/100.0,"rated_power_kw":params.rated_power_w/1000.0,"cut_in_ms":params.cut_in_v,"cut_out_ms":params.cut_out_v,"air_density":(rho*10000.0).round()/10000.0,"swept_area_m2":pc.swept_area_m2}))
    },
        sync "energy_turbine_aep" => "Annual energy from Weibull distribution (k=shape, c=scale)" ; serde_json::json!({"type":"object","properties":{"weibull_k":{"type":"number","default":2.0},"weibull_c":{"type":"number","default":8.0},"altitude_m":{"type":"number","default":0},"turbine":{"type":"string","default":"V80"},"hours_per_year":{"type":"number","default":8760}},"required":["weibull_k","weibull_c"]}) => |args| -> ToolResult {
        let wk=args["weibull_k"].as_f64().unwrap_or(2.0);
        let wc=args["weibull_c"].as_f64().unwrap_or(8.0);
        let alt=args["altitude_m"].as_f64().unwrap_or(0.0);
        let hpy=args["hours_per_year"].as_f64().unwrap_or(8760.0);
        let params=match args["turbine"].as_str().unwrap_or("V80") {
            "V164" => crate::turbine::TurbineParams::vestas_v164(),
            "G114" => crate::turbine::TurbineParams::gamesa_g114(),
            _ => crate::turbine::TurbineParams::vestas_v80(),
        };
        let rho=crate::turbine::air_density(alt);
        let kwh=crate::turbine::annual_energy_production(&params,rho,wk,wc,hpy);
        Ok(serde_json::json!({"aep_kwh":(kwh*100.0).round()/100.0,"aep_mwh":(kwh/1000.0*100.0).round()/100.0}))
    },
        sync "energy_jensen_wake" => "Jensen single wake model: downwind wind speed deficit" ; serde_json::json!({"type":"object","properties":{"free_stream_wind_ms":{"type":"number"},"ct":{"type":"number","default":0.8},"rotor_radius_m":{"type":"number","default":40},"distance_m":{"type":"number","description":"Downwind distance from turbine"},"wake_decay_k":{"type":"number","default":0.075}},"required":["free_stream_wind_ms","distance_m"]}) => |args| -> ToolResult {
        let v0=args["free_stream_wind_ms"].as_f64().unwrap_or(0.0);
        let ct=args["ct"].as_f64().unwrap_or(0.8);
        let rr=args["rotor_radius_m"].as_f64().unwrap_or(40.0);
        let d=args["distance_m"].as_f64().unwrap_or(500.0);
        let k=args["wake_decay_k"].as_f64().unwrap_or(0.075);
        let v=crate::wake::jensen_wake(v0,ct,rr,d,k);
        let r=crate::wake::wake_radius(rr,d,k);
        let deficit=if v0>0.0{(v0-v)/v0*100.0}else{0.0};
        Ok(serde_json::json!({"downstream_wind_ms":(v*100.0).round()/100.0,"wake_radius_m":(r*100.0).round()/100.0,"speed_deficit_pct":(deficit*100.0).round()/100.0}))
    },
        sync "energy_farm_wake_efficiency" => "Farm-level wake efficiency for wind farm layout" ; serde_json::json!({"type":"object","properties":{"turbine_positions":{"type":"array","items":{"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4,"description":"[[x_m,y_m,ct,radius_m],...]"}},"wind_speed_ms":{"type":"number","default":10},"wind_direction_deg":{"type":"number","default":270},"wake_decay_k":{"type":"number","default":0.075},"rho":{"type":"number","default":1.225},"cp":{"type":"number","default":0.45},"spacing_m":{"type":"number","default":2000}},"required":["turbine_positions"]}) => |args| -> ToolResult {
        let v0=args["wind_speed_ms"].as_f64().unwrap_or(10.0);
        let wdir=args["wind_direction_deg"].as_f64().unwrap_or(270.0);
        let k=args["wake_decay_k"].as_f64().unwrap_or(0.075);
        let rho=args["rho"].as_f64().unwrap_or(1.225);
        let cp=args["cp"].as_f64().unwrap_or(0.45);
        let sp=args["spacing_m"].as_f64().unwrap_or(2000.0);
        let turbines:Vec<(f64,f64,f64,f64)>=args["turbine_positions"]
            .as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter()
            .filter_map(|v|{let a=v.as_array()?;Some((a.first()?.as_f64()?,a.get(1)?.as_f64()?,a.get(2)?.as_f64()?,a.get(3)?.as_f64()?))})
            .collect();
        if turbines.is_empty(){return Ok(serde_json::json!({"error":"no turbines"}))}
        let (eff,powers)=crate::wake::farm_wake_efficiency(&turbines,v0,wdir,k,&crate::wake::WakeSummation::Linear,rho,cp,sp);
        let powers_json:Vec<serde_json::Value>=powers.iter().map(|p|serde_json::json!((p*100.0).round()/100.0)).collect();
        Ok(serde_json::json!({"farm_efficiency":(eff*10000.0).round()/100.0,"total_power_w":(powers.iter().sum::<f64>()*100.0).round()/100.0,"turbine_powers":powers_json}))
    },
        sync "energy_wind_shear" => "Wind shear: extrapolate wind speed to hub height" ; serde_json::json!({"type":"object","properties":{"wind_speed_ref_ms":{"type":"number"},"height_ref_m":{"type":"number","default":10},"height_hub_m":{"type":"number","default":80},"method":{"type":"string","default":"log","enum":["log","power"]},"roughness_length_m":{"type":"number","default":0.03},"shear_exponent":{"type":"number","default":0.14}},"required":["wind_speed_ref_ms"]}) => |args| -> ToolResult {
        let v=args["wind_speed_ref_ms"].as_f64().unwrap_or(0.0);
        let zr=args["height_ref_m"].as_f64().unwrap_or(10.0);
        let zh=args["height_hub_m"].as_f64().unwrap_or(80.0);
        let method=args["method"].as_str().unwrap_or("log");
        let result=if method=="power"{
            let a=args["shear_exponent"].as_f64().unwrap_or(0.14);
            crate::turbine::wind_shear_power(v,zr,zh,a)
        }else{
            let z0=args["roughness_length_m"].as_f64().unwrap_or(0.03);
            crate::turbine::wind_shear_log(v,zr,zh,z0)
        };
        Ok(serde_json::json!({"wind_speed_hub_ms":(result*100.0).round()/100.0,"method":method}))
    },
    ]);
}
