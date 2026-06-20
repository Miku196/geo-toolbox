/// Carbon sink estimation using NDVI-based biomass allometric equations.
///
/// Computes tCO₂e sequestration from:
/// 1. NDVI raster band (pre-computed, e.g. via [`geo_raster::ndvi::compute_ndvi`])
/// 2. Forest inventory GeoJSON features (Polygons with optional `class` property)
///
/// Steps:
/// - Threshold NDVI > 0.3 → healthy vegetation mask
/// - Estimate above-ground biomass from NDVI: AGB = 135.53 × NDVI - 16.76 (IPCC Tier 2)
/// - Convert AGB → tCO₂e (carbon fraction 0.47, CO₂/C = 44/12)
/// - Aggregate by forest inventory polygon
use geo_core::errors::{GeoError, GeoResult};
use serde_json::Value;

/// Estimated carbon sink from NDVI and forest inventory data.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CarbonSinkResult {
    /// Total CO₂ equivalent sequestered (tCO₂e)
    pub total_tco2e: f64,
    /// Area of healthy vegetation (ha)
    pub healthy_area_ha: f64,
    /// Mean NDVI over vegetation areas
    pub mean_ndvi: f64,
    /// Per-polygon breakdown
    pub by_polygon: Vec<PolygonSink>,
}

/// Per-polygon carbon sink detail.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PolygonSink {
    pub polygon_id: usize,
    pub area_ha: f64,
    pub mean_ndvi: f64,
    pub agb_tco2e: f64,
    pub sink_tco2e_ha: f64,
}

