# geo-toolbox

English | [简体中文](README.zh-CN.md)

Rust geospatial toolbox for building reproducible GIS automation pipelines. It combines pure Rust spatial primitives with feature-gated adapters for PostGIS, GDAL, PDAL, Google Earth Engine, QGIS, DuckDB, STAC, OSM, CAD, and simulation tools.

## What It Provides

- A 45-package Rust workspace with 15 core crates, 18 domain plugins, 5 adapter crates, and CLI/WASM/Agent/Server entry points.
- A layered dependency model: Core -> Facade -> Plugin -> Wiring -> Adapter. Only `geo-wiring` composes plugins with adapters.
- Offline-friendly spatial utilities for CRS, vectors, rasters, tiles, temporal analysis, indexing, statistics, carbon accounting, GeoParquet, and OGC services.
- Integrations for AI agents through the CLI, MCP endpoint, HTTP GeoAgent, Python bindings, MapLibre bindings, QGIS plugin, and a field PWA.

## Repository Layout

```text
geo-toolbox/
|- core/       # Pure Rust geospatial algorithms and shared abstractions
|- plugins/    # Domain workflows: carbon, ecology, hydro, remote sensing, and more
|- adapters/   # Feature-gated bridges to external GIS systems
|- crates/     # CLI, WASM, Agent, Server, and composition root
|- bindings/   # Python, MapLibre GL JS, Jupyter, and QGIS integrations
|- apps/       # Field PWA
|- examples/   # Reproducible geospatial examples and fixtures
|- docs/       # Supplemental documentation
|- fuzz/       # cargo-fuzz targets
```

Read [BOUNDARY.md](BOUNDARY.md) for the dependency rules, [WIKI.md](WIKI.md) for usage guidance, and [ROADMAP.md](ROADMAP.md) for current capability and reliability work.

## Quick Start

### Prerequisites

- Rust stable, with an MSRV target of Rust 1.80.
- Optional external tools only for the adapters that need them: GDAL, QGIS, PostgreSQL/PostGIS, PDAL, or Python.

### Build and Test the Portable Workspace

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace -- -D warnings \
  -A clippy::too_many_arguments \
  -A clippy::manual_clamp \
  -A clippy::needless_range_loop \
  -A clippy::should_implement_trait
```

The CI workflow runs the same portable checks on Linux and Windows. Native GDAL bindings are checked separately on Ubuntu with `libgdal-dev`, so a local build does not need a GDAL SDK unless that feature is explicitly enabled.

### Run the CLI

```bash
cargo run -p geo-cli -- carbon assess input.geojson
cargo run -p geo-cli -- hydro basin dem.tif
cargo run -p geo-cli -- mcp-serve
```

The default CLI build keeps the existing full adapter behavior. The feature group is named `full` so CI and downstream packaging can select it explicitly. Use `minimal` for a portable build without optional adapters:

```bash
# Full CLI, equivalent to the historical default.
cargo build --release --features full -p geo-cli

# Portable CLI with no optional adapter family.
cargo build --release --no-default-features --features minimal -p geo-cli
```

### Run GeoAgent

```bash
cargo run -p geo-agent
curl -X POST http://127.0.0.1:3000/agent \
  -H "Content-Type: application/json" \
  -d '{"query":"calculate NDVI for this area"}'
```

See [crates/geo-agent/README.md](crates/geo-agent/README.md) for provider and endpoint configuration. The main CI also builds and tests this target explicitly as the AI edge gateway.

### Build the Browser and Offline Targets

The project has two browser-facing delivery paths:

- **WASM library**: `crates/geo-wasm` exposes CRS, vector, raster, carbon, geohash, tile, statistics, and IndexedDB-backed local storage to browser JavaScript. It is paired with the MapLibre integration in `crates/geo-wasm-maplibre` and `bindings/maplibre-gl-geo-toolbox`.
- **Offline Field PWA**: `apps/field-pwa` is a Vite PWA for offline field collection, local IndexedDB storage, map-based area capture, and carbon calculation. Its production build includes a service worker and precache manifest.

```bash
# Compile the browser target without native network backends.
cargo check -p geo-wasm --target wasm32-unknown-unknown

# Create a web-consumable WASM package.
wasm-pack build --target web --out-dir pkg crates/geo-wasm

# Build the offline browser application.
cd apps/field-pwa
npm ci
npm run build
```

`WASM CI` runs on `master` and `develop`. It validates the WASM package, browser tests, demo output, and the Field PWA production build. The build artifact is produced locally or by CI; it is distinct from the source-controlled field application and examples.

## Configuration

Copy `config.example.json` to `config.json` or set `GEO_CONFIG_PATH` to a configuration file outside the repository. `config.json` is intentionally ignored because adapter paths, service endpoints, and credentials are machine-specific.

```bash
cp config.example.json config.json
# Set GEO_CONFIG_PATH when configuration is stored elsewhere.
```

Never commit credentials, real service-account paths, or workstation-specific executable locations.

## External Adapter Expectations

Adapters validate failure conditions before launching external processes where possible. The repository default test suite is hermetic; real service and binary integration checks belong in opt-in environments with the required tools installed.

The internal `GT v1` tile archive is not PMTiles v3. Do not present it as interoperable with MapLibre or Protomaps PMTiles readers until an actual PMTiles v3 implementation and official-fixture tests exist.

## Quality and Security

- CI checks formatting, Clippy, workspace builds and tests, dependency audit, coverage, fuzz targets, and release/WASM workflows.
- `cargo audit` blocks unapproved advisories. Existing upstream exceptions are documented in [`.cargo/audit.toml`](.cargo/audit.toml) and should be revisited when upstream dependencies change.
- The current main development branch is `master`.

## Bindings and Examples

- [MapLibre GL JS binding](bindings/maplibre-gl-geo-toolbox/README.md)
- [QGIS plugin](bindings/qgis/geo_toolbox_qgis/README.md)
- [Jupyter integration](bindings/jupyter/README.md)
- [ObservableHQ examples](docs/observablehq/README.md)
- [China risk assessment example](examples/china-risk-assessment/README.md)

### Versioned China Risk Assessment Outputs

The China risk assessment example includes source data, reproducible Python pipelines, and four checked-in reference outputs. These files are intentionally versioned so the example can be reviewed without running its data pipeline:

- `china_flood_risk_2026.png`: national flood-risk thematic map.
- `china_flood_risk_2026_regions.png`: regional risk-distribution map.
- `china_flood_risk_2026_stats.png`: risk statistics chart.
- `中国2026年洪水高风险区评估报告.pdf`: Chinese flood high-risk assessment report.

Do not delete these reference images or the PDF as build artifacts. The two generated GeoJSON result layers remain reproducible pipeline outputs; the PNG maps and PDF report are retained as project documentation and visual regression references.

## License

MIT
