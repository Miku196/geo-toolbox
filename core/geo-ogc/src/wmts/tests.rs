use super::*;
use crate::common::Wgs84Bbox;
use std::sync::Arc;

fn make_service() -> WmtsService {
    let mut svc = WmtsService::new("Test WMTS", "https://example.com/wmts");
    svc.add_layer(WmtsLayer {
        name: "sentinel-2".into(),
        title: "Sentinel-2 NDVI".into(),
        abstract_: Some("Sentinel-2 satellite NDVI imagery".into()),
        keywords: vec!["sentinel".into(), "ndvi".into()],
        wgs84_bbox: Some(Wgs84Bbox::new(-180.0, -90.0, 180.0, 90.0)),
        crs: vec!["EPSG:4326".into(), "EPSG:3857".into()],
        tile_matrix_sets: vec!["EPSG:4326".into(), "EPSG:3857".into()],
        formats: vec!["image/png".into()],
        styles: vec!["default".into()],
        resource_url: Some(
            "https://example.com/tiles/{TileMatrixSet}/{TileMatrix}/{TileCol}/{TileRow}.png".into(),
        ),
        renderer: None,
        mvt_source: None,
    });
    svc.add_tile_matrix_set(global_geodetic_tile_matrix_set());
    svc.add_tile_matrix_set(global_mercator_tile_matrix_set());
    svc
}

#[test]
fn test_get_capabilities_xml() {
    let svc = make_service();
    let xml = svc.build_capabilities_xml();
    assert!(xml.contains("WMTS"));
    assert!(xml.contains("sentinel-2"));
    assert!(xml.contains("EPSG:4326"));
    assert!(xml.contains("EPSG:3857"));
    assert!(xml.contains("TileMatrixSet"));
}

#[test]
fn test_get_tile_valid() {
    let svc = make_service();
    let params = WmtsGetTileParams {
        layer: "sentinel-2".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "5".into(),
        tile_col: 16,
        tile_row: 8,
        format: "image/png".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(result.is_ok());
}

#[test]
fn test_get_tile_unknown_layer() {
    let svc = make_service();
    let params = WmtsGetTileParams {
        layer: "nonexistent".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "5".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(result.is_err());
}

#[test]
fn test_global_geodetic_tms() {
    let tms = global_geodetic_tile_matrix_set();
    assert_eq!(tms.identifier, "EPSG:4326");
    assert_eq!(tms.tile_matrices.len(), 22);
    assert_eq!(tms.tile_matrices[0].matrix_width, 2);
    assert_eq!(tms.tile_matrices[0].matrix_height, 1);
    assert_eq!(tms.tile_matrices[21].matrix_width, 2u32.pow(21) * 2);
}

#[test]
fn test_global_mercator_tms() {
    let tms = global_mercator_tile_matrix_set();
    assert_eq!(tms.identifier, "EPSG:3857");
    assert_eq!(tms.tile_matrices.len(), 22);
    assert_eq!(tms.tile_matrices[0].matrix_width, 1);
    assert_eq!(tms.tile_matrices[0].matrix_height, 1);
}

// ── TileCache tests ──

#[test]
fn test_tile_cache_insert_get() {
    let cache = TileCache::new(100);
    let data = vec![1u8, 2, 3, 4];
    cache.insert("nlcd", "EPSG:4326", "0", 0, 0, "image/png", data.clone());
    let result = cache.get("nlcd", "EPSG:4326", "0", 0, 0, "image/png");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), &[1, 2, 3, 4]);
}

#[test]
fn test_tile_cache_miss() {
    let cache = TileCache::new(100);
    assert!(cache
        .get("nlcd", "EPSG:4326", "0", 0, 0, "image/png")
        .is_none());
}

#[test]
fn test_tile_cache_pre_cache() {
    let cache = TileCache::new(10000);
    let count = cache.pre_cache("nlcd", "EPSG:4326", "image/png", 2, 2);
    assert!(count > 0);
    assert_eq!(cache.len(), count as usize);
    let result = cache.get("nlcd", "EPSG:4326", "0", 0, 0, "image/png");
    assert!(result.is_some());
}

