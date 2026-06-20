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
"""

from geo_toolbox._geo_toolbox import (
    # CRS
    CrsEngine,
    # Carbon
    CarbonEngine,
    # Tile
    latlon_to_tile,
    tile_to_latlon,
    tile_url,
    MvtEncoder,
    # Spatial
    compute_area_sqm,
    compute_bbox,
    compute_centroid,
    simplify_geometry,
    convex_hull,
    # IO
    parse_csv_to_json,
    generate_geojson,
    generate_excel,
    generate_carbon_report_md,
    # Ingest
    parse_nmea,
    parse_nmea_batch,
    validate_coord,
    validate_gps_fix,
    validate_sensor_reading,
    # Geohash
    geohash_encode,
    geohash_decode,
    geohash_neighbors,
    # Version
    __version__,
)

__all__ = [
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
    "__version__",
]
