//! Registry setup — delegates to geo-wiring for shared plugin registration.
use geo_wiring::{GeoConfig, PluginRegistry};

pub fn build_registry() -> PluginRegistry {
    let mut reg = PluginRegistry::new();
    let config = GeoConfig::load_default().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}");
        None
    });
    geo_wiring::populate_defaults(&mut reg, config.as_ref());
    reg
}
