//! MVT (Mapbox Vector Tile) feature source abstraction.
//!
//! Provides a trait-based interface for supplying GeoJSON features per tile,
//! enabling WMTS layers to serve vector tiles via `geo_tile::MvtEncoder`.

use geo_core::errors::GeoResult;
use geo_tile::{MvtEncoder, MvtLayer};
use serde_json::Value;

/// A provider of GeoJSON features for a given tile coordinate.
pub trait MvtFeatureProvider: Send + Sync {
    /// Return GeoJSON features that intersect the tile at (z, x, y).
    fn features_for_tile(&self, z: u8, x: u32, y: u32) -> Vec<Value>;

    /// Optional: pre-load data for a zoom range. Default is no-op.
    fn prepare_zoom_range(&self, _min_z: u8, _max_z: u8) {}
}

/// An MVT feature provider backed by an in-memory GeoJSON FeatureCollection.
///
/// # Example
/// ```ignore
/// use geo_ogc::mvt_source::{JsonFeatureProvider, MvtFeatureProvider};
/// let provider = JsonFeatureProvider::new(geojson_str);
/// let features = provider.features_for_tile(10, 844, 385);
/// ```
pub struct JsonFeatureProvider {
    features: Vec<Value>,
}

impl JsonFeatureProvider {
    /// Create a provider from a GeoJSON FeatureCollection string.
    pub fn new(geojson: &str) -> GeoResult<Self> {
        let fc: Value = serde_json::from_str(geojson)?;
        let features = fc
            .get("features")
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(Self { features })
    }

    /// Create a provider from an existing list of GeoJSON feature Values.
    pub fn from_features(features: Vec<Value>) -> Self {
        Self { features }
    }

    /// Number of features in the provider.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Whether the provider has no features.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Create a feature collection JSON from all features.
    pub fn to_feature_collection(&self) -> Value {
        serde_json::json!({
            "type": "FeatureCollection",
            "features": self.features,
        })
    }
}

impl MvtFeatureProvider for JsonFeatureProvider {
    fn features_for_tile(&self, _z: u8, _x: u32, _y: u32) -> Vec<Value> {
        self.features.clone()
    }
}

/// Render an MVT tile from a feature provider using `geo_tile::MvtEncoder`.
///
/// # Arguments
/// * `provider` - The feature source.
/// * `layer_name` - Name of the MVT layer within the tile.
/// * `z` - Zoom level.
/// * `x` - Tile column.
/// * `y` - Tile row.
/// * `extent` - Tile extent in pixels (typically 4096).
pub fn render_mvt_tile(
    provider: &dyn MvtFeatureProvider,
    layer_name: &str,
    z: u8,
    x: u32,
    y: u32,
    extent: u32,
) -> GeoResult<Vec<u8>> {
    let features = provider.features_for_tile(z, x, y);
    let encoder = MvtEncoder::new(extent);
    encoder.encode_tile(layer_name, &features, x, y, z)
}

/// Build an MVT layer object (useful for PMTiles archiving).
pub fn build_mvt_layer(
    provider: &dyn MvtFeatureProvider,
    layer_name: &str,
    z: u8,
    x: u32,
    y: u32,
    extent: u32,
) -> GeoResult<MvtLayer> {
    let features = provider.features_for_tile(z, x, y);
    let encoder = MvtEncoder::new(extent);
    let mvt_features = features
        .iter()
        .map(|f| encoder.feature_from_geojson(f, x, y, z))
        .collect::<GeoResult<Vec<_>>>()?;

    Ok(MvtLayer {
        name: layer_name.to_string(),
        extent,
        features: mvt_features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_geojson() -> &'static str {
        r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": {"name": "point1"},
                    "geometry": {"type": "Point", "coordinates": [104.0, 30.0]}
                },
                {
                    "type": "Feature",
                    "properties": {"name": "line1"},
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[104.0, 30.0], [104.1, 30.1]]
                    }
                }
            ]
        }"#
    }

    #[test]
    fn test_json_feature_provider_create() {
        let provider = JsonFeatureProvider::new(sample_geojson()).unwrap();
        assert_eq!(provider.len(), 2);
        assert!(!provider.is_empty());
    }

    #[test]
    fn test_json_feature_provider_empty() {
        let provider = JsonFeatureProvider::from_features(vec![]);
        assert!(provider.is_empty());
    }

    #[test]
    fn test_features_for_tile_returns_all() {
        let provider = JsonFeatureProvider::new(sample_geojson()).unwrap();
        let features = provider.features_for_tile(10, 844, 385);
        assert_eq!(features.len(), 2);
    }

    #[test]
    fn test_render_mvt_tile_point() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"id": 1},
                "geometry": {"type": "Point", "coordinates": [104.0, 30.0]}
            }]
        }"#;
        let provider = JsonFeatureProvider::new(geojson).unwrap();
        let data = render_mvt_tile(&provider, "test", 10, 844, 385, 4096).unwrap();
        assert!(!data.is_empty());
        // MVT tiles should start with a protobuf tag
        assert!(data[0] != 0);
    }

    #[test]
    fn test_build_mvt_layer() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {"type": "Point", "coordinates": [104.0, 30.0]}
            }]
        }"#;
        let provider = JsonFeatureProvider::new(geojson).unwrap();
        let layer = build_mvt_layer(&provider, "test", 10, 844, 385, 4096).unwrap();
        assert_eq!(layer.name, "test");
        assert_eq!(layer.extent, 4096);
        assert_eq!(layer.features.len(), 1);
    }
}
