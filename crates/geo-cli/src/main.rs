//! geo-toolbox CLI entry point.
//!
//! Maps each subcommand to the corresponding handler module under `commands/`.

use clap::{Parser, Subcommand};
use geo_core::plugin::PluginCategory;
use geo_registry::PluginRegistry;

mod commands;
mod mcp;

#[derive(Parser)]
#[command(
    name = "geo-toolbox",
    version = env!("CARGO_PKG_VERSION"),
    about = "Rust geospatial pipeline toolbox",
    long_about = "High-performance glue layer for PostGIS, GEE, QGIS, and carbon accounting.\n\
                  Driven by Pi Agent via MCP or invoked directly from the command line."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// CRS registry: list, transform, register coordinate systems
    Crs {
        #[command(subcommand)]
        action: CrsAction,
    },

    /// Start the MCP (Model Context Protocol) server for Pi Agent integration
    McpServe {
        /// Port to listen on (default 9378 over stdio)
        #[arg(long, default_value = "9378")]
        port: u16,
    },

    /// Data ingestion: CamoFox, NMEA, MQTT
    Ingest {
        #[command(subcommand)]
        action: IngestAction,
    },

    /// Data storage: PostGIS, TimescaleDB, DVC, MinIO
    Store {
        #[command(subcommand)]
        action: StoreAction,
    },

    /// Geoprocessing: GEE dispatcher, GCS bridge, QGIS, GDAL
    #[command(subcommand)]
    Process(ProcessAction),

    /// Carbon accounting: emission factor, LCA, carbon sink
    #[command(subcommand)]
    Carbon(CarbonAction),

    /// Output: Excel dashboard, DXF, GeoJSON, reports
    #[command(subcommand)]
    Output(OutputAction),

    /// Plugin registry: list plugins, check health
    Plugins {
        #[command(subcommand)]
        action: PluginsAction,
    },
}

#[derive(Subcommand)]
enum PluginsAction {
    /// List all registered plugins and adapters
    List {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
    },
    /// Show plugin details
    Show {
        /// Plugin name
        name: String,
    },
}

// ... (其余子命令定义保持不变)

#[derive(Subcommand)]
enum CrsAction {
    /// List all registered coordinate reference systems
    List,

    /// Show details for a specific CRS
    Show {
        /// EPSG code
        epsg: u16,
    },

    /// Transform a coordinate or WKT geometry between CRS.
    /// Use --batch to read points from stdin (one "x,y" per line).
    Transform {
        /// Source EPSG code
        #[arg(long, default_value = "4326")]
        from: u16,
        /// Target EPSG code
        #[arg(long, default_value = "4326")]
        to: u16,
        /// X / longitude (ignored if --batch)
        x: Option<f64>,
        /// Y / latitude (ignored if --batch)
        y: Option<f64>,
        /// Batch mode: read "x,y" pairs from stdin, output "x,y" per line
        #[arg(long)]
        batch: bool,
    },

    /// Register a new CRS (adds to runtime — not persisted yet)
    Register {
        /// EPSG code
        epsg: u16,
        /// Human-readable name
        name: String,
        /// PROJ string
        proj4: String,
    },
}

#[derive(Subcommand)]
enum IngestAction {
    /// Ingest a CamoFox JSON file into PostGIS
    Camofox {
        /// Path to the JSON file
        file: String,
    },
    /// Parse an NMEA log file and print GPS fixes
    Nmea {
        /// Path to NMEA log file
        file: String,
    },
    /// Subscribe to MQTT topic and stream to TimescaleDB
    #[cfg(feature = "mqtt")]
    Mqtt {
        /// MQTT broker address
        #[arg(long, default_value = "localhost")]
        broker: String,
        /// MQTT broker port
        #[arg(long, default_value = "1883")]
        port: u16,
        /// MQTT topic to subscribe
        #[arg(long)]
        topic: String,
    },
}

