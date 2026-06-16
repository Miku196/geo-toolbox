//! geo-server: HTTP API for geo-toolbox.
//!
//! Routes MCP tools and OGC WMS behind a REST interface.
//! Usage: `cargo run -p geo-server --release`
//! Server listens on http://0.0.0.0:9378

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use geo_ogc::wms::{GetMapParams, WmsLayer, WmsRequest, WmsResponse, WmsService};
use geo_ogc::wmts::{
    WmtsGetTileParams, WmtsLayer as WmtsDef, WmtsRequest, WmtsResponse, WmtsService,
};
use geo_wiring::PluginRegistry;
use serde::Deserialize;
use std::sync::Arc;

mod registry;
use registry::build_registry;

/// Shared application state.
struct AppState {
    registry: PluginRegistry,
}

/// WMS query parameters (KVP encoding per OGC WMS 1.3.0).
#[derive(Debug, Deserialize)]
struct WmsQuery {
    request: Option<String>,
    service: Option<String>,

    // GetMap
    #[serde(default)]
    layers: Option<String>,
    #[serde(default)]
    styles: Option<String>,
    crs: Option<String>,
    bbox: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    transparent: Option<bool>,
    bgcolor: Option<String>,

    // GetFeatureInfo
    i: Option<u32>,
    j: Option<u32>,
    #[serde(default)]
    query_layers: Option<String>,
    #[serde(default)]
    info_format: Option<String>,
    feature_count: Option<u32>,
}

/// WMTS query parameters (KVP encoding per OGC WMTS 1.0.0).
#[derive(Debug, Deserialize)]
struct WmtsQuery {
    request: Option<String>,
    service: Option<String>,

    // GetTile
    layer: Option<String>,
    #[serde(default)]
    style: Option<String>,
    format: Option<String>,
    #[serde(rename = "TileMatrixSet")]
    tile_matrix_set: Option<String>,
    #[serde(rename = "TileMatrix")]
    tile_matrix: Option<String>,
    #[serde(rename = "TileCol")]
    tile_col: Option<u32>,
    #[serde(rename = "TileRow")]
    tile_row: Option<u32>,
}

fn parse_bbox(s: &str) -> Option<geo_ogc::common::Wgs84Bbox> {
    let parts: Vec<f64> = s.split(',').filter_map(|v| v.trim().parse().ok()).collect();
    if parts.len() == 4 {
        Some(geo_ogc::common::Wgs84Bbox {
            west: parts[0],
            south: parts[1],
            east: parts[2],
            north: parts[3],
        })
    } else {
        None
    }
}

fn parse_csv(s: &str) -> Vec<String> {
    s.split(',').map(|v| v.trim().to_string()).collect()
}

impl WmsQuery {
    fn into_wms_request(self) -> Result<WmsRequest, String> {
        let req_type = self.request.as_deref().unwrap_or("");
        match req_type {
            "GetCapabilities" => Ok(WmsRequest::GetCapabilities),
            "GetMap" => {
                let layers = self.layers.as_deref().map(parse_csv).unwrap_or_default();
                let styles = self.styles.as_deref().map(parse_csv).unwrap_or_default();
                let crs = self.crs.ok_or("missing crs")?;
                let bbox_str = self.bbox.ok_or("missing bbox")?;
                let bbox = parse_bbox(&bbox_str).ok_or("invalid bbox")?;
                let width = self.width.ok_or("missing width")?;
                let height = self.height.ok_or("missing height")?;
                let format = self.format.unwrap_or_else(|| "image/png".into());

                Ok(WmsRequest::GetMap(GetMapParams {
                    layers,
                    styles,
                    crs,
                    bbox,
                    width,
                    height,
                    format,
                    transparent: self.transparent.unwrap_or(false),
                    bgcolor: self.bgcolor,
                }))
            }
            "GetFeatureInfo" => Ok(WmsRequest::GetFeatureInfo(
                geo_ogc::wms::GetFeatureInfoParams {
                    map_params: GetMapParams {
                        layers: self.layers.as_deref().map(parse_csv).unwrap_or_default(),
                        styles: self.styles.as_deref().map(parse_csv).unwrap_or_default(),
                        crs: self.crs.ok_or("missing crs")?,
                        bbox: self
                            .bbox
                            .as_deref()
                            .and_then(parse_bbox)
                            .ok_or("missing bbox")?,
                        width: self.width.unwrap_or(256),
                        height: self.height.unwrap_or(256),
                        format: self.format.unwrap_or_else(|| "image/png".into()),
                        transparent: self.transparent.unwrap_or(false),
                        bgcolor: self.bgcolor,
                    },
                    i: self.i.ok_or("missing i")?,
                    j: self.j.ok_or("missing j")?,
                    query_layers: self
                        .query_layers
                        .as_deref()
                        .map(parse_csv)
                        .unwrap_or_default(),
                    info_format: self
                        .info_format
                        .unwrap_or_else(|| "application/json".into()),
                    feature_count: self.feature_count.unwrap_or(1),
                },
            )),
            _ => Err(format!("unknown WMS request type: {req_type}")),
        }
    }
}

