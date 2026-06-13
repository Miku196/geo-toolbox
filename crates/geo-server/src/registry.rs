//! Registry setup — mirrors geo-cli build_registry with lightweight adapters only.
use geo_registry::PluginRegistry;

pub fn build_registry() -> PluginRegistry {
    let mut reg = PluginRegistry::new();

    // ── Core: CRS + Ingest + Spatial ops ──
    geo_io::tools::register_tools(&mut reg);
    geo_carbon_math::tools::register_tools(&mut reg);
    geo_tile::tools::register_tools(&mut reg);
    geo_temporal::tools::register_tools(&mut reg);
    geo_vector::tools::register_tools(&mut reg);
    geo_index::tools::register_tools(&mut reg);
    geo_stats::tools::register_tools(&mut reg);
    geo_report::tools::register_tools(&mut reg);

    // ── Plugins ──
    geo_plugin_carbon::tools::register_tools(&mut reg);
    geo_plugin_ecology::tools::register_tools(&mut reg);
    geo_plugin_energy::tools::register_tools(&mut reg);
    geo_plugin_forestry::tools::register_tools(&mut reg);
    geo_plugin_coastal::tools::register_tools(&mut reg);
    geo_plugin_survey::tools::register_tools(&mut reg);
    geo_plugin_hydro::tools::register_tools(&mut reg);
    geo_plugin_geohazard::tools::register_tools(&mut reg);
    geo_plugin_agri::tools::register_tools(&mut reg);
    geo_plugin_urban::tools::register_tools(&mut reg);

    // ── Adapters: lightweight (always-on) ──
    geo_adapter_duckdb::tools::register_tools(&mut reg);
    geo_adapter_stac::tools::register_tools(&mut reg);
    geo_adapter_osm::tools::register_tools(&mut reg);

    reg
}