#[derive(Subcommand)]
enum StoreAction {
    /// Run database migrations
    Migrate,
    /// Write a GeoJSON file to a PostGIS table
    Write {
        /// Target table name
        table: String,
        /// Path to GeoJSON / GPKG file
        file: String,
    },
    /// Execute a SQL query and print results as JSON
    Read {
        /// SQL query
        sql: String,
    },
    /// Run DVC snapshot (dvc add + dvc push) on a file
    DvcSnapshot {
        /// Path to the file to track
        file: String,
    },
    /// Pull DVC-tracked data from remote
    DvcPull {
        /// Optional target file
        target: Option<String>,
    },
    /// Show DVC hash of a tracked file
    DvcHash {
        /// Path to the tracked file
        file: String,
    },
}

#[derive(Subcommand)]
enum ProcessAction {
    /// Dispatch a GEE task via message queue
    Gee {
        #[command(subcommand)]
        action: GeeAction,
    },
    /// Convert raster/vector formats via GDAL
    Gdal {
        #[command(subcommand)]
        action: GdalAction,
    },
    /// Run QGIS processing
    Qgis {
        #[command(subcommand)]
        action: QgisAction,
    },
}

#[derive(Subcommand)]
enum GeeAction {
    /// Send landcover classification task to Python gee-worker
    Classify {
        /// AOI asset path on S3
        #[arg(long)]
        aoi: String,
        /// Target year
        #[arg(long)]
        year: u16,
        /// Output GCS URI
        #[arg(long)]
        output_gcs: String,
        /// Optional classifier parameters as JSON
        #[arg(long)]
        params: Option<String>,
    },
    /// Send NDVI time-series task
    Ndvi {
        #[arg(long)]
        aoi: String,
        #[arg(long)]
        year: u16,
        #[arg(long)]
        output_gcs: String,
    },
    /// Send change detection task (two-year comparison)
    Change {
        #[arg(long)]
        aoi: String,
        /// Baseline year
        #[arg(long)]
        from: u16,
        /// Comparison year
        #[arg(long)]
        to: u16,
        #[arg(long)]
        output_gcs: String,
    },
    /// Check GEE task status
    Status {
        /// Correlation ID
        #[arg(long)]
        cid: String,
    },
    /// Show summary of all GEE tasks
    Summary,
}

#[derive(Subcommand)]
enum GdalAction {
    /// Convert raster to COG format
    Cog {
        /// Input raster path
        input: String,
        /// Output COG path
        output: String,
        /// Compression type (default: DEFLATE)
        #[arg(long, default_value = "DEFLATE")]
        compression: String,
    },
    /// Reproject a raster
    Reproject {
        /// Input raster path
        input: String,
        /// Output raster path
        output: String,
        /// Target EPSG code
        #[arg(long)]
        epsg: u16,
    },
    /// Convert between vector formats (ogr2ogr)
    Ogr2Ogr {
        /// Input file path
        input: String,
        /// Output file path
        output: String,
        /// Target EPSG (optional)
        #[arg(long)]
        epsg: Option<u16>,
        /// Where clause filter (optional)
        #[arg(long)]
        r#where: Option<String>,
        /// Overwrite output
        #[arg(long)]
        overwrite: bool,
    },
    /// Sync file from GCS to MinIO or local
    GcsBridge {
        /// GCS URI (gs://bucket/path)
        gcs_uri: String,
        /// Target prefix for MinIO or local directory
        #[arg(long, default_value = "gee-exports")]
        prefix: String,
        /// Convert to COG during transfer
        #[arg(long)]
        cog: bool,
        /// Output to local dir only (skip MinIO)
        #[arg(long)]
        local: bool,
    },
}

#[derive(Subcommand)]
enum QgisAction {
    /// Submit a job to PyQGIS REST service
    Submit {
        /// Algorithm ID (e.g., native:buffer)
        #[arg(long)]
        algorithm: String,
        /// JSON params (e.g., '{"INPUT":"layer","DISTANCE":100}')
        #[arg(long)]
        params: String,
        /// Input file path
        #[arg(long)]
        input: String,
        /// Output file path
        #[arg(long)]
        output: String,
        /// PyQGIS service URL
        #[arg(long, default_value = "http://localhost:9100")]
        server: String,
    },
    /// Run qgis_process CLI directly
    Batch {
        /// Algorithm ID
        #[arg(long)]
        algorithm: String,
        /// Input file path
        #[arg(long)]
        input: String,
        /// Output file path
        #[arg(long)]
        output: String,
        /// Extra params as JSON array of [key, value] pairs
        #[arg(long, default_value = "[]")]
        extra: String,
    },
    /// Check if PyQGIS service is alive
    Health {
        #[arg(long, default_value = "http://localhost:9100")]
        server: String,
    },
}