#[test]
fn test_tile_cache_clear() {
    let cache = TileCache::new(100);
    cache.insert("nlcd", "EPSG:4326", "0", 0, 0, "image/png", vec![1, 2, 3]);
    assert!(!cache.is_empty());
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn test_tile_cache_default() {
    let cache = TileCache::default();
    assert_eq!(cache.max_entries, 10_000);
    assert!(cache.is_empty());
}

#[test]
fn test_wmts_cache_integration() {
    let mut svc = WmtsService::new("Test", "http://localhost/test");
    svc.add_layer(WmtsLayer {
        name: "test_layer".into(),
        title: "Test".into(),
        abstract_: None,
        keywords: vec![],
        wgs84_bbox: None,
        crs: vec![],
        tile_matrix_sets: vec!["EPSG:4326".into()],
        formats: vec!["image/png".into()],
        styles: vec![],
        resource_url: None,
        renderer: None,
        mvt_source: None,
    });
    svc.add_tile_matrix_set(TileMatrixSet {
        identifier: "EPSG:4326".into(),
        bounding_box: Wgs84Bbox {
            west: -180.0,
            south: -90.0,
            east: 180.0,
            north: 90.0,
        },
        supported_crs: "EPSG:4326".into(),
        tile_matrices: vec![TileMatrix {
            identifier: "0".into(),
            scale_denominator: 2.0,
            top_left_x: -180.0,
            top_left_y: 90.0,
            tile_width: 256,
            tile_height: 256,
            matrix_width: 1,
            matrix_height: 1,
        }],
    });
    // Pre-cache and verify tile served from cache
    svc.cache.insert(
        "test_layer",
        "EPSG:4326",
        "0",
        0,
        0,
        "image/png",
        vec![0xFF; 256 * 256 * 4],
    );
    let params = WmtsGetTileParams {
        layer: "test_layer".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "0".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(result.is_ok());
}

#[test]
fn test_wmts_renderer_elevation() {
    let mut svc = WmtsService::new("Test", "http://localhost/test");
    svc.add_layer(WmtsLayer {
        name: "elevation".into(),
        title: "Elevation".into(),
        abstract_: None,
        keywords: vec![],
        wgs84_bbox: None,
        crs: vec![],
        tile_matrix_sets: vec!["EPSG:4326".into()],
        formats: vec!["image/png".into()],
        styles: vec![],
        resource_url: None,
        renderer: Some(renderers::elevation),
        mvt_source: None,
    });
    svc.add_tile_matrix_set(TileMatrixSet {
        identifier: "EPSG:4326".into(),
        bounding_box: Wgs84Bbox {
            west: -180.0,
            south: -90.0,
            east: 180.0,
            north: 90.0,
        },
        supported_crs: "EPSG:4326".into(),
        tile_matrices: vec![TileMatrix {
            identifier: "0".into(),
            scale_denominator: 2.0,
            top_left_x: -180.0,
            top_left_y: 90.0,
            tile_width: 256,
            tile_height: 256,
            matrix_width: 1,
            matrix_height: 1,
        }],
    });
    let params = WmtsGetTileParams {
        layer: "elevation".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "0".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params)).unwrap();
    match result {
        WmtsResponse::Tile { data, .. } => {
            assert_eq!(data.len(), 256 * 256 * 4);
            assert!(data.iter().any(|&b| b > 0), "tile should not be all zeros");
        }
        _ => panic!("Expected Tile response"),
    }
}

#[test]
fn test_renderer_fallback() {
    let mut svc = WmtsService::new("Test", "http://localhost/test");
    svc.add_layer(WmtsLayer {
        name: "no_renderer".into(),
        title: "No Renderer".into(),
        abstract_: None,
        keywords: vec![],
        wgs84_bbox: None,
        crs: vec![],
        tile_matrix_sets: vec!["EPSG:4326".into()],
        formats: vec!["image/png".into()],
        styles: vec![],
        resource_url: None,
        renderer: None,
        mvt_source: None,
    });
    svc.add_tile_matrix_set(TileMatrixSet {
        identifier: "EPSG:4326".into(),
        bounding_box: Wgs84Bbox {
            west: -180.0,
            south: -90.0,
            east: 180.0,
            north: 90.0,
        },
        supported_crs: "EPSG:4326".into(),
        tile_matrices: vec![TileMatrix {
            identifier: "0".into(),
            scale_denominator: 2.0,
            top_left_x: -180.0,
            top_left_y: 90.0,
            tile_width: 256,
            tile_height: 256,
            matrix_width: 1,
            matrix_height: 1,
        }],
    });
    let params = WmtsGetTileParams {
        layer: "no_renderer".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "0".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(result.is_ok());
}

// ── MVT tile tests ──

fn make_service_with_mvt() -> WmtsService {
    use crate::mvt_source::JsonFeatureProvider;

    let geojson = r#"{
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": {"name": "test-point"},
            "geometry": {"type": "Point", "coordinates": [104.0, 30.0]}
        }]
    }"#;
    let provider = JsonFeatureProvider::new(geojson).unwrap();

    let mut svc = WmtsService::new("Test MVT", "http://localhost/wmts");
    svc.add_layer(WmtsLayer {
        name: "test-mvt".into(),
        title: "Test MVT Layer".into(),
        abstract_: None,
        keywords: vec![],
        wgs84_bbox: Some(Wgs84Bbox::new(100.0, 20.0, 110.0, 40.0)),
        crs: vec!["EPSG:3857".into()],
        tile_matrix_sets: vec!["EPSG:3857".into()],
        formats: vec![
            "image/png".into(),
            "application/vnd.mapbox-vector-tile".into(),
        ],
        styles: vec!["default".into()],
        resource_url: Some(
            "http://localhost/wmts?request=GetTile&layer={layer}&TileMatrixSet={TileMatrixSet}&TileMatrix={TileMatrix}&TileCol={TileCol}&TileRow={TileRow}&format={format}"
                .into(),
        ),
        renderer: None,
        mvt_source: Some(Arc::new(provider)),
    });
    svc.add_tile_matrix_set(global_mercator_tile_matrix_set());
    svc
}

