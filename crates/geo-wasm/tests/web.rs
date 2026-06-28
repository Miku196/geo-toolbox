//! Browser integration tests for geo-wasm.
//! Test every WASM binding function.
//! Run: wasm-pack test --headless --chrome crates/geo-wasm

use geo_wasm::*;
use wasm_bindgen_test::*;

// Defaults to Node.js mode (wasm-bindgen-test).
// Switch to `wasm_bindgen_test_configure!(run_in_browser);` for headless browser tests.

// ─── Panic Hook ───────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_init_panic_hook() {
    // wasm-bindgen-test already sets a panic hook.
    // Just verify the function exists by wrapping in catch_unwind.
    let _ = std::panic::catch_unwind(|| {
        init_panic_hook();
    });
}

// ─── CRS ──────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_crs_list_all() {
    let engine = CrsEngine::new();
    let list = engine.list_all();
    assert!(!list.is_empty(), "CRS list should not be empty");
    assert!(list.contains("4326"), "should contain WGS84 (4326)");
}

#[wasm_bindgen_test]
fn test_crs_identity_transform() {
    let engine = CrsEngine::new();
    let r = engine.transform(4326, 4326, 116.3974, 39.9042).unwrap();
    assert_eq!(r[0], 116.3974, "lon unchanged in identity transform");
    assert_eq!(r[1], 39.9042, "lat unchanged in identity transform");
}

#[wasm_bindgen_test]
fn test_crs_wgs84_to_gcj02() {
    let engine = CrsEngine::new();
    let r = engine
        .transform(4326, EPSG_GCJ02, 116.3974, 39.9042)
        .unwrap();
    assert!(r[0] != 116.3974, "GCJ02 lon should differ from WGS84");
    assert!(r[1] != 39.9042, "GCJ02 lat should differ from WGS84");
}

#[wasm_bindgen_test]
fn test_crs_gcj02_wgs84_roundtrip() {
    let engine = CrsEngine::new();
    let gcj = engine
        .transform(4326, EPSG_GCJ02, 116.3974, 39.9042)
        .unwrap();
    let back = engine.transform(EPSG_GCJ02, 4326, gcj[0], gcj[1]).unwrap();
    let d_lon = (back[0] - 116.3974).abs();
    let d_lat = (back[1] - 39.9042).abs();
    assert!(d_lon < 0.001, "lon roundtrip error: {d_lon}");
    assert!(d_lat < 0.001, "lat roundtrip error: {d_lat}");
}

#[wasm_bindgen_test]
fn test_crs_wgs84_to_bd09() {
    let engine = CrsEngine::new();
    let r = engine
        .transform(4326, EPSG_BD09, 116.3974, 39.9042)
        .unwrap();
    assert!(
        (r[0] - 116.3974).abs() > 0.01,
        "BD09 lon should differ from WGS84"
    );
}

#[wasm_bindgen_test]
fn test_crs_mercator_roundtrip() {
    let engine = CrsEngine::new();
    let merc = engine.transform(4326, 3857, 116.0, 39.0).unwrap();
    assert!(merc[0] > 1_000_000.0, "Mercator x > 1M meters for lon=116");
    let back = engine.transform(3857, 4326, merc[0], merc[1]).unwrap();
    assert!((back[0] - 116.0).abs() < 0.01, "Mercator roundtrip lon");
    assert!((back[1] - 39.0).abs() < 0.01, "Mercator roundtrip lat");
}

#[wasm_bindgen_test]
fn test_crs_transform_batch() {
    let engine = CrsEngine::new();
    let coords = vec![0.0, 0.0, 10.0, 20.0];
    let r = engine.transform_batch(4326, 3857, &coords).unwrap();
    assert_eq!(r.len(), 4);
}

// ─── Geohash ──────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_geohash_encode() {
    let hash = geo_wasm::geohash_encode(116.3974, 39.9042, 8);
    assert_eq!(hash.len(), 8);
    assert!(
        hash.starts_with("wx4"),
        "Tiananmen geohash starts with wx4, got {hash}"
    );
}

