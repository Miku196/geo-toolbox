//! geo-toolbox CLI entry point.
//!
//! All subcommands dispatch through PluginRegistry.
//! Heavy adapters (PostGIS, GEE, QGIS, CAD) are feature-gated — compile only what you need.

use clap::{Parser, Subcommand};
use geo_core::plugin::PluginCategory;
use geo_wiring::PluginRegistry;

mod commands;
mod mcp;

// ── CLI Surface ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "geo-toolbox",
    version = env!("CARGO_PKG_VERSION"),
    about = "Rust geospatial pipeline toolbox",
    long_about = "High-performance glue layer. Uses PluginRegistry dispatch.\n\
                  Compile with --no-default-features --features minimal for zero external deps."
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
    /// MCP server (Model Context Protocol for AI Agent integration)
    McpServe {
        #[arg(long, default_value = "9378")]
        port: u16,
    },
    /// Data ingestion: CamoFox, NMEA
    Ingest {
        #[command(subcommand)]
        action: IngestAction,
    },
    /// Data storage (requires --features postgis)
    #[cfg(feature = "postgis")]
    #[command(subcommand)]
    Store(StoreAction),
    /// Geoprocessing (requires --features gee,qgis,gdal)
    #[cfg(any(feature = "gee", feature = "gdal", feature = "qgis"))]
    #[command(subcommand)]
    Process(ProcessAction),
    /// Carbon accounting (requires --features postgis)
    #[command(subcommand)]
    Carbon(CarbonAction),
    /// Output: Excel, DXF, GeoJSON, reports (requires --features cad)
    #[command(subcommand)]
    Output(OutputAction),
    /// Plugin registry: list, show
    Plugins {
        #[command(subcommand)]
        action: PluginsAction,
    },
    /// Pipeline mode: Unix pipe geospatial processing (read→buffer→simplify→reproject→write)
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },
}

// ── Subcommand enums ──────────────────────────────────────────────────────

/// Pipeline subcommands for Unix pipe geospatial processing.
#[derive(Subcommand, Debug, Clone)]
pub enum PipelineAction {
    /// Read geospatial data (file or stdin) and output GeoJSON
    Read {
        input: Option<String>,
        #[arg(long, default_value = "geojson")]
        format: String,
    },
    /// Buffer all geometries by a distance
    Buffer {
        #[arg(long, default_value = "100")]
        distance: f64,
        #[arg(long)]
        units: Option<String>,
    },
    /// Simplify geometries using Douglas-Peucker
    Simplify {
        #[arg(long, default_value = "0.01")]
        epsilon: f64,
    },
    /// Reproject geometries between CRS
    Reproject {
        #[arg(long)]
        from_epsg: u16,
        #[arg(long)]
        to_epsg: u16,
    },
    /// Write GeoJSON from stdin to a file
    Write {
        output: String,
        #[arg(long, default_value = "geojson")]
        format: String,
    },
    /// Compute area of all geometries
    Area,
    /// Filter features by property value
    Filter { key: String, value: String },
}

#[derive(Subcommand)]
enum PluginsAction {
    List {
        #[arg(long)]
        category: Option<String>,
    },
    Show {
        name: String,
    },
}

#[derive(Subcommand)]
enum CrsAction {
    List,
    Show {
        epsg: u16,
    },
    Transform {
        #[arg(long, default_value = "4326")]
        from: u16,
        #[arg(long, default_value = "4326")]
        to: u16,
        x: Option<f64>,
        y: Option<f64>,
        #[arg(long)]
        batch: bool,
    },
    Register {
        epsg: u16,
        name: String,
        proj4: String,
    },
}

#[derive(Subcommand)]
enum IngestAction {
    Camofox {
        file: String,
    },
    Nmea {
        file: String,
    },
    #[cfg(feature = "mqtt")]
    Mqtt {
        #[arg(long, default_value = "localhost")]
        broker: String,
        #[arg(long, default_value = "1883")]
        port: u16,
        #[arg(long)]
        topic: String,
    },
}