#[derive(Subcommand)]
enum CarbonAction {
    /// Emission factor method (IPCC)
    EmissionFactor {
        #[command(subcommand)]
        action: EfAction,
    },
    /// LCA via brightway2 (WIP)
    Lca {
        /// Path to inventory file
        inventory: String,
    },
}

#[derive(Subcommand)]
enum EfAction {
    /// Import emission factors from CSV
    Register {
        /// Path to CSV file
        csv: String,
    },
    /// Calculate emissions for an AOI
    Calculate {
        /// AOI UUID
        #[arg(long)]
        aoi: String,
        /// Accounting year
        #[arg(long)]
        year: u16,
        /// Emission factor source (e.g. IPCC_2019)
        #[arg(long, default_value = "IPCC_2019")]
        source: String,
    },
}

#[derive(Subcommand)]
enum OutputAction {
    /// Generate Excel dashboard from SQL
    Excel {
        /// SQL query
        sql: String,
        /// Output xlsx path
        #[arg(long)]
        output: String,
        /// Sheet name
        #[arg(long, default_value = "Data")]
        sheet: String,
    },
    /// Export GeoJSON from SQL spatial query or local file.
    /// With --from-file: validate/compact/reproject a local GeoJSON file.
    Geojson {
        /// SQL query (must return a 'feature' or 'geojson' column)
        #[arg(required_unless_present = "from_file")]
        sql: Option<String>,
        /// Output GeoJSON path
        #[arg(long)]
        output: String,
        /// Use aggregate mode (PostGIS builds FeatureCollection)
        #[arg(long)]
        aggregate: bool,
        /// Read from local file instead of SQL (validate + compact + reproject)
        #[arg(long)]
        from_file: Option<String>,
        /// Reproject to EPSG when using --from-file
        #[arg(long)]
        to_epsg: Option<u16>,
    },
    /// Export PostGIS vectors to DXF (CAD format)
    Dxf {
        /// SQL query (must return geom_json and layer columns)
        sql: String,
        /// Output DXF path
        #[arg(long)]
        output: String,
        /// Source EPSG
        #[arg(long, default_value = "4326")]
        from_epsg: u16,
        /// Target EPSG for DXF
        #[arg(long, default_value = "4326")]
        to_epsg: u16,
    },
    /// Generate a carbon accounting report (Markdown or HTML)
    Report {
        /// AOI UUID
        #[arg(long)]
        aoi: String,
        /// Year
        #[arg(long)]
        year: u16,
        /// AOI display name
        #[arg(long, default_value = "Unknown AOI")]
        name: String,
        /// Source name
        #[arg(long, default_value = "IPCC_2019")]
        source: String,
        /// Output format: md or html
        #[arg(long, default_value = "md")]
        format: String,
        /// Output file path
        #[arg(long)]
        output: String,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Crs { action } => commands::crs::handle(action),
        Commands::Ingest { action } => commands::ingest::handle(action).await,
        Commands::Store { action } => commands::store::handle(action).await,
        Commands::Process(action) => commands::process::handle(action).await,
        Commands::Carbon(action) => commands::carbon::handle(action).await,
        Commands::Output(action) => commands::output::handle(action).await,
        Commands::Plugins { action } => handle_plugins(action),
        Commands::McpServe { port: _ } => {
            let registry = build_registry();
            mcp::serve(registry).await?;
            Ok(())
        }
    }
}

