use geo_core::plugin::PluginCategory;
use geo_registry::registry::{ToolDef, ToolResult};
use geo_registry::PluginRegistry;

/// Register agri tools into the PluginRegistry.
pub fn register_tools(registry: &mut PluginRegistry) {
    let _plugin = crate::AgriPlugin::new(Default::default());

    registry.register(geo_core::plugin::PluginMeta {
        name: "agri".into(),
        version: "0.1.0".into(),
        description: "Agriculture: crop yield, LAI/NPP, soil rating, irrigation".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });

    // ── 1. agri_yield ──
    registry.register_tool_sync("agri", ToolDef {
        name: "agri_yield".into(),
        description: "Estimate crop yield from NDVI and area. Supports wheat/corn/rice/soybean.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "area_ha": {"type": "number", "description": "Field area in hectares"},
                "ndvi_mean": {"type": "number", "description": "Mean NDVI (0..1)"},
                "crop_type": {"type": "string", "enum": ["wheat", "corn", "rice", "soybean"], "default": "wheat"}
            },
            "required": ["area_ha", "ndvi_mean"]
        }),
    }, |args| -> ToolResult {
        let area = args["area_ha"].as_f64().unwrap_or(10.0);
        let ndvi = args["ndvi_mean"].as_f64().unwrap_or(0.7);
        let crop = args["crop_type"].as_str().unwrap_or("wheat");
        let result = crate::AgriPlugin::new(Default::default()).estimate_yield(area, ndvi, crop);
        Ok(serde_json::json!({
            "yield_kg": result.yield_kg,
            "yield_casa_kg": result.yield_casa_kg,
            "yield_simple_kg": result.yield_simple_kg,
            "lai": result.lai,
            "npp_gcm2_season": result.npp_gcm2_season,
            "crop_type": result.crop_type,
            "area_ha": result.area_ha,
        }))
    });

    // ── 2. agri_soil ──
    registry.register_tool_sync("agri", ToolDef {
        name: "agri_soil".into(),
        description: "Comprehensive soil quality rating (0-100) with N/P/K/texture analysis.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "organic_matter_pct": {"type": "number", "description": "Soil organic matter (%)"},
                "ph": {"type": "number", "description": "Soil pH"},
                "n_mg_kg": {"type": "number", "description": "Available nitrogen (mg/kg)", "default": 100},
                "p_mg_kg": {"type": "number", "description": "Available phosphorus (mg/kg)", "default": 15},
                "k_mg_kg": {"type": "number", "description": "Available potassium (mg/kg)", "default": 120},
                "texture": {"type": "string", "enum": ["loam", "clay_loam", "sandy_loam", "silt_loam", "clay", "sand", "silt"], "default": "loam"},
                "drainage_ok": {"type": "boolean", "default": true}
            },
            "required": ["organic_matter_pct", "ph"]
        }),
    }, |args| -> ToolResult {
        let p = crate::AgriPlugin::new(Default::default());
        let result = p.soil_rating_detailed(
            args["organic_matter_pct"].as_f64().unwrap_or(2.0),
            args["ph"].as_f64().unwrap_or(6.5),
            args["n_mg_kg"].as_f64().unwrap_or(100.0),
            args["p_mg_kg"].as_f64().unwrap_or(15.0),
            args["k_mg_kg"].as_f64().unwrap_or(120.0),
            args["texture"].as_str().unwrap_or("loam"),
            args["drainage_ok"].as_bool().unwrap_or(true),
        );
        Ok(serde_json::json!({
            "score": result.score,
            "grade": result.grade,
            "om_score": result.om_score,
            "ph_score": result.ph_score,
            "n_score": result.n_score,
            "p_score": result.p_score,
            "k_score": result.k_score,
            "texture_score": result.texture_score,
            "drainage_score": result.drainage_score,
        }))
    });

    // ── 3. agri_lai ──
    registry.register_tool_sync("agri", ToolDef {
        name: "agri_lai".into(),
        description: "NDVI → LAI → fIPAR → NPP conversion chain.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "ndvi": {"type": "number", "description": "NDVI value (0..1)"},
                "crop_type": {"type": "string", "enum": ["wheat", "corn", "rice", "soybean"], "default": "wheat"},
                "par_mj_m2_day": {"type": "number", "description": "Incident PAR (MJ/m²/day)", "default": 20}
            },
            "required": ["ndvi"]
        }),
    }, |args| -> ToolResult {
        let p = crate::AgriPlugin::new(Default::default());
        let ndvi = args["ndvi"].as_f64().unwrap_or(0.7);
        let crop = args["crop_type"].as_str().unwrap_or("wheat");
        let par = args["par_mj_m2_day"].as_f64().unwrap_or(20.0);
        let k = p.config.crops.get(crop).map(|c| c.k).unwrap_or(0.5);
        let lai = p.ndvi_to_lai(ndvi, k);
        let fipar = crate::AgriPlugin::lai_to_fipar(lai);
        let npp = p.estimate_npp(par, ndvi, crop);
        Ok(serde_json::json!({
            "lai": lai,
            "fipar": fipar,
            "npp_gcm2_day": npp / 120.0,
            "npp_gcm2_season": npp,
            "crop_type": crop,
            "k": k,
        }))
    });

    // ── 4. agri_irrigation ──
    registry.register_tool_sync("agri", ToolDef {
        name: "agri_irrigation".into(),
        description: "Net & gross irrigation requirement from ET₀, rainfall, and crop type.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "et0_mm_day": {"type": "number", "description": "Reference evapotranspiration (mm/day)"},
                "rainfall_mm": {"type": "number", "description": "Rainfall (mm)"},
                "crop_type": {"type": "string", "enum": ["wheat", "corn", "rice", "soybean"], "default": "wheat"}
            },
            "required": ["et0_mm_day"]
        }),
    }, |args| -> ToolResult {
        let p = crate::AgriPlugin::new(Default::default());
        let et0 = args["et0_mm_day"].as_f64().unwrap_or(5.0);
        let rain = args["rainfall_mm"].as_f64().unwrap_or(0.0);
        let crop = args["crop_type"].as_str().unwrap_or("wheat");
        let net = p.net_irrigation(et0, crop, rain);
        let gross = p.gross_irrigation(net);
        Ok(serde_json::json!({
            "net_irrigation_mm": net,
            "gross_irrigation_mm": gross,
            "kc": p.config.crops.get(crop).map(|c| c.kc),
            "crop_type": crop,
        }))
    });
}
