//! geo-toolbox CLI entry point.
//!
//! Maps each subcommand to the corresponding handler module under `commands/`.

use clap::{Parser, Subcommand};

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
}

#[derive(Subcommand)]
enum CrsAction {
    /// List all registered coordinate reference systems
    List,

    /// Show details for a specific CRS
    Show {
        /// EPSG code
        epsg: u16,
    },

    /// Transform a coordinate or WKT geometry between CRS
    Transform {
        /// Source EPSG code
        #[arg(long, default_value = "4326")]
        from: u16,
        /// Target EPSG code
        #[arg(long, default_value = "4326")]
        to: u16,
        /// X / longitude
        x: f64,
        /// Y / latitude
        y: f64,
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
    /// Export GeoJSON from SQL spatial query
    Geojson {
        /// SQL query (must return a 'feature' or 'geojson' column)
        sql: String,
        /// Output GeoJSON path
        #[arg(long)]
        output: String,
        /// Use aggregate mode (PostGIS builds FeatureCollection)
        #[arg(long)]
        aggregate: bool,
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
        Commands::McpServe { port: _ } => {
            mcp::serve().await?;
            Ok(())
        }
    }
}
