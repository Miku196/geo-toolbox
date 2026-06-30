# ── Geo Toolbox QGIS Plugin ──
# Entry point for QGIS Plugin Manager.
# QGIS calls classFactory(iface) to instantiate the plugin.

def classFactory(iface):
    """QGIS plugin entry point — returns a QGIS plugin instance."""
    from .geo_toolbox_plugin import GeoToolboxPlugin
    return GeoToolboxPlugin(iface)
