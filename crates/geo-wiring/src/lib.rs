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
    geo_plugin_climate::tools::register_tools(reg);
    geo_plugin_geomorph::tools::register_tools(reg);
    geo_plugin_remote_sensing::tools::register_tools(reg);
    geo_plugin_seismology::tools::register_tools(reg);
    geo_plugin_socioeconomic::tools::register_tools(reg);
    geo_plugin_atmosphere::tools::register_tools(reg);
    geo_plugin_volcanology::tools::register_tools(reg);
    geo_plugin_groundwater::tools::register_tools(reg);

    // ── Adapters: lightweight (feature-gated I/O adapters) ──
    #[cfg(feature = "duckdb")]
    geo_adapters_io::duckdb::register_tools(reg);
    #[cfg(feature = "stac")]
    geo_adapters_io::stac::register_tools(reg);
    #[cfg(feature = "osm")]
    geo_adapters_io::osm::register_tools(reg);

    // ── Adapters: feature-gated ──
    #[cfg(feature = "postgis")]
    {
        let _ = geo_adapters_geo::postgis::register_tools(reg);
    }
    #[cfg(feature = "gee")]
    {
        geo_adapters_geo::gee::register_tools(reg);
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
        geo_adapters_io::cad::register_tools(reg);
    }
    #[cfg(feature = "gdal")]
    {
        geo_adapters_geo::gdal::register_tools(reg);
    }
    #[cfg(feature = "iot")]
    {
        let _ = geo_adapters_sim::iot::iot_tools::register_tools(reg);
    }
}

// ════════════════════════════════════════════════════════════════
// Dependency Injection: Plugin ← dyn Trait ← Adapter
//
// Wiring 层持有 adapter 的具体实现，通过 Box<dyn Trait> 注入 Plugin。
// Plugin 完全不依赖 Adapter crate — 违反者被 check-deps.sh 拦截。
// ════════════════════════════════════════════════════════════════

#[cfg(feature = "geo-adapters-sim")]
use geo_core::traits::{DssatGenerator, ModflowGenerator};

/// 实例化 DSSAT 适配器并返回 trait 对象。
/// 用于注入 `geo_plugin_agri::AgriPlugin` 构造函数。
#[cfg(feature = "geo-adapters-sim")]
fn assemble_dssat_generator() -> Box<dyn DssatGenerator> {
    Box::new(geo_adapters_sim::dssat::DssatAdapter)
}

/// 实例化 MODFLOW 适配器并返回 trait 对象。
/// 用于注入 `geo_plugin_hydro::HydroPlugin` 构造函数。
#[cfg(feature = "geo-adapters-sim")]
fn assemble_modflow_generator() -> Box<dyn ModflowGenerator> {
    Box::new(geo_adapters_sim::modflow::ModflowAdapter)
}

/// 组装完整的 AgriPlugin（含 DSSAT 模型输入文件生成能力）。
#[cfg(feature = "geo-adapters-sim")]
pub fn assemble_agri_plugin(config: geo_plugin_agri::AgriConfig) -> geo_plugin_agri::AgriPlugin {
    let dssat = assemble_dssat_generator();
    geo_plugin_agri::AgriPlugin::new(config).with_dssat_generator(dssat)
}

/// 组装完整的 HydroPlugin（含 MODFLOW 地下水模拟能力）。
#[cfg(feature = "geo-adapters-sim")]
pub fn assemble_hydro_plugin(
    config: geo_plugin_hydro::HydroConfig,
) -> geo_plugin_hydro::HydroPlugin {
    let modflow = assemble_modflow_generator();
    geo_plugin_hydro::HydroPlugin::new(config).with_modflow_generator(modflow)
}
// ════════════════════════════════════════════════════════════════
// Composition-root adapter facade consumed by geo-cli
//
// geo-cli must NOT depend on adapter crates directly (five-layer
// single-direction dependency rule).  It reaches every adapter exclusively
// through these wiring-provided re-exports.  geo-wiring remains the ONLY
// crate in the stack that depends on Plugin + Adapter.
// ════════════════════════════════════════════════════════════════

/// PostGIS adapter surface used by `geo-cli store` / `output report`.
#[cfg(feature = "postgis")]
pub mod postgis {
    pub use geo_adapters_geo::postgis::{
        dvc_available, dvc_hash, dvc_pull, dvc_snapshot, run_migrations, PostgisCarbonEngine,
        PostgisStore,
    };
}

/// GEE adapter surface used by `geo-cli process gee`.
#[cfg(feature = "gee")]
pub mod gee {
    pub use geo_adapters_geo::gee::{create_mq, GeeDispatcher, GeeTracker};
}

/// GDAL adapter surface used by `geo-cli process gdal`.
#[cfg(feature = "gdal")]
pub mod gdal {
    pub use geo_adapters_geo::gdal::{
        CogOptions, GcsBridge, GcsBridgeConfig, Ogr2OgrOptions, RasterOps, VectorOps,
    };
}

/// CAD (I/O) adapter surface used by `geo-cli output` (Geojson/Dxf/Excel).
#[cfg(feature = "cad")]
pub mod cad {
    pub use geo_adapters_io::cad::{DxfExporter, ExcelDashboard, GeoJsonExporter};
}

/// IoT / MQTT adapter surface used by `geo-cli ingest mqtt`.
#[cfg(feature = "mqtt")]
pub mod iot {
    pub use geo_adapters_sim::iot::iot_mqtt::{MqttConfig, MqttIngestor};
}

/// QGIS adapter surface used by `geo-cli process qgis`.
#[cfg(feature = "qgis")]
pub mod qgis {
    pub use geo_adapter_qgis::grpc_client::{QgisClient, QgisInput, QgisJob, QgisToolStep};
    pub use geo_adapter_qgis::process_runner::{BatchQgisRunner, QgisProcessConfig, QgisTool};
}
