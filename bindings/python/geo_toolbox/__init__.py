"""
geo-toolbox — Python GIS toolkit.

Pure-Rust backend via PyO3. Provides:
- CRS transformation: WGS84 ↔ GCJ-02 ↔ BD-09 ↔ Web Mercator
- Carbon engine: IPCC emission calculation from GeoJSON + factors
- Tile math: lat/lon ↔ tile(x,y,z), tile URLs, MVT encoding
- Spatial ops: area, bbox, centroid, simplify, convex hull
- IO: CSV parse, GeoJSON/Excel export, carbon report markdown
- NMEA ingest: parse GGA/RMC/GLL/VTG sentences, validate coords
- Geohash: encode, decode, neighbors
- Stats: zonal_stats, Moran's I, Gi* hotspot, IDW interpolation, Jenks
- Temporal: Mann-Kendall trend + linear regression
- Report: carbon report Markdown, Tera template rendering
"""

from geo_toolbox._geo_toolbox import (
    CrsEngine,
    CarbonEngine,
    latlon_to_tile,
    tile_to_latlon,
    tile_url,
    MvtEncoder,
    compute_area_sqm,
    compute_bbox,
    compute_centroid,
    simplify_geometry,
    convex_hull,
    parse_csv_to_json,
    generate_geojson,
    generate_excel,
    generate_carbon_report_md,
    parse_nmea,
    parse_nmea_batch,
    validate_coord,
    validate_gps_fix,
    validate_sensor_reading,
    geohash_encode,
    geohash_decode,
    geohash_neighbors,
    zonal_stats,
    morans_i,
    gistar,
    idw_grid,
    jenks_classify,
    temporal_trend,
    report_carbon,
    report_render,
    __version__,
)

import json as _json


class Geo:
    """Unified geo-toolbox call interface.

    >>> import geo_toolbox
    >>> geo = geo_toolbox.Geo()
    >>> result = geo.call("geohash_encode", {"lat": 39.9, "lon": 116.4, "precision": 8})
    >>> "ok" in result
    True

    Or use the module-level convenience function:
    >>> geo_toolbox.call("geohash_encode", {"lat": 39.9, "lon": 116.4, "precision": 8})
    """

    def __init__(self):
        from geo_toolbox._geo_toolbox import Geo as _Geo
        self._geo = _Geo()

    def call(self, tool_name: str, params: dict | str) -> dict:
        """Call a registered tool by name.

        Args:
            tool_name: Registered tool name (e.g. "geohash_encode")
            params: Dict or JSON string of parameters

        Returns:
            Dict with "ok" key on success or "error" key on failure
        """
        if isinstance(params, dict):
            params = _json.dumps(params)
        raw = self._geo.call(tool_name, params)
        return _json.loads(raw)

    def list_tools(self) -> list:
        """List all registered tools with name, description, input_schema."""
        return _json.loads(self._geo.list_tools())

    def tool_schema(self, tool_name: str) -> dict | None:
        """Get JSON Schema for a specific tool's parameters."""
        return _json.loads(self._geo.tool_schema(tool_name))


# --- Module-level convenience ---

_geo_instance: "Geo | None" = None


def _get_geo() -> Geo:
    global _geo_instance
    if _geo_instance is None:
        _geo_instance = Geo()
    return _geo_instance


def call(tool_name: str, params: dict | str) -> dict:
    """Convenience function: call a tool without instantiating Geo.

    >>> import geo_toolbox
    >>> geo_toolbox.call("geohash_encode", {"lat": 39.9, "lon": 116.4, "precision": 8})
    """
    return _get_geo().call(tool_name, params)


def list_tools() -> list:
    """List all registered tools."""
    return _get_geo().list_tools()


def tool_schema(tool_name: str) -> dict | None:
    """Get JSON Schema for a tool."""
    return _get_geo().tool_schema(tool_name)


__all__ = [
    "Geo",
    "call",
    "list_tools",
    "tool_schema",
    "CrsEngine",
    "CarbonEngine",
    "latlon_to_tile",
    "tile_to_latlon",
    "tile_url",
    "MvtEncoder",
    "compute_area_sqm",
    "compute_bbox",
    "compute_centroid",
    "simplify_geometry",
    "convex_hull",
    "parse_csv_to_json",
    "generate_geojson",
    "generate_excel",
    "generate_carbon_report_md",
    "parse_nmea",
    "parse_nmea_batch",
    "validate_coord",
    "validate_gps_fix",
    "validate_sensor_reading",
    "geohash_encode",
    "geohash_decode",
    "geohash_neighbors",
    "zonal_stats",
    "morans_i",
    "gistar",
    "idw_grid",
    "jenks_classify",
    "temporal_trend",
    "report_carbon",
    "report_render",
    "__version__",
]
