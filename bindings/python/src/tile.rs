//! Tile math and MVT encoding.

use pyo3::prelude::*;
use pyo3::types::PyList;
use serde_json::Value;

/// Convert WGS84 lat/lon to tile (x, y, z).
pub fn latlon_to_tile_impl(lat: f64, lon: f64, zoom: u8) -> (u32, u32, u8) {
    let (x, y, z) = geo_tile::latlon_to_tile(lon, lat, zoom);
    (x, y, z)
}

/// Convert tile (x, y, z) to WGS84 bounding box (west, south, east, north).
pub fn tile_to_latlon_impl(x: u32, y: u32, zoom: u8) -> (f64, f64, f64, f64) {
    let (west, south, east, north) = geo_tile::tile_bounds(x, y, zoom);
    (west, south, east, north)
}

/// Get a tile URL for a known tile source.
pub fn tile_url_impl(source: &str, x: u32, y: u32, zoom: u8) -> PyResult<String> {
    use geo_tile::TileSource;
    let src = match source.to_lowercase().as_str() {
        "osm" => TileSource::OpenStreetMap,
        "gaode" => TileSource::Gaode,
        "tianditu" => TileSource::TianDiTu,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown tile source: {source} (try: osm, gaode, tianditu)"
            )))
        }
    };
    Ok(geo_tile::tile_url(src, x, y, zoom))
}

fn dict_to_value(py: Python<'_>, item: &Bound<'_, PyAny>) -> PyResult<Value> {
    let json_module = py.import("json")?;
    let json_str: String = json_module.call_method1("dumps", (item,))?.extract()?;
    serde_json::from_str(&json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

struct PendingLayer {
    name: String,
    features: Vec<Value>,
    tile_x: u32,
    tile_y: u32,
    zoom: u8,
}

/// Mapbox Vector Tile encoder (non-pyclass, called from lib.rs wrapper).
pub struct MvtEncoder {
    inner: geo_tile::MvtEncoder,
    layers: Vec<PendingLayer>,
}

impl MvtEncoder {
    pub fn new(extent: u32) -> Self {
        Self {
            inner: geo_tile::MvtEncoder::new(extent),
            layers: Vec::new(),
        }
    }

    pub fn add_layer(
        &mut self,
        name: &str,
        features: &Bound<'_, PyAny>,
        tile_x: u32,
        tile_y: u32,
        zoom: u8,
    ) -> PyResult<()> {
        let py = features.py();
        let list: &Bound<'_, PyList> = features.downcast::<PyList>()?;
        let mut feature_values = Vec::with_capacity(list.len());
        for item in list.iter() {
            feature_values.push(dict_to_value(py, &item)?);
        }
        self.layers.push(PendingLayer {
            name: name.to_string(),
            features: feature_values,
            tile_x,
            tile_y,
            zoom,
        });
        Ok(())
    }

    pub fn encode(&self) -> PyResult<Vec<u8>> {
        use geo_tile::MvtLayer;
        let mut mvt_layers = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            let mut layer_features = Vec::with_capacity(layer.features.len());
            for f in &layer.features {
                let feature = self
                    .inner
                    .feature_from_geojson(f, layer.tile_x, layer.tile_y, layer.zoom)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
                layer_features.push(feature);
            }
            mvt_layers.push(MvtLayer {
                name: layer.name.clone(),
                extent: 4096,
                features: layer_features,
            });
        }
        self.inner
            .encode(&mvt_layers)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}
