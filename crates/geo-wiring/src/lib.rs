//! Shared registry wiring for geo-toolbox entry points (geo-cli, geo-server).
//!
//! Registers all core crates, plugins, and feature-gated adapters
//! into a PluginRegistry. Both CLI and HTTP server call `populate_defaults()`.

pub use geo_core::config::GeoConfig;
pub use geo_registry::PluginRegistry;

/// Register all Core + Plugin + lightweight Adapter tools.
///
/// Heavy adapters (PostGIS, GEE, QGIS, CAD, CLI, IoT) are registered only
/// when the corresponding feature flag is active.
///
/// Callers should then add any remaining custom adapters.
pub fn populate_defaults(reg: &mut PluginRegistry, config: Option<&GeoConfig>) {
    let _ = config;
    // ── Core: CRS + Ingest + Spatial ops ──
    geo_io::tools::register_tools(reg);
    geo_carbon_math::tools::register_tools(reg);
    geo_tile::tools::register_tools(reg);
    geo_temporal::tools::register_tools(reg);
    geo_vector::tools::register_tools(reg);
    geo_index::tools::register_tools(reg);
    geo_stats::tools::register_tools(reg);
    geo_report::tools::register_tools(reg);

    // ── Plugins ──
    geo_plugin_carbon::tools::register_tools(reg);
    geo_plugin_ecology::tools::register_tools(reg);
    geo_plugin_energy::tools::register_tools(reg);
    geo_plugin_forestry::tools::register_tools(reg);
    geo_plugin_coastal::tools::register_tools(reg);
    geo_plugin_survey::tools::register_tools(reg);
    geo_plugin_hydro::tools::register_tools(reg);
    geo_plugin_geohazard::tools::register_tools(reg);
    geo_plugin_agri::tools::register_tools(reg);
    geo_plugin_urban::tools::register_tools(reg);

    // ── Adapters: lightweight (always-on) ──
    geo_adapter_duckdb::tools::register_tools(reg);
    geo_adapter_stac::tools::register_tools(reg);
    geo_adapter_osm::tools::register_tools(reg);

    // ── Adapters: feature-gated ──
    #[cfg(feature = "postgis")]
    {
        let _ = geo_adapter_postgis::tools::register_tools(reg);
    }
    #[cfg(feature = "gee")]
    {
        geo_adapter_gee::tools::register_tools(reg);
    }
    #[cfg(feature = "qgis")]
    {
        if let Some(cfg) = config {
            if cfg.adapters.qgis.enabled {
                let path = &cfg.adapters.qgis.qgis_process_path;
                if !path.is_empty() && std::env::var("QGIS_PROCESS_PATH").is_err() {
                    std::env::set_var("QGIS_PROCESS_PATH", path);
                }
            }
        }
        geo_adapter_qgis::tools::register_tools(reg);
    }
    #[cfg(feature = "cad")]
    {
        geo_adapter_cad::tools::register_tools(reg);
    }
    #[cfg(feature = "gdal")]
    {
        geo_adapter_cli::tools::register_tools(reg);
    }
    #[cfg(feature = "iot")]
    {
        let _ = geo_adapter_iot::tools::register_tools(reg);
    }
}