#[wasm_bindgen_test]
fn test_geohash_decode() {
    let json_str = geo_wasm::geohash_decode("wx4g0bm9").unwrap();
    assert!(!json_str.is_empty());
    assert!(json_str.contains("lat"), "decoded JSON should contain lat");
}

#[wasm_bindgen_test]
fn test_geohash_encode_decode_roundtrip() {
    let hash = geo_wasm::geohash_encode(121.4737, 31.2304, 7);
    assert_eq!(hash.len(), 7);
    let _ = geo_wasm::geohash_decode(&hash).unwrap();
}

#[wasm_bindgen_test]
fn test_geohash_neighbors() {
    let json_str = geo_wasm::geohash_neighbors("wx4g0bm9").unwrap();
    assert!(!json_str.is_empty());
}

#[wasm_bindgen_test]
fn test_geohash_invalid_decode() {
    assert!(geo_wasm::geohash_decode("").is_err());
}

#[wasm_bindgen_test]
fn test_bbox_to_geohashes() {
    let json_str = geo_wasm::bbox_to_geohashes(116.3, 39.9, 116.41, 39.91, 6).unwrap();
    assert!(!json_str.is_empty());
}

// ─── NMEA Parsing ─────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_nmea_parse_valid_gga() {
    let r =
        geo_wasm::parse_nmea("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47");
    assert!(r.is_ok());
}

#[wasm_bindgen_test]
fn test_nmea_parse_garbage() {
    let r = geo_wasm::parse_nmea("not valid nmea");
    assert!(r.is_err(), "garbage input should error");
}

#[wasm_bindgen_test]
fn test_nmea_parse_empty() {
    let r = geo_wasm::parse_nmea("");
    assert!(r.is_err(), "empty string should error");
}

#[wasm_bindgen_test]
fn test_nmea_parse_batch() {
    let r = geo_wasm::parse_nmea_batch(
        "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\nnot valid",
    );
    assert!(r.is_ok());
}

#[wasm_bindgen_test]
fn test_validate_gps_fix_good() {
    let r = geo_wasm::validate_gps_fix(2.0, 8);
    assert!(r.is_ok());
}

#[wasm_bindgen_test]
fn test_validate_gps_fix_bad_hdop() {
    let r = geo_wasm::validate_gps_fix(10.0, 8);
    assert!(r.is_ok());
}

#[wasm_bindgen_test]
fn test_validate_gps_fix_few_sats() {
    let r = geo_wasm::validate_gps_fix(2.0, 2);
    assert!(r.is_ok());
}

#[wasm_bindgen_test]
fn test_validate_sensor_reading_good() {
    let r = geo_wasm::validate_sensor_reading("temperature", 25.0);
    assert!(r.is_ok());
}

#[wasm_bindgen_test]
fn test_validate_sensor_reading_bad_range() {
    let r = geo_wasm::validate_sensor_reading("humidity", 150.0);
    assert!(r.is_ok());
}

#[wasm_bindgen_test]
fn test_validate_coord_valid() {
    let r = geo_wasm::validate_coord(116.3974, 39.9042);
    assert!(r.is_ok(), "valid coord should not error");
}

#[wasm_bindgen_test]
fn test_validate_coord_invalid() {
    let r = geo_wasm::validate_coord(200.0, 100.0);
    assert!(r.is_ok(), "invalid coord returns Ok with validation result");
}

// ─── Spatial Operations ───────────────────────────────────────

fn rect_poly() -> String {
    r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]}"#.to_string()
}

#[wasm_bindgen_test]
fn test_compute_area() {
    let json = geo_wasm::compute_area(&rect_poly()).unwrap();
    assert!(json.contains("area"), "result should contain area key");
}