#[cfg(feature = "postgis")]
#[derive(Subcommand)]
enum StoreAction {
    Migrate,
    Write { table: String, file: String },
    Read { sql: String },
    DvcSnapshot { file: String },
    DvcPull { target: Option<String> },
    DvcHash { file: String },
}

#[derive(Subcommand)]
enum ProcessAction {
    #[cfg(feature = "gee")]
    Gee {
        #[command(subcommand)]
        action: GeeAction,
    },
    #[cfg(feature = "gdal")]
    Gdal {
        #[command(subcommand)]
        action: GdalAction,
    },
    #[cfg(feature = "qgis")]
    Qgis {
        #[command(subcommand)]
        action: QgisAction,
    },
}

#[cfg(feature = "gee")]
#[derive(Subcommand)]
enum GeeAction {
    Classify {
        #[arg(long)]
        aoi: String,
        #[arg(long)]
        year: u16,
        #[arg(long)]
        output_gcs: String,
        #[arg(long)]
        params: Option<String>,
    },
    Ndvi {
        #[arg(long)]
        aoi: String,
        #[arg(long)]
        year: u16,
        #[arg(long)]
        output_gcs: String,
    },
    Change {
        #[arg(long)]
        aoi: String,
        #[arg(long)]
        from: u16,
        #[arg(long)]
        to: u16,
        #[arg(long)]
        output_gcs: String,
    },
    Status {
        #[arg(long)]
        cid: String,
    },
    Summary,
}

#[cfg(feature = "gdal")]
#[derive(Subcommand)]
enum GdalAction {
    Cog {
        input: String,
        output: String,
        #[arg(long, default_value = "DEFLATE")]
        compression: String,
    },
    Reproject {
        input: String,
        output: String,
        #[arg(long)]
        epsg: u16,
    },
    Ogr2Ogr {
        input: String,
        output: String,
        #[arg(long)]
        epsg: Option<u16>,
        #[arg(long)]
        r#where: Option<String>,
        #[arg(long)]
        overwrite: bool,
    },
    GcsBridge {
        gcs_uri: String,
        #[arg(long, default_value = "gee-exports")]
        prefix: String,
        #[arg(long)]
        cog: bool,
        #[arg(long)]
        local: bool,
    },
}

#[cfg(feature = "qgis")]
#[derive(Subcommand)]
enum QgisAction {
    Submit {
        #[arg(long)]
        algorithm: String,
        #[arg(long)]
        params: String,
        #[arg(long)]
        input: String,
        #[arg(long)]
        output: String,
        #[arg(long, default_value = "http://localhost:9100")]
        server: String,
    },
    Batch {
        #[arg(long)]
        algorithm: String,
        #[arg(long)]
        input: String,
        #[arg(long)]
        output: String,
        #[arg(long, default_value = "[]")]
        extra: String,
    },
    Health {
        #[arg(long, default_value = "http://localhost:9100")]
        server: String,
    },
}

#[derive(Subcommand)]
enum CarbonAction {
    EmissionFactor {
        #[command(subcommand)]
        action: EfAction,
    },
    Lca {
        inventory: String,
    },
}

#[derive(Subcommand)]
enum EfAction {
    #[cfg(feature = "postgis")]
    Register { csv: String },
    #[cfg(feature = "postgis")]
    Calculate {
        #[arg(long)]
        aoi: String,
        #[arg(long)]
        year: u16,
        #[arg(long, default_value = "IPCC_2019")]
        source: String,
    },
}