/// 构建插件注册表，注册所有已知工具及其处理器。
/// MCP / CLI 统一通过 registry.dispatch() 调用。
fn build_registry() -> PluginRegistry {
    use geo_registry::registry::{ToolDef, ToolResult};

    let mut registry = PluginRegistry::new();

    // ══════════════════════════════════════════════════
    // CRS — 同步
    // ══════════════════════════════════════════════════
    registry.register(geo_core::plugin::PluginMeta {
        name: "crs".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "CRS coordinate reference system registry".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_sync(
        "crs",
        ToolDef {
            name: "crs_list".into(),
            description: "List all registered coordinate reference systems".into(),
            input_schema: serde_json::json!({"type":"object","properties":{},"required":[]}),
        },
        |_args| -> ToolResult {
            let reg = geo_core::crs::CrsRegistry::new();
            let list: Vec<serde_json::Value> = reg
                .list()
                .map(|c| {
                    serde_json::json!({
                        "epsg": c.epsg,
                        "name": c.name,
                        "category": format!("{:?}", c.category),
                        "proj4": c.proj4
                    })
                })
                .collect();
            Ok(serde_json::json!(list))
        },
    );

    registry.register_tool_sync(
        "crs",
        ToolDef {
            name: "crs_transform".into(),
            description: "Transform coordinates between CRS".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{
                    "from_epsg":{"type":"integer"},
                    "to_epsg":{"type":"integer"},
                    "x":{"type":"number"},
                    "y":{"type":"number"}
                },
                "required":["from_epsg","to_epsg","x","y"]
            }),
        },
        |args| -> ToolResult {
            let reg = geo_core::crs::CrsRegistry::new();
            let from = args["from_epsg"].as_u64().unwrap_or(4326) as u16;
            let to = args["to_epsg"].as_u64().unwrap_or(4326) as u16;
            let x = args["x"].as_f64().unwrap_or(0.0);
            let y = args["y"].as_f64().unwrap_or(0.0);
            let (ox, oy) = reg
                .transform_point(from, to, x, y)
                .map_err(|e| geo_core::GeoError::CrsTransform(e.to_string()))?;
            Ok(serde_json::json!({
                "from_epsg": from, "to_epsg": to,
                "input": [x, y], "output": [ox, oy],
                "message": format!("EPSG:{from} ({x}, {y}) → EPSG:{to} ({ox:.4}, {oy:.4})")
            }))
        },
    );

    // ══════════════════════════════════════════════════
    // Store / PostGIS — 异步
    // ══════════════════════════════════════════════════
    registry.register(geo_core::plugin::PluginMeta {
        name: "store".into(),
        version: "0.1.0".into(),
        description: "PostGIS spatial data storage".into(),
        category: PluginCategory::Store,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_async(
        "store",
        ToolDef {
            name: "store_migrate".into(),
            description: "Run PostGIS database migrations".into(),
            input_schema: serde_json::json!({"type":"object","properties":{},"required":[]}),
        },
        |_args| {
            Box::pin(async move {
                let db_url = std::env::var("DATABASE_URL")
                    .map_err(|_| geo_core::GeoError::Other("DATABASE_URL not set".into()))?;
                let store = geo_adapter_postgis::PostgisStore::connect(&db_url)
                    .await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                geo_adapter_postgis::run_migrations(store.pool())
                    .await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                Ok(serde_json::json!("Migrations applied successfully"))
            })
        },
    );

    registry.register_tool_async(
        "store",
        ToolDef {
            name: "store_query".into(),
            description: "Execute a SQL query and return results as JSON".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{"sql":{"type":"string"}},
                "required":["sql"]
            }),
        },
        |args| {
            Box::pin(async move {
                let db_url = std::env::var("DATABASE_URL")
                    .map_err(|_| geo_core::GeoError::Other("DATABASE_URL not set".into()))?;
                let store = geo_adapter_postgis::PostgisStore::connect(&db_url)
                    .await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let sql = args["sql"].as_str().unwrap_or("SELECT 1");
                store
                    .query_json(sql)
                    .await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))
                    .map(|rows| serde_json::json!(rows))
            })
        },
    );

    // ══════════════════════════════════════════════════
    // Ingest — 异步
    // ══════════════════════════════════════════════════
    registry.register(geo_core::plugin::PluginMeta {
        name: "ingest".into(),
        version: "0.1.0".into(),
        description: "Data ingestion (CamoFox, NMEA)".into(),
        category: PluginCategory::Ingest,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_async(
        "ingest",
        ToolDef {
            name: "ingest_camofox".into(),
            description: "Parse a CamoFox JSON file and return records".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{"file":{"type":"string"}},
                "required":["file"]
            }),
        },
        |args| {
            Box::pin(async move {
                let file = args["file"].as_str().unwrap_or("");
                let content = tokio::fs::read_to_string(file)
                    .await
                    .map_err(geo_core::GeoError::from)?;
                let (_rows, result) = geo_io::camofox::parse_camofox_file(&content, file)
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({
                    "accepted": result.accepted,
                    "rejected": result.rejected,
                    "file": file
                }))
            })
        },
    );

    registry.register_tool_async(
        "ingest",
        ToolDef {
            name: "ingest_nmea".into(),
            description: "Parse an NMEA GPS log file and return fixes".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{"file":{"type":"string"}},
                "required":["file"]
            }),
        },
        |args| {
            Box::pin(async move {
                let file = args["file"].as_str().unwrap_or("");
                let content = tokio::fs::read_to_string(file)
                    .await
                    .map_err(geo_core::GeoError::from)?;
                let mut fixes = 0u32;
                let mut records: Vec<serde_json::Value> = Vec::new();
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(msg) = geo_io::nmea::parse_nmea_line(line.trim()) {
                        match msg {
                            geo_io::nmea::NmeaMessage::Gga(fix) => {
                                records.push(serde_json::json!({
                                    "type":"GGA", "time":fix.time,
                                    "lat":fix.lat, "lng":fix.lng,
                                    "quality":fix.quality, "satellites":fix.satellites
                                }));
                                fixes += 1;
                            }
                            geo_io::nmea::NmeaMessage::Rmc(rmc) => {
                                records.push(serde_json::json!({
                                    "type":"RMC", "time":rmc.time,
                                    "lat":rmc.lat, "lng":rmc.lng,
                                    "speed_knots":rmc.speed_knots
                                }));
                                fixes += 1;
                            }
                            _ => {}
                        }
                    }
                }
                let preview: Vec<_> = records.iter().take(10).cloned().collect();
                Ok(serde_json::json!({
                    "total_fixes": fixes,
                    "records": preview
                }))
            })
        },
    );

    // ══════════════════════════════════════════════════
    // DVC — 同步
    // ══════════════════════════════════════════════════
    registry.register(geo_core::plugin::PluginMeta {
        name: "dvc".into(),
        version: "0.1.0".into(),
        description: "DVC data version control".into(),
        category: PluginCategory::Store,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_sync(
        "dvc",
        ToolDef {
            name: "dvc_snapshot".into(),
            description: "Run DVC add + push on a file for version tracking".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{"file":{"type":"string"}},
                "required":["file"]
            }),
        },
        |args| -> ToolResult {
            let file = args["file"].as_str().unwrap_or("");
            let snap = geo_adapter_postgis::dvc_snapshot(file)
                .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
            Ok(serde_json::json!({
                "file": snap.file,
                "dvc_hash": snap.dvc_hash
            }))
        },
    );

    registry.register_tool_sync(
        "dvc",
        ToolDef {
            name: "dvc_hash".into(),
            description: "Get the DVC MD5 hash of a tracked file".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{"file":{"type":"string"}},
                "required":["file"]
            }),
        },
        |args| -> ToolResult {
            let file = args["file"].as_str().unwrap_or("");
            let hash = geo_adapter_postgis::dvc_hash(file)
                .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
            Ok(serde_json::json!({"dvc_hash": hash}))
        },
    );

    // ══════════════════════════════════════════════════
    // Carbon — 异步
    // ══════════════════════════════════════════════════
    registry.register(geo_core::plugin::PluginMeta {
        name: "carbon".into(),
        version: "0.1.0".into(),
        description: "Carbon accounting engine (IPCC Tier 1 + PostGIS)".into(),
        category: PluginCategory::Carbon,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_async(
        "carbon",
        ToolDef {
            name: "carbon_calculate".into(),
            description: "Calculate carbon emissions using emission factor method".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{
                    "aoi_id":{"type":"string"},
                    "year":{"type":"integer"},
                    "source":{"type":"string","default":"IPCC_2019"}
                },
                "required":["aoi_id","year"]
            }),
        },
        |args| {
            Box::pin(async move {
                let db_url = std::env::var("DATABASE_URL")
                    .map_err(|_| geo_core::GeoError::Other("DATABASE_URL not set".into()))?;
                let aoi = args["aoi_id"].as_str().unwrap_or("");
                let year = args["year"].as_u64().unwrap_or(2025) as u16;
                let source = args["source"].as_str().unwrap_or("IPCC_2019");
                let aoi_id = uuid::Uuid::parse_str(aoi)
                    .map_err(|e| geo_core::GeoError::Validation(format!("invalid AOI UUID: {e}")))?;

                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&db_url)
                    .await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let engine = geo_adapter_postgis::PostgisCarbonEngine::new(pool);
                let results = engine
                    .calculate_emission_factor(aoi_id, year, source)
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;

                let total: f64 = results.iter().map(|r| r.emission_tco2e).sum();
                let summary: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "landcover_class": r.landcover_class,
                            "area_ha": r.area_ha,
                            "emission_tco2e": r.emission_tco2e
                        })
                    })
                    .collect();
                Ok(serde_json::json!({
                    "aoi_id": aoi, "year": year, "total_tco2e": total, "results": summary
                }))
            })
        },
    );

    registry.register_tool_async(
        "carbon",
        ToolDef {
            name: "carbon_import_factors".into(),
            description: "Import emission factors from a CSV file".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{"csv_path":{"type":"string"}},
                "required":["csv_path"]
            }),
        },
        |args| {
            Box::pin(async move {
                let db_url = std::env::var("DATABASE_URL")
                    .map_err(|_| geo_core::GeoError::Other("DATABASE_URL not set".into()))?;
                let csv_path = args["csv_path"].as_str().unwrap_or("");
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&db_url)
                    .await
                    .map_err(|e| geo_core::GeoError::Database(e.to_string()))?;
                let engine = geo_adapter_postgis::PostgisCarbonEngine::new(pool);
                let count = engine
                    .import_factors_csv(csv_path)
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({
                    "imported": count,
                    "file": csv_path
                }))
            })
        },
    );

    // ══════════════════════════════════════════════════
    // GEE — 异步（需 feature = "gee"）
    // ══════════════════════════════════════════════════
    #[cfg(feature = "gee")]
    registry.register(geo_core::plugin::PluginMeta {
        name: "gee".into(),
        version: "0.1.0".into(),
        description: "Google Earth Engine remote sensing adapter".into(),
        category: PluginCategory::Adapter,
        healthy: true,
        extra: serde_json::json!({}),
    });

    #[cfg(feature = "gee")]
    registry.register_tool_async(
        "gee",
        ToolDef {
            name: "gee_classify".into(),
            description: "Submit landcover classification task to GEE".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{
                    "aoi":{"type":"string","description":"AOI asset path on S3"},
                    "year":{"type":"integer"},
                    "output_gcs":{"type":"string","description":"Output GCS URI"}
                },
                "required":["aoi","year","output_gcs"]
            }),
        },
        |args| {
            Box::pin(async move {
                let adapter = geo_adapter_gee::GeeAdapter::new_default()
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                let aoi = args["aoi"].as_str().unwrap_or("");
                let year = args["year"].as_u64().unwrap_or(2025) as u16;
                let output_gcs = args["output_gcs"].as_str().unwrap_or("gs://gee-exports/lc.tif");
                let image_collection = "COPERNICUS/S2_SR_HARMONIZED";
                let task = adapter
                    .submit_classification(aoi, year, image_collection, output_gcs)
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({
                    "task_id": task,
                    "aoi": aoi,
                    "year": year,
                    "collection": image_collection
                }))
            })
        },
    );

    #[cfg(feature = "gee")]
    registry.register_tool_async(
        "gee",
        ToolDef {
            name: "gee_status".into(),
            description: "Check GEE task status by correlation ID".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{"cid":{"type":"string"}},
                "required":["cid"]
            }),
        },
        |args| {
            Box::pin(async move {
                let adapter = geo_adapter_gee::GeeAdapter::new_default()
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                let cid = args["cid"].as_str().unwrap_or("");
                let status = adapter
                    .job_status(cid)
                    .await
                    .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;
                Ok(serde_json::json!({"cid": cid, "status": status}))
            })
        },
    );

    // ══════════════════════════════════════════════════
    // Ecology Plugin — 同步（纯 Rust 计算）
    // ══════════════════════════════════════════════════
    registry.register(geo_core::plugin::PluginMeta {
        name: "ecology".into(),
        version: "0.1.0".into(),
        description: "Ecological restoration assessment — NDVI change detection, carbon sink".into(),
        category: PluginCategory::Process,
        healthy: true,
        extra: serde_json::json!({}),
    });

    registry.register_tool_sync(
        "ecology",
        ToolDef {
            name: "ecology_assess".into(),
            description: "Assess ecological restoration via two-period NDVI comparison + carbon sink".into(),
            input_schema: serde_json::json!({
                "type":"object",
                "properties":{
                    "aoi_name":{"type":"string","description":"AOI display name"},
                    "baseline_year":{"type":"integer"},
                    "assessment_year":{"type":"integer"},
                    "aoi_geojson":{"type":"string","description":"AOI GeoJSON FeatureCollection"},
                    "config_path":{"type":"string","description":"Optional path to rules.toml"}
                },
                "required":["aoi_name","baseline_year","assessment_year"]
            }),
        },
        |args| -> ToolResult {
            use geo_plugin_ecology::ecology::AssessmentInput;

            let aoi_name_str = args["aoi_name"].as_str().unwrap_or("Unknown").to_string();
            let baseline_year = args["baseline_year"].as_u64().unwrap_or(2020) as u16;
            let assessment_year = args["assessment_year"].as_u64().unwrap_or(2025) as u16;
            let geojson_str = args["aoi_geojson"].as_str().unwrap_or("{}").to_string();

            // 加载配置
            let config_path = args["config_path"]
                .as_str()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    std::path::PathBuf::from("plugins/geo-plugin-ecology/rules.toml")
                });

            let plugin = geo_plugin_ecology::EcologyPlugin::load_from_file(&config_path)
                .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;

            // 构建模拟 NDVI 数据（MCP 场景下用模拟数据演示管线）
            let red = geo_raster::RasterBand::new("B4", 100, 100, vec![0.05; 10000], -999.0);
            let nir = geo_raster::RasterBand::new("B8", 100, 100, vec![0.50; 10000], -999.0);

            let input = AssessmentInput {
                aoi_name: &aoi_name_str,
                aoi_geojson: &geojson_str,
                baseline_red: &red,
                baseline_nir: &nir,
                assessment_red: &red,
                assessment_nir: &nir,
                baseline_year,
                assessment_year,
            };

            let assessment = plugin
                .assess_restoration(&input)
                .map_err(|e| geo_core::GeoError::Other(e.to_string()))?;

            Ok(serde_json::json!({
                "aoi_name": assessment.aoi_name,
                "baseline_year": assessment.baseline_year,
                "assessment_year": assessment.assessment_year,
                "conclusion": {
                    "grade": assessment.conclusion.grade,
                    "summary": assessment.conclusion.summary
                }
            }))
        },
    );

    registry
}

