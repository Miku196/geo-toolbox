use geo_core::plugin::PluginCategory;
use geo_registry::registry::ToolResult;
use geo_registry::{register_plugin, PluginRegistry};

use crate::coordinates::{celestial_to_geographic, lunar_coordinate_transform, PlanetaryFrame};
use crate::solar::solar_elevation_azimuth;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "planetary", "Planetary astronomy: coordinate transforms, solar position, extraterrestrial radiation", PluginCategory::Process, [
        sync "planetary_coordinate_transform" => "Transform coordinates between planetary frames (Lunar MEP, Mars2000, Earth ITRF, J2000)" ; serde_json::json!({"type":"object","properties":{"lon_deg":{"type":"number"},"lat_deg":{"type":"number"},"altitude_km":{"type":"number","default":0.0},"from_frame":{"type":"string","enum":["Earth_ITRF","Lunar_MEP","Mars2000","J2000"],"default":"Earth_ITRF"},"to_frame":{"type":"string","enum":["Earth_ITRF","Lunar_MEP","Mars2000","J2000"],"default":"Earth_ITRF"}},"required":["lon_deg","lat_deg"]}) => |args| -> ToolResult {
            let lon = args["lon_deg"].as_f64().unwrap_or(0.0);
            let lat = args["lat_deg"].as_f64().unwrap_or(0.0);
            let alt = args["altitude_km"].as_f64().unwrap_or(0.0);
            let from_s = args["from_frame"].as_str().unwrap_or("Earth_ITRF");
            let to_s = args["to_frame"].as_str().unwrap_or("Earth_ITRF");
            let from = PlanetaryFrame::from_str(from_s).unwrap_or(PlanetaryFrame::Earth);
            let to = PlanetaryFrame::from_str(to_s).unwrap_or(PlanetaryFrame::Earth);
            let r = lunar_coordinate_transform(lon, lat, alt, from, to)
                .ok_or_else(|| geo_core::GeoError::invalid_input("from_frame", format!("Unsupported coordinate transform: {from_s} → {to_s}")))?;
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "planetary_solar_position" => "Solar elevation, azimuth, declination, extraterrestrial radiation" ; serde_json::json!({"type":"object","properties":{"lat_deg":{"type":"number"},"lon_deg":{"type":"number"},"day_of_year":{"type":"integer","minimum":1,"maximum":366},"utc_hour":{"type":"number","default":12.0}},"required":["lat_deg","lon_deg","day_of_year"]}) => |args| -> ToolResult {
            let lat = args["lat_deg"].as_f64().unwrap_or(0.0);
            let lon = args["lon_deg"].as_f64().unwrap_or(0.0);
            let doy = args["day_of_year"].as_u64().unwrap_or(1) as u16;
            let utc = args["utc_hour"].as_f64().unwrap_or(12.0);
            let r = solar_elevation_azimuth(lat, lon, doy, utc);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },

        sync "planetary_celestial_to_geo" => "Convert J2000 celestial coordinates (RA, Dec) to Earth geographic coordinates" ; serde_json::json!({"type":"object","properties":{"ra_deg":{"type":"number","description":"Right ascension in degrees"},"dec_deg":{"type":"number","description":"Declination in degrees"},"distance_au":{"type":"number","default":1.0,"description":"Distance in AU"}},"required":["ra_deg","dec_deg"]}) => |args| -> ToolResult {
            let ra = args["ra_deg"].as_f64().unwrap_or(0.0);
            let dec = args["dec_deg"].as_f64().unwrap_or(0.0);
            let dist = args["distance_au"].as_f64().unwrap_or(1.0);
            let r = celestial_to_geographic(ra, dec, dist);
            serde_json::to_value(r).map_err(geo_core::errors::GeoError::Serde)
        },
    ]);
}