#[derive(Subcommand)]
enum OutputAction {
    #[cfg(feature = "cad")]
    Excel {
        sql: String,
        #[arg(long)]
        output: String,
        #[arg(long, default_value = "Data")]
        sheet: String,
    },
    Geojson {
        #[arg(required_unless_present = "from_file")]
        sql: Option<String>,
        #[arg(long)]
        output: String,
        #[arg(long)]
        aggregate: bool,
        #[arg(long)]
        from_file: Option<String>,
        #[arg(long)]
        to_epsg: Option<u16>,
    },
    #[cfg(feature = "cad")]
    Dxf {
        sql: String,
        #[arg(long)]
        output: String,
        #[arg(long, default_value = "4326")]
        from_epsg: u16,
        #[arg(long, default_value = "4326")]
        to_epsg: u16,
    },
    Report {
        #[arg(long)]
        aoi: String,
        #[arg(long)]
        year: u16,
        #[arg(long, default_value = "Unknown AOI")]
        name: String,
        #[arg(long, default_value = "IPCC_2019")]
        source: String,
        #[arg(long, default_value = "md")]
        format: String,
        #[arg(long)]
        output: String,
    },
}

// ── main ───────────────────────────────────────────────────────────────────

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
        Commands::McpServe { port: _ } => {
            mcp::serve(build_registry()).await?;
            Ok(())
        }
        other => {
            let registry = build_registry();
            dispatch_cli(&registry, other).await
        }
    }
}

async fn dispatch_cli(
    registry: &PluginRegistry,
    command: Commands,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Crs { action } => commands::crs::handle(registry, action),
        Commands::Ingest { action } => commands::ingest::handle(registry, action).await,
        Commands::Plugins { action } => handle_plugins(registry, action),
        Commands::Pipeline { action } => commands::pipeline::handle(registry, action),

        #[cfg(feature = "postgis")]
        Commands::Store(action) => commands::store::handle(registry, action).await,

        #[cfg(any(feature = "gee", feature = "gdal", feature = "qgis"))]
        Commands::Process(action) => commands::process::handle(action).await,
        Commands::Carbon(action) => commands::carbon::handle(registry, action).await,
        Commands::Output(action) => commands::output::handle(registry, action).await,

        Commands::McpServe { .. } => unreachable!(),
    }
}

// ── Plugin Registry Setup ──────────────────────────────────────────────────

fn build_registry() -> PluginRegistry {
    let mut reg = PluginRegistry::new();
    let config = geo_wiring::GeoConfig::load_default().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}");
        None
    });
    geo_wiring::populate_defaults(&mut reg, config.as_ref());
    reg
}

// ── plugins handler ────────────────────────────────────────────────────────

fn handle_plugins(
    registry: &PluginRegistry,
    action: PluginsAction,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        PluginsAction::List { category } => {
            let plugins = registry.list_plugins();
            let filtered: Vec<_> = if let Some(cat) = &category {
                let cat =
                    PluginCategory::parse(cat).ok_or_else(|| format!("Unknown category: {cat}"))?;
                plugins.iter().filter(|p| p.category == cat).collect()
            } else {
                plugins.iter().collect()
            };
            println!(
                "{:<15} {:<8} {:<10} DESCRIPTION",
                "NAME", "VERSION", "CATEGORY"
            );
            println!("{}", "-".repeat(80));
            for p in &filtered {
                println!(
                    "{:<15} {:<8} {:<10} {}",
                    p.name,
                    p.version,
                    p.category.as_str(),
                    p.description
                );
            }
            println!("\nTotal: {} plugins/adapters", filtered.len());
        }
        PluginsAction::Show { name } => {
            let plugins = registry.list_plugins();
            if let Some(p) = plugins.iter().find(|p| p.name == name) {
                println!("Name:        {}", p.name);
                println!("Version:     {}", p.version);
                println!("Category:    {:?}", p.category);
                println!("Description: {}", p.description);
                println!("Healthy:     {}", p.healthy);
                let tools = registry
                    .list_tools()
                    .iter()
                    .filter(|t| t.name.starts_with(&name))
                    .count();
                println!("Tools:       {tools}");
            } else {
                println!("Plugin '{name}' not found");
            }
        }
    }
    Ok(())
}
