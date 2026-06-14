//! Registry setup — delegates to geo-wiring for shared plugin registration.
use geo_wiring::PluginRegistry;

pub fn build_registry() -> PluginRegistry {
    let mut reg = PluginRegistry::new();
    geo_wiring::populate_defaults(&mut reg);
    reg
}