/// 处理 plugins 子命令。
fn handle_plugins(action: PluginsAction) -> Result<(), Box<dyn std::error::Error>> {
    let registry = build_registry();
    match action {
        PluginsAction::List { category } => {
            let plugins = registry.list_plugins();
            let filtered: Vec<_> = if let Some(cat) = category {
                let cat = PluginCategory::parse(&cat)
                    .ok_or_else(|| format!("Unknown category: {cat}"))?;
                plugins.iter().filter(|p| p.category == cat).collect()
            } else {
                plugins.iter().collect()
            };
            println!("{:<15} {:<8} {:<10} DESCRIPTION", "NAME", "VERSION", "CATEGORY");
            println!("{}", "-".repeat(80));
            let total = filtered.len();
            for p in filtered {
                println!("{:<15} {:<8} {:<10} {}", p.name, p.version, p.category.as_str(), p.description);
            }
            println!("\nTotal: {total} plugins/adapters");
        }
        PluginsAction::Show { name } => {
            let plugins = registry.list_plugins();
            if let Some(p) = plugins.iter().find(|p| p.name == name) {
                println!("Name:        {}", p.name);
                println!("Version:     {}", p.version);
                println!("Category:    {:?}", p.category);
                println!("Description: {}", p.description);
                println!("Healthy:     {}", p.healthy);
                let tools = registry.list_tools().iter().filter(|t| t.name.starts_with(&name)).count();
                println!("Tools:       {tools}");
            } else {
                println!("Plugin '{name}' not found");
            }
        }
    }
    Ok(())
}