#[test]
fn test_mvt_get_tile() {
    let svc = make_service_with_mvt();
    let params = WmtsGetTileParams {
        layer: "test-mvt".into(),
        tile_matrix_set: "EPSG:3857".into(),
        tile_matrix: "10".into(),
        tile_col: 844,
        tile_row: 385,
        format: "application/vnd.mapbox-vector-tile".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(result.is_ok());
    let response = result.unwrap();
    match response {
        WmtsResponse::Tile { data, mime_type } => {
            assert!(!data.is_empty(), "MVT tile data should not be empty");
            assert_eq!(
                mime_type, "application/vnd.mapbox-vector-tile",
                "MIME type should be MVT"
            );
        }
        _ => panic!("Expected Tile response, got Xml"),
    }
}

#[test]
fn test_mvt_get_tile_png_fallback() {
    // When requesting PNG format, the MVT layer should fall through to checkerboard
    let svc = make_service_with_mvt();
    let params = WmtsGetTileParams {
        layer: "test-mvt".into(),
        tile_matrix_set: "EPSG:3857".into(),
        tile_matrix: "10".into(),
        tile_col: 844,
        tile_row: 385,
        format: "image/png".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(result.is_ok());
}

#[test]
fn test_mvt_get_tile_layer_not_found() {
    let svc = make_service_with_mvt();
    let params = WmtsGetTileParams {
        layer: "nonexistent".into(),
        tile_matrix_set: "EPSG:3857".into(),
        tile_matrix: "10".into(),
        tile_col: 844,
        tile_row: 385,
        format: "application/vnd.mapbox-vector-tile".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(result.is_err());
}

#[test]
fn test_mvt_layer_empty_source() {
    // A layer without mvt_source should return error for MVT format
    let svc = make_service(); // No MVT source
    let params = WmtsGetTileParams {
        layer: "sentinel-2".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "0".into(),
        tile_col: 0,
        tile_row: 0,
        format: "application/vnd.mapbox-vector-tile".into(),
    };
    let result = svc.handle(&WmtsRequest::GetTile(params));
    assert!(
        result.is_err(),
        "Layer without MVT source should error on MVT request"
    );
}

#[test]
fn test_pmtiles_archive_build() {
    let svc = make_service_with_mvt();
    let cursor = std::io::Cursor::new(Vec::new());
    let result = svc.build_pmtiles_archive("test-mvt", cursor);
    assert!(result.is_ok(), "PMTiles archive build should succeed");
    let pm_writer = result.unwrap();
    assert!(pm_writer.finish().is_ok());
}

#[test]
fn test_pmtiles_archive_layer_not_found() {
    let svc = make_service_with_mvt();
    let writer = std::io::Cursor::new(Vec::new());
    let result = svc.build_pmtiles_archive("nonexistent", writer);
    assert!(result.is_err());
}

#[test]
fn test_estimate_mvt_tile_count() {
    let svc = make_service_with_mvt();
    let count = svc.estimate_mvt_tile_count("test-mvt");
    assert!(count.is_some(), "Should get a tile count estimate");
    assert!(count.unwrap() > 0, "Should have at least 1 non-empty tile");
}

#[test]
fn test_estimate_mvt_tile_count_no_mvt() {
    let svc = make_service(); // No MVT source
    let count = svc.estimate_mvt_tile_count("sentinel-2");
    assert!(
        count.is_none(),
        "Layer without MVT source should return None"
    );
}

// ── Honest cache & validation tests ──

#[test]
fn test_tile_cache_format_distinct() {
    // The cache key must include the output format so PNG and MVT for the
    // same tile do not collide (regression for PNG/MVT cross-talk).
    let cache = TileCache::new(100);
    cache.insert("layer", "EPSG:4326", "3", 2, 1, "image/png", vec![1, 2, 3]);
    assert!(cache
        .get("layer", "EPSG:4326", "3", 2, 1, "image/png")
        .is_some());
    assert!(
        cache
            .get(
                "layer",
                "EPSG:4326",
                "3",
                2,
                1,
                "application/vnd.mapbox-vector-tile"
            )
            .is_none(),
        "MVT request must not hit the PNG cache entry"
    );
    assert_eq!(cache.len(), 1);
}

fn make_single_zoom_service() -> WmtsService {
    let mut svc = WmtsService::new("Test", "http://localhost/test");
    svc.add_layer(WmtsLayer {
        name: "zonly".into(),
        title: "Zoom-only".into(),
        abstract_: None,
        keywords: vec![],
        wgs84_bbox: None,
        crs: vec![],
        tile_matrix_sets: vec!["EPSG:4326".into()],
        formats: vec!["image/png".into()],
        styles: vec![],
        resource_url: None,
        renderer: None,
        mvt_source: None,
    });
    svc.add_tile_matrix_set(TileMatrixSet {
        identifier: "EPSG:4326".into(),
        bounding_box: Wgs84Bbox {
            west: -180.0,
            south: -90.0,
            east: 180.0,
            north: 90.0,
        },
        supported_crs: "EPSG:4326".into(),
        tile_matrices: vec![TileMatrix {
            identifier: "0".into(),
            scale_denominator: 2.0,
            top_left_x: -180.0,
            top_left_y: 90.0,
            tile_width: 256,
            tile_height: 256,
            matrix_width: 1,
            matrix_height: 1,
        }],
    });
    svc
}

#[test]
fn test_get_tile_zoom_out_of_bounds_errors() {
    // A tile_matrix that does not exist in the tile matrix set must be
    // rejected, not silently downgraded to zoom 0.
    let svc = make_single_zoom_service();
    let params = WmtsGetTileParams {
        layer: "zonly".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "5".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };
    assert!(svc.handle(&WmtsRequest::GetTile(params)).is_err());
}

#[test]
fn test_get_tile_invalid_zoom_garbage_errors() {
    let svc = make_single_zoom_service();
    let params = WmtsGetTileParams {
        layer: "zonly".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "abc".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };
    assert!(svc.handle(&WmtsRequest::GetTile(params)).is_err());
}

#[test]
fn test_get_tile_registered_nonnumeric_matrix_errors_for_raster_renderer() {
    // A TileMatrix may be registered by string identity, but the raster
    // renderer accepts only a numeric zoom and must not silently use zoom 0.
    let mut svc = make_single_zoom_service();
    svc.tile_matrix_sets[0].tile_matrices[0].identifier = "custom-level".into();
    let params = WmtsGetTileParams {
        layer: "zonly".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "custom-level".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };

    let error = svc
        .handle(&WmtsRequest::GetTile(params))
        .expect_err("a raster renderer requires a numeric TileMatrix zoom");
    assert_eq!(error.exceptions[0].code, "InvalidParameterValue");
    assert!(error.exceptions[0].text.contains("custom-level"));
}

#[test]
fn test_get_tile_column_out_of_range_errors() {
    let svc = make_single_zoom_service();
    let params = WmtsGetTileParams {
        layer: "zonly".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "0".into(),
        tile_col: 7, // matrix_width is 1
        tile_row: 0,
        format: "image/png".into(),
    };
    assert!(svc.handle(&WmtsRequest::GetTile(params)).is_err());
}

#[test]
fn test_get_tile_unsupported_format_errors() {
    // An arbitrary format must not pass through unchanged.
    let svc = make_single_zoom_service();
    let params = WmtsGetTileParams {
        layer: "zonly".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "0".into(),
        tile_col: 0,
        tile_row: 0,
        format: "application/x-arbitrary".into(),
    };
    assert!(svc.handle(&WmtsRequest::GetTile(params)).is_err());
}

#[test]
fn test_wmts_get_tile_populates_cache() {
    // The previously "read-only, always-miss" cache must now really insert.
    let svc = make_single_zoom_service();
    let params = WmtsGetTileParams {
        layer: "zonly".into(),
        tile_matrix_set: "EPSG:4326".into(),
        tile_matrix: "0".into(),
        tile_col: 0,
        tile_row: 0,
        format: "image/png".into(),
    };
    svc.handle(&WmtsRequest::GetTile(params)).unwrap();
    assert!(
        !svc.cache.is_empty(),
        "serving a tile must populate the cache"
    );
}