impl WmtsQuery {
    fn into_wmts_request(self) -> Result<WmtsRequest, String> {
        let req_type = self.request.as_deref().unwrap_or("");
        match req_type {
            "GetCapabilities" => Ok(WmtsRequest::GetCapabilities),
            "GetTile" => {
                let layer = self.layer.ok_or("missing layer")?;
                let tile_matrix_set = self.tile_matrix_set.ok_or("missing TileMatrixSet")?;
                let tile_matrix = self.tile_matrix.ok_or("missing TileMatrix")?;
                let tile_col = self.tile_col.ok_or("missing TileCol")?;
                let tile_row = self.tile_row.ok_or("missing TileRow")?;
                let format = self.format.unwrap_or_else(|| "image/png".into());
                Ok(WmtsRequest::GetTile(WmtsGetTileParams {
                    layer,
                    tile_matrix_set,
                    tile_matrix,
                    tile_col,
                    tile_row,
                    format,
                }))
            }
            _ => Err(format!("unknown WMTS request type: {req_type}")),
        }
    }
}

fn build_wms_service() -> WmsService {
    let mut svc = WmsService::new("geo-toolbox WMS", "http://localhost:9378/wms");
    svc.add_layer(WmsLayer {
        name: "sentinel-2".into(),
        title: "Sentinel-2 NDVI".into(),
        abstract_: Some("Sentinel-2 satellite NDVI imagery".into()),
        keywords: vec![],
        wgs84_bbox: Some(geo_ogc::common::Wgs84Bbox {
            west: -180.0,
            south: -90.0,
            east: 180.0,
            north: 90.0,
        }),
        crs: vec!["EPSG:4326".into()],
        queryable: false,
        min_scale: None,
        max_scale: None,
        children: vec![],
    });
    svc.add_layer(WmsLayer {
        name: "landcover".into(),
        title: "Land Cover Classification".into(),
        abstract_: Some("Land cover types from GEE classification".into()),
        keywords: vec![],
        wgs84_bbox: Some(geo_ogc::common::Wgs84Bbox {
            west: 73.0,
            south: 18.0,
            east: 135.0,
            north: 54.0,
        }),
        crs: vec!["EPSG:4326".into()],
        queryable: true,
        min_scale: None,
        max_scale: None,
        children: vec![],
    });
    svc
}

