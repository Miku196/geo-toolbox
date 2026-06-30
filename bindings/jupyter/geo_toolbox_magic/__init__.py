"""
geo_toolbox_magic — Jupyter/IPython magic extension for geo-toolbox.

Usage:
    %load_ext geo_toolbox_magic

Magics provided:
    %%geo         — cell magic: first line tool_name, rest JSON params
    %geo          — line magic: %geo <tool_name> <json_params>
    %geo_list     — list all available tools
    %geo_schema   — %geo_schema <tool_name> shows JSON schema

Example:
    %%geo geohash_encode
    {"lat": 39.9, "lon": 116.4, "precision": 8}
"""

from .magic import GeoMagics


def load_ipython_extension(ipython):
    """Register geo-toolbox magics with IPython."""
    magics = GeoMagics(ipython)
    ipython.register_magics(magics)


def unload_ipython_extension(ipython):
    """Unregister geo-toolbox magics (placeholder — IPython handles cleanup)."""
    pass
