//! Browser-side spatial indexing — H3 hexagonal grids, R-tree, Quadtree.
//!
//! Thin WASM wrappers around [`geo_index`].

use geo_core::types::BBox;
use wasm_bindgen::prelude::*;

// ── H3 Hexagonal Grid ───────────────────────────────────────────

/// Convert a lat/lon coordinate to an H3 cell index at the given resolution (0-15).
/// Returns the H3 index as a hex string (e.g. "88283082d1fffff").
#[wasm_bindgen(js_name = latlonToH3)]
pub fn latlon_to_h3(lat: f64, lon: f64, resolution: u8) -> Result<String, JsValue> {
    let idx = geo_index::latlon_to_h3(lat, lon, resolution)
        .ok_or_else(|| JsValue::from_str("Invalid lat/lon or resolution"))?;
    Ok(geo_index::h3_to_string(&idx))
}

/// Convert an H3 index string to a GeoJSON polygon representing the cell boundary.
#[wasm_bindgen(js_name = h3ToGeoJSON)]
pub fn h3_to_geojson(h3_str: &str) -> Result<String, JsValue> {
    let idx = geo_index::h3_from_string(h3_str)
        .ok_or_else(|| JsValue::from_str("Invalid H3 index string"))?;
    let geojson = geo_index::h3_to_geojson(&idx);
    serde_json::to_string(&geojson).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the center coordinate of an H3 cell.
/// Returns JSON: `[lon, lat]`
#[wasm_bindgen(js_name = h3Center)]
pub fn h3_center(h3_str: &str) -> Result<String, JsValue> {
    let idx = geo_index::h3_from_string(h3_str)
        .ok_or_else(|| JsValue::from_str("Invalid H3 index string"))?;
    let (lat, lon) = idx.to_latlon();
    serde_json::to_string(&[lon, lat]).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the boundary vertices of an H3 cell.
/// Returns JSON array of [lon, lat] pairs.
#[wasm_bindgen(js_name = h3Boundary)]
pub fn h3_boundary(h3_str: &str) -> Result<String, JsValue> {
    let idx = geo_index::h3_from_string(h3_str)
        .ok_or_else(|| JsValue::from_str("Invalid H3 index string"))?;
    let boundary: Vec<[f64; 2]> = idx
        .to_boundary()
        .into_iter()
        .map(|(lon, lat)| [lon, lat])
        .collect();
    serde_json::to_string(&boundary).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get all neighbor H3 cells of a given cell (k-ring of depth 1).
/// Returns JSON array of H3 index strings.
#[wasm_bindgen(js_name = h3Neighbors)]
pub fn h3_neighbors(h3_str: &str) -> Result<String, JsValue> {
    let idx = geo_index::h3_from_string(h3_str)
        .ok_or_else(|| JsValue::from_str("Invalid H3 index string"))?;
    let nbs: Vec<String> = idx
        .neighbors()
        .iter()
        .map(geo_index::h3_to_string)
        .collect();
    serde_json::to_string(&nbs).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Cover a bounding box with H3 cells at a given resolution.
/// Returns JSON array of H3 index strings.
#[wasm_bindgen(js_name = h3CoverBbox)]
pub fn h3_cover_bbox(
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    resolution: u8,
) -> Result<String, JsValue> {
    let bbox = BBox::new(min_lon, min_lat, max_lon, max_lat);
    let cells: Vec<String> = geo_index::h3_cover_bbox(&bbox, resolution)
        .iter()
        .map(geo_index::h3_to_string)
        .collect();
    serde_json::to_string(&cells).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Create a disk (approximate circle) of H3 cells around a center point.
/// Returns JSON array of H3 index strings.
#[wasm_bindgen(js_name = h3GridDisk)]
pub fn h3_grid_disk(
    center_lat: f64,
    center_lon: f64,
    radius_km: f64,
    resolution: u8,
) -> Result<String, JsValue> {
    let cells: Vec<String> = geo_index::h3_grid_disk(center_lat, center_lon, radius_km, resolution)
        .iter()
        .map(geo_index::h3_to_string)
        .collect();
    serde_json::to_string(&cells).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the area of an H3 hexagon at a given resolution (in km²).
#[wasm_bindgen(js_name = h3HexAreaKm2)]
pub fn h3_hex_area_km2(resolution: u8) -> Result<JsValue, JsValue> {
    let area =
        h3_hex_area_km2_inner(resolution).ok_or_else(|| JsValue::from_str("Invalid resolution"))?;
    Ok(JsValue::from_f64(area))
}

fn h3_hex_area_km2_inner(resolution: u8) -> Option<f64> {
    geo_index::h3_hex_area_km2(resolution)
}

/// Get the edge length of an H3 hexagon at a given resolution (in km).
#[wasm_bindgen(js_name = h3EdgeLengthKm)]
pub fn h3_edge_length_km(resolution: u8) -> Result<JsValue, JsValue> {
    let len = h3_edge_length_km_inner(resolution)
        .ok_or_else(|| JsValue::from_str("Invalid resolution"))?;
    Ok(JsValue::from_f64(len))
}

fn h3_edge_length_km_inner(resolution: u8) -> Option<f64> {
    geo_index::h3_edge_length_km(resolution)
}

/// Get the total number of H3 cells at a given resolution.
/// Returns the number as a string (to avoid JS integer precision issues).
#[wasm_bindgen(js_name = h3NumHexagons)]
pub fn h3_num_hexagons(resolution: u8) -> Result<JsValue, JsValue> {
    let n =
        h3_num_hexagons_inner(resolution).ok_or_else(|| JsValue::from_str("Invalid resolution"))?;
    Ok(JsValue::from_str(&n.to_string()))
}

fn h3_num_hexagons_inner(resolution: u8) -> Option<u64> {
    geo_index::h3_num_hexagons(resolution)
}

// ── R-tree Spatial Index ────────────────────────────────────────

/// Build an R-tree from a list of bounding boxes and query it.
/// Returns JSON array of index positions whose bbox intersects the query bbox.
///
/// Parameters:
/// - `bboxes`: flat array [minX0, minY0, maxX0, maxY0, minX1, minY1, ...]
/// - `query_min_x`, `query_min_y`, `query_max_x`, `query_max_y`: query bbox
#[wasm_bindgen(js_name = rtreeQuery)]
pub fn rtree_query(
    bboxes: Vec<f64>,
    query_min_x: f64,
    query_min_y: f64,
    query_max_x: f64,
    query_max_y: f64,
) -> Result<String, JsValue> {
    let box_list: Vec<BBox> = bboxes
        .chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| BBox::new(c[0], c[1], c[2], c[3]))
        .collect();

    let mut tree = geo_index::RTree::new();
    tree.load(box_list);

    let query_bbox = BBox::new(query_min_x, query_min_y, query_max_x, query_max_y);
    let results = tree.query(&query_bbox);
    serde_json::to_string(&results).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Build an R-tree and return the k-nearest neighbors of a query bbox.
/// Returns JSON array of `[index, distance]` pairs.
#[wasm_bindgen(js_name = rtreeKnn)]
pub fn rtree_knn(
    bboxes: Vec<f64>,
    query_min_x: f64,
    query_min_y: f64,
    query_max_x: f64,
    query_max_y: f64,
    k: usize,
) -> Result<String, JsValue> {
    let box_list: Vec<BBox> = bboxes
        .chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| BBox::new(c[0], c[1], c[2], c[3]))
        .collect();

    let mut tree = geo_index::RTree::new();
    tree.load(box_list);

    let query_bbox = BBox::new(query_min_x, query_min_y, query_max_x, query_max_y);
    let results = tree.knn(&query_bbox, k);
    serde_json::to_string(&results).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Build an R-tree and check which bboxes contain a point.
/// Returns JSON array of indices.
#[wasm_bindgen(js_name = rtreeQueryPoint)]
pub fn rtree_query_point(bboxes: Vec<f64>, x: f64, y: f64) -> Result<String, JsValue> {
    let box_list: Vec<BBox> = bboxes
        .chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| BBox::new(c[0], c[1], c[2], c[3]))
        .collect();

    let mut tree = geo_index::RTree::new();
    tree.load(box_list);

    let results = tree.query_point(x, y);
    serde_json::to_string(&results).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ── Quadtree Spatial Index ──────────────────────────────────────

/// Build a Quadtree from a list of bounding boxes and query it.
/// Returns JSON array of index positions whose bbox intersects the query bbox.
#[wasm_bindgen(js_name = quadtreeQuery)]
pub fn quadtree_query(
    bboxes: Vec<f64>,
    query_min_x: f64,
    query_min_y: f64,
    query_max_x: f64,
    query_max_y: f64,
) -> Result<String, JsValue> {
    let box_list: Vec<BBox> = bboxes
        .chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| BBox::new(c[0], c[1], c[2], c[3]))
        .collect();

    let mut tree = geo_index::Quadtree::new();
    tree.load(box_list);

    let query_bbox = BBox::new(query_min_x, query_min_y, query_max_x, query_max_y);
    let results = tree.query(&query_bbox);
    serde_json::to_string(&results).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h3_latlon() {
        let h3 = latlon_to_h3(30.5, 104.0, 6).unwrap();
        assert!(h3.len() > 10);
        // Decode back: center should be near the original point
        let center = h3_center(&h3).unwrap();
        let arr: Vec<f64> = serde_json::from_str(&center).unwrap();
        assert_eq!(arr.len(), 2);
        // Resolution 6 hex area ≈ 35 km², center within ~0.03°
        assert!(
            (arr[0] - 104.0).abs() < 0.5,
            "lon offset too large: {}",
            arr[0]
        );
        assert!(
            (arr[1] - 30.5).abs() < 0.5,
            "lat offset too large: {}",
            arr[1]
        );
    }

    #[test]
    fn test_h3_geojson() {
        let h3 = latlon_to_h3(30.5, 104.0, 6).unwrap();
        let geojson = h3_to_geojson(&h3).unwrap();
        assert!(geojson.contains("Polygon"));
    }

    #[test]
    fn test_h3_neighbors() {
        let h3 = latlon_to_h3(30.5, 104.0, 4).unwrap();
        let nbs = h3_neighbors(&h3).unwrap();
        let arr: Vec<String> = serde_json::from_str(&nbs).unwrap();
        assert!(!arr.is_empty());
    }

    #[test]
    fn test_h3_cover_bbox() {
        let cells = h3_cover_bbox(104.0, 30.5, 104.1, 30.6, 5).unwrap();
        let arr: Vec<String> = serde_json::from_str(&cells).unwrap();
        assert!(!arr.is_empty());
    }

    #[test]
    fn test_h3_grid_disk() {
        let cells = h3_grid_disk(30.5, 104.0, 5.0, 6).unwrap();
        let arr: Vec<String> = serde_json::from_str(&cells).unwrap();
        assert!(!arr.is_empty());
    }

    #[test]
    fn test_rtree_query() {
        // 4 bboxes: [minX,minY,maxX,maxY, ...]
        let bboxes = vec![
            0.0, 0.0, 10.0, 10.0, 5.0, 5.0, 15.0, 15.0, 20.0, 20.0, 30.0, 30.0, 8.0, 8.0, 12.0,
            12.0,
        ];
        let result = rtree_query(bboxes, 5.0, 5.0, 11.0, 11.0).unwrap();
        let indices: Vec<usize> = serde_json::from_str(&result).unwrap();
        assert!(indices.contains(&0));
        assert!(indices.contains(&1));
        assert!(indices.contains(&3));
    }

    #[test]
    fn test_quadtree_query() {
        let bboxes = vec![
            0.0, 0.0, 10.0, 10.0, 5.0, 5.0, 15.0, 15.0, 20.0, 20.0, 30.0, 30.0,
        ];
        let result = quadtree_query(bboxes, 7.0, 7.0, 12.0, 12.0).unwrap();
        let indices: Vec<usize> = serde_json::from_str(&result).unwrap();
        assert!(!indices.is_empty());
    }

    #[test]
    fn test_h3_metadata() {
        let area = h3_hex_area_km2_inner(6).unwrap();
        assert!(area > 0.0);
        let edge = h3_edge_length_km_inner(6).unwrap();
        assert!(edge > 0.0);
        let n = h3_num_hexagons_inner(6).unwrap();
        assert!(n > 0);
    }
}