fn build_wmts_service() -> WmtsService {
    let mut svc = WmtsService::new("geo-toolbox WMTS", "http://localhost:9378/wmts");
    svc.add_layer(WmtsDef {
        name: "sentinel-2".into(),
        title: "Sentinel-2 NDVI".into(),
        abstract_: Some("Sentinel-2 satellite NDVI imagery".into()),
        keywords: vec![],
        wgs84_bbox: Some(geo_ogc::common::Wgs84Bbox {
            west: -180.0,
            south: -90.0,
            east: 180.0,
            north: 90.0,
        }),
        crs: vec!["EPSG:4326".into(), "EPSG:3857".into()],
        tile_matrix_sets: vec!["EPSG:4326".into(), "EPSG:3857".into()],
        formats: vec!["image/png".into()],
        styles: vec!["default".into()],
        resource_url: Some("http://localhost:9378/wmts?request=GetTile&layer={layer}&TileMatrixSet={TileMatrixSet}&TileMatrix={TileMatrix}&TileCol={TileCol}&TileRow={TileRow}&format=image/png".into()),
    });
    svc.add_layer(WmtsDef {
        name: "landcover".into(),
        title: "Land Cover Classification".into(),
        abstract_: Some("Land cover types from GEE classification".into()),
        keywords: vec![],
        wgs84_bbox: Some(geo_ogc::common::Wgs84Bbox {
            west: 73.0,
            south: 18.0,
            east: 135.0,
            north: 54.0,
        }),
        crs: vec!["EPSG:4326".into(), "EPSG:3857".into()],
        tile_matrix_sets: vec!["EPSG:4326".into(), "EPSG:3857".into()],
        formats: vec!["image/png".into()],
        styles: vec!["default".into()],
        resource_url: Some("http://localhost:9378/wmts?request=GetTile&layer={layer}&TileMatrixSet={TileMatrixSet}&TileMatrix={TileMatrix}&TileCol={TileCol}&TileRow={TileRow}&format=image/png".into()),
    });
    svc.add_tile_matrix_set(geo_ogc::wmts::global_geodetic_tile_matrix_set());
    svc.add_tile_matrix_set(geo_ogc::wmts::global_mercator_tile_matrix_set());
    svc
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        registry: build_registry(),
    });
    let wms = Arc::new(build_wms_service());
    let wmts = Arc::new(build_wmts_service());

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/tools", get(list_tools))
        .route("/api/call/{tool}", post(call_tool))
        .route("/wms", get(wms_handler))
        .route("/wmts", get(wmts_handler))
        .with_state((state, wms, wmts));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9378").await.unwrap();
    tracing::info!("geo-server listening on http://0.0.0.0:9378");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn list_tools(
    State((state, _, _)): State<(Arc<AppState>, Arc<WmsService>, Arc<WmtsService>)>,
) -> Json<serde_json::Value> {
    Json(state.registry.generate_mcp_tools())
}

async fn call_tool(
    State((state, _, _)): State<(Arc<AppState>, Arc<WmsService>, Arc<WmtsService>)>,
    Path(tool): Path<String>,
    Json(args): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    match state.registry.dispatch(&tool, args).await {
        Ok(result) => Json(serde_json::json!({"ok": true, "data": result})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn wmts_handler(
    State((_, _, wmts)): State<(Arc<AppState>, Arc<WmsService>, Arc<WmtsService>)>,
    Query(query): Query<WmtsQuery>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    use axum::response::IntoResponse;

    let request = query.into_wmts_request().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid WMTS request: {e}"),
        )
    })?;

    match wmts.handle(&request) {
        Ok(response) => match response {
            WmtsResponse::Xml(xml) => Ok((
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "text/xml; charset=utf-8")],
                xml,
            )
                .into_response()),
            WmtsResponse::Tile { data, mime_type } => Ok((
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, mime_type.as_str())],
                data,
            )
                .into_response()),
        },
        Err(e) => {
            let xml = e.to_xml();
            Err((StatusCode::BAD_REQUEST, xml))
        }
    }
}

async fn wms_handler(
    State((_, wms, _)): State<(Arc<AppState>, Arc<WmsService>, Arc<WmtsService>)>,
    Query(query): Query<WmsQuery>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    use axum::response::IntoResponse;

    let request = query
        .into_wms_request()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid WMS request: {e}")))?;

    match wms.handle(&request) {
        Ok(response) => match response {
            WmsResponse::Xml(xml) => Ok((
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "text/xml; charset=utf-8")],
                xml,
            )
                .into_response()),
            WmsResponse::Image {
                data, mime_type, ..
            } => Ok((
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, mime_type.as_str())],
                String::from_utf8_lossy(&data).to_string(),
            )
                .into_response()),
        },
        Err(e) => {
            let xml = e.to_xml();
            Err((StatusCode::BAD_REQUEST, xml))
        }
    }
}