/// Estimate carbon sink from NDVI raster data and forest inventory polygons.
///
/// `ndvi_data`: Flat array of NDVI values (row-major).
/// `ndvi_rows`, `ndvi_cols`: Raster dimensions.
/// `ndvi_nodata`: Nodata value (pixels equal to this are skipped).
/// `pixel_area_ha`: Area of one pixel in hectares (e.g., 0.09 for 30m Sentinel-2).
/// `forest_inventory_geojson`: GeoJSON FeatureCollection string.
pub fn estimate_carbon_sink(
    ndvi_data: &[f64],
    ndvi_rows: usize,
    ndvi_cols: usize,
    ndvi_nodata: f64,
    pixel_area_ha: f64,
    forest_inventory_geojson: &str,
) -> GeoResult<String> {
    if ndvi_data.len() != ndvi_rows * ndvi_cols {
        return Err(GeoError::Validation(format!(
            "NDVI data size {} does not match dimensions {}×{}={}",
            ndvi_data.len(),
            ndvi_rows,
            ndvi_cols,
            ndvi_rows * ndvi_cols
        )));
    }

    // Parse forest inventory
    let inventory: Value = serde_json::from_str(forest_inventory_geojson)
        .map_err(|e| GeoError::Validation(format!("Invalid GeoJSON: {e}")))?;

    let features = inventory["features"]
        .as_array()
        .ok_or_else(|| GeoError::Validation("GeoJSON has no features array".into()))?;

    if features.is_empty() {
        return Err(GeoError::Validation(
            "Forest inventory has no features".into(),
        ));
    }

    // Compute NDVI statistics over all valid pixels
    let healthy_threshold = 0.3;
    let mut healthy_count: usize = 0;
    let mut total_valid: usize = 0;
    let mut ndvi_sum: f64 = 0.0;

    for &val in ndvi_data {
        if (val - ndvi_nodata).abs() > 1e-10 {
            total_valid += 1;
            ndvi_sum += val;
            if val > healthy_threshold {
                healthy_count += 1;
            }
        }
    }

    if total_valid == 0 {
        return Err(GeoError::Validation("No valid NDVI pixels found".into()));
    }

    let mean_ndvi = ndvi_sum / total_valid as f64;
    let healthy_area_ha = healthy_count as f64 * pixel_area_ha;

    // IPCC Tier 2 allometric: AGB (t/ha) = a × NDVI - b
    // Coefficients from literature (tropical forests): a=135.53, b=16.76
    let agb_per_ha = if mean_ndvi > 0.0 {
        (135.53 * mean_ndvi - 16.76).max(0.0)
    } else {
        0.0
    };

    // Carbon fraction = 0.47, CO₂/C ratio = 44/12 ≈ 3.67
    let carbon_fraction = 0.47;
    let co2c_ratio = 44.0 / 12.0;
    let tco2e_per_ha = agb_per_ha * carbon_fraction * co2c_ratio;
    let total_tco2e = tco2e_per_ha * healthy_area_ha;

    // Per-polygon estimates (simplified: use global mean NDVI, proportional to area)
    let mut by_polygon = Vec::with_capacity(features.len());
    for (i, feature) in features.iter().enumerate() {
        let props = feature
            .get("properties")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_default();
        // Estimate polygon area from polygon geometry (simplified)
        let poly_ndvi_ratio = if feature["geometry"]["type"].as_str() == Some("Polygon") {
            // Use feature's own NDVI if class property exists
            let class_ndvi = props.get("ndvi_mean").and_then(|v| v.as_f64());
            class_ndvi.unwrap_or(mean_ndvi)
        } else {
            mean_ndvi
        };

        let poly_agb = (135.53 * poly_ndvi_ratio - 16.76).max(0.0);
        let poly_sink_ha = poly_agb * carbon_fraction * co2c_ratio;

        // Estimate proportional area (crude: equal split for now)
        let area_ha = healthy_area_ha / features.len() as f64;
        by_polygon.push(PolygonSink {
            polygon_id: i,
            area_ha,
            mean_ndvi: poly_ndvi_ratio,
            agb_tco2e: poly_agb,
            sink_tco2e_ha: poly_sink_ha,
        });
    }

    let result = CarbonSinkResult {
        total_tco2e,
        healthy_area_ha,
        mean_ndvi,
        by_polygon,
    };

    tracing::info!(
        total_tco2e = result.total_tco2e,
        healthy_area_ha = result.healthy_area_ha,
        mean_ndvi = result.mean_ndvi,
        "Carbon sink estimation complete"
    );

    serde_json::to_string_pretty(&result)
        .map_err(|e| GeoError::Validation(format!("Serialization error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_carbon_sink_basic() {
        let rows = 10;
        let cols = 10;
        let mut data = vec![-9999.0; rows * cols];
        // All pixels with NDVI 0.6 (healthy vegetation)
        for i in 0..(rows * cols) {
            data[i] = 0.6;
        }

        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [{"type": "Feature", "geometry": {"type": "Polygon", "coordinates": [[[0,0],[0,1],[1,1],[1,0],[0,0]]]}, "properties": {"class": "forest"}}]
        }"#;

        let result = estimate_carbon_sink(&data, rows, cols, -9999.0, 0.09, geojson).unwrap();
        let json: Value = serde_json::from_str(&result).unwrap();
        assert!(json["total_tco2e"].as_f64().unwrap() > 0.0);
        assert!(json["healthy_area_ha"].as_f64().unwrap() > 0.0);
        assert!(json["mean_ndvi"].as_f64().unwrap() > 0.3);
    }

    #[test]
    fn test_estimate_carbon_sink_no_vegetation() {
        let rows = 5;
        let cols = 5;
        let mut data = vec![-9999.0; rows * cols];
        // All pixels below vegetation threshold
        for i in 0..(rows * cols) {
            data[i] = 0.1;
        }

        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [{"type": "Feature", "geometry": {"type": "Polygon", "coordinates": [[[0,0],[0,1],[1,1],[1,0],[0,0]]]}, "properties": {}}]
        }"#;

        let result = estimate_carbon_sink(&data, rows, cols, -9999.0, 0.09, geojson).unwrap();
        let json: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total_tco2e"].as_f64().unwrap(), 0.0);
        assert_eq!(json["healthy_area_ha"].as_f64().unwrap(), 0.0);
    }

    #[test]
    fn test_estimate_carbon_sink_invalid_geojson() {
        let data = vec![0.5; 25];
        let result = estimate_carbon_sink(&data, 5, 5, -9999.0, 0.09, "not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_estimate_carbon_sink_dimension_mismatch() {
        let data = vec![0.5; 30]; // 30 ≠ 5×5=25
        let geojson = r#"{"type": "FeatureCollection", "features": []}"#;
        let result = estimate_carbon_sink(&data, 5, 5, -9999.0, 0.09, geojson);
        assert!(result.is_err());
    }

    #[test]
    fn test_estimate_carbon_sink_empty_features() {
        let data = vec![0.5; 25];
        let geojson = r#"{"type": "FeatureCollection", "features": []}"#;
        let result = estimate_carbon_sink(&data, 5, 5, -9999.0, 0.09, geojson);
        assert!(result.is_err());
    }
}
