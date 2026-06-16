"""
geo-toolbox — Python GIS toolkit.

Pure-Rust backend via PyO3. Provides:
- Tile math: lat/lon ↔ tile(x,y,z), tile URLs
- MVT encoder: GeoJSON → Mapbox Vector Tile (protobuf)
"""

from geo_toolbox._geo_toolbox import (
    latlon_to_tile,
    tile_to_latlon,
    tile_url,
    MvtEncoder,
    __version__,
)

__all__ = [
    "latlon_to_tile",
    "tile_to_latlon",
    "tile_url",
    "MvtEncoder",
    "__version__",
]