#[wasm_bindgen_test]
fn test_compute_bbox() {
    let json = geo_wasm::compute_bbox(&rect_poly()).unwrap();
    assert!(
        json.contains("minX") && json.contains("maxY"),
        "bbox JSON keys: {json}"
    );
}

#[wasm_bindgen_test]
fn test_compute_centroid() {
    let json = geo_wasm::compute_centroid(&rect_poly()).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_simplify_geometry() {
    let json = geo_wasm::simplify_geometry(&rect_poly(), 0.1).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_convex_hull() {
    let json = geo_wasm::convex_hull(&rect_poly()).unwrap();
    assert!(!json.is_empty());
}

// ─── Vector Operations ────────────────────────────────────────

#[wasm_bindgen_test]
fn test_compute_buffer() {
    let json = geo_wasm::compute_buffer(&rect_poly(), 0.1, "flat", None).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_compute_intersect() {
    let a = r#"{"type":"Polygon","coordinates":[[[0,0],[2,0],[2,2],[0,2],[0,0]]]}"#;
    let b = r#"{"type":"Polygon","coordinates":[[[1,0],[3,0],[3,2],[1,2],[1,0]]]}"#;
    let json = geo_wasm::compute_intersect(a, b).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_union_all() {
    let polys = r#"[
        {"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]},
        {"type":"Polygon","coordinates":[[[0.5,0],[1.5,0],[1.5,1],[0.5,1],[0.5,0]]]}
    ]"#;
    let json = geo_wasm::union_all(polys).unwrap();
    assert!(!json.is_empty());
}

// ─── Carbon ───────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_carbon_engine_new() {
    let _engine = CarbonEngine::new();
}

fn valid_carbon_geojson() -> String {
    r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","properties":{"class":"forest"},
         "geometry":{"type":"Polygon",
           "coordinates":[[[116.3,39.9],[116.41,39.9],[116.41,39.96],[116.3,39.96],[116.3,39.9]]]}}
    ]}"#
    .to_string()
}

#[wasm_bindgen_test]
fn test_carbon_calculate() {
    let engine = CarbonEngine::new();
    let geojson = valid_carbon_geojson();
    let csv = "category,factor_value,source\nforest,50.0,test\n";
    let result = engine.calculate(&geojson, csv, 2024);
    assert!(
        result.is_ok(),
        "carbon calculate failed: {:?}",
        result.err()
    );
}

#[wasm_bindgen_test]
fn test_carbon_calculate_with_json_factors() {
    let engine = CarbonEngine::new();
    let geojson = valid_carbon_geojson();
    let factors = r#"[
        {"category":"forest","factor_value":50.0,"source":"test"}
    ]"#;
    let result = engine.calculate_with_json_factors(&geojson, factors, 2024);
    assert!(
        result.is_ok(),
        "carbon calculate with json factors failed: {:?}",
        result.err()
    );
}

#[wasm_bindgen_test]
fn test_carbon_invalid_json_factors() {
    let engine = CarbonEngine::new();
    let geojson = r#"{"type":"FeatureCollection","features":[]}"#;
    let result = engine.calculate_with_json_factors(geojson, "not json", 2024);
    assert!(result.is_err(), "invalid JSON factors should error");
}

// ─── Tile ─────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_tile_latlon_to_tile_static() {
    let tile = geo_wasm::latlon_to_tile(116.3974, 39.9042, 10);
    assert!(tile.x > 0);
    assert!(tile.y > 0);
    assert_eq!(tile.z, 10);
}

#[wasm_bindgen_test]
fn test_tile_engine_latlon_to_tile() {
    let engine = TileEngine::new();
    let tile = engine.latlon_to_tile(116.3974, 39.9042, 10);
    assert!(tile.x > 0);
    assert_eq!(tile.z, 10);
}

#[wasm_bindgen_test]
fn test_tile_to_latlon() {
    let engine = TileEngine::new();
    let arr = engine.tile_to_latlon(843, 388, 10);
    assert_eq!(arr.length(), 2);
}

