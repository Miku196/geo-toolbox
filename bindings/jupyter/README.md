# geo-toolbox-magic

Jupyter/IPython magic extension for [geo-toolbox](https://github.com/geo-toolbox/geo-toolbox) —
browser-grade GIS operations directly in notebooks.

## Installation

```bash
# Install geo-toolbox Python bindings first
cd bindings/python && pip install .

# Install the magic extension
cd ../jupyter && pip install .
```

Or install both in one go:
```bash
pip install bindings/python bindings/jupyter
```

## Usage

Load the extension in any Jupyter notebook or IPython session:

```python
%load_ext geo_toolbox_magic
```

### `%%geo` — Cell magic

First line is the tool name, remaining lines are JSON parameters.

```python
%%geo geohash_encode
{"lat": 39.9, "lon": 116.4, "precision": 8}
```

```python
%%geo compute_area_sqm
{"geojson_geom": "{\"type\":\"Polygon\",\"coordinates\":[[[116.0,39.8],[116.5,39.8],[116.5,40.1],[116.0,40.1],[116.0,39.8]]]}"}
```

### `%geo` — Line magic

One-line tool calls.

```python
%geo geohash_encode {"lat": 39.9, "lon": 116.4, "precision": 8}
```

### `%geo_list` — List all tools

```python
%geo_list
```

### `%geo_schema` — Show tool schema

```python
%geo_schema geohash_encode
```

## Available tools

The extension exposes all core-layer tools registered in geo-toolbox:
CRS transforms, tile math, spatial ops, carbon calculations, geohash,
I/O utilities, NMEA parsing, and more.

Use `%geo_list` to see the full list in your notebook.

## License

MIT — see the [geo-toolbox repository](https://github.com/geo-toolbox/geo-toolbox).