#[wasm_bindgen_test]
fn test_tile_bounds() {
    let engine = TileEngine::new();
    let arr = engine.tile_bounds(843, 388, 10);
    assert_eq!(arr.length(), 4);
}

#[wasm_bindgen_test]
fn test_tile_url() {
    let engine = TileEngine::new();
    // source names: "osm" / "gaode" / "tianditu"
    let url = engine.tile_url("osm", 843, 388, 10);
    assert!(
        url.contains("openstreetmap"),
        "OSM tile URL contains openstreetmap"
    );
}

#[wasm_bindgen_test]
fn test_tile_url_osm_free() {
    let url = geo_wasm::tile_url_osm(843, 388, 10);
    assert!(!url.is_empty());
}

#[wasm_bindgen_test]
fn test_tile_url_gaode() {
    let url = geo_wasm::tile_url_gaode(843, 388, 10);
    assert!(!url.is_empty());
}

// ─── Raster ───────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_ndvi_computation() {
    let nir = vec![0.8, 0.6, 0.9, 0.5];
    let red = vec![0.2, 0.3, 0.1, 0.4];
    let json = geo_wasm::compute_ndvi(red, nir, 2, 2).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("ndvi"));
}

#[wasm_bindgen_test]
fn test_band_add() {
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![5.0, 6.0, 7.0, 8.0];
    let json = geo_wasm::bandAdd(a, 2, 2, b, 2, 2).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_band_sub() {
    let a = vec![10.0, 9.0, 8.0, 7.0];
    let b = vec![1.0, 2.0, 3.0, 4.0];
    let json = geo_wasm::bandSub(a, 2, 2, b, 2, 2).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_band_mul() {
    let a = vec![2.0, 3.0];
    let b = vec![4.0, 5.0];
    let json = geo_wasm::bandMul(a, 1, 2, b, 1, 2).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_band_div() {
    let a = vec![10.0, 20.0];
    let b = vec![2.0, 5.0];
    let json = geo_wasm::bandDiv(a, 1, 2, b, 1, 2).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_band_threshold() {
    let data = vec![0.1, 0.6, 0.8, 0.3];
    let json = geo_wasm::band_threshold(data, 2, 2, 0.5).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_resample_nearest() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let result = geo_wasm::resample_nearest(data, 3, 3, 2, 2, None);
    assert_eq!(result.len(), 4);
}

#[wasm_bindgen_test]
fn test_resample_cubic() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let result = geo_wasm::resample_cubic(data, 3, 3, 2, 2, None);
    assert_eq!(result.len(), 4);
}

#[wasm_bindgen_test]
fn test_compute_zonal_stats() {
    let values = vec![1.0, 2.0, 3.0, 4.0];
    let zones = vec![1, 1, 2, 2];
    let json = geo_wasm::compute_zonal_stats(values, zones, 3, None).unwrap();
    assert!(!json.is_empty());
}

#[wasm_bindgen_test]
fn test_ndvi_difference() {
    let prev = vec![0.5, 0.6];
    let curr = vec![0.7, 0.8];
    let json = geo_wasm::ndvi_difference(prev, 1, 2, curr, 1, 2).unwrap();
    assert!(!json.is_empty());
}

// ─── Utils ────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_get_version() {
    let v = geo_wasm::get_version();
    assert!(!v.is_empty());
}

#[wasm_bindgen_test]
fn test_get_build_info() {
    let info = geo_wasm::get_build_info();
    assert!(!info.is_empty());
}

#[wasm_bindgen_test]
fn test_get_memory_stats() {
    let stats = geo_wasm::get_memory_stats();
    assert!(stats.contains("total") || stats.contains("memory") || stats.contains("heap"));
}

#[wasm_bindgen_test]
fn test_log_to_console() {
    geo_wasm::log_to_console("browser test: hello from geo-wasm!");
}
