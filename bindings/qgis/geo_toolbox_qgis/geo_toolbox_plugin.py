"""Geo Toolbox QGIS Plugin — main plugin class.

Duck-typed PyQGIS integration: no hard imports of qgis.* at module level,
so the file is import-safe outside a QGIS environment (e.g. CI, testing).
"""

import os

# ── Icons ──────────────────────────────────────────────────────
# Resources compiled by resources.py at project build time;
# fall back to inline SVG if resources_rc not importable.

PLUGIN_DIR = os.path.dirname(os.path.abspath(__file__))
ICON_PATH = os.path.join(PLUGIN_DIR, "icon.svg")


# ── Plugin class ───────────────────────────────────────────────

class GeoToolboxPlugin:
    """QGIS plugin that exposes geo-toolbox operations."""

    def __init__(self, iface):
        """Save reference to QGIS interface.  iface may be any object
        exposing addToolBarIcon / addPluginToMenu / removeToolBarIcon /
        removePluginMenu — the plugin uses duck-typing so it works
        inside QGIS and is testable outside it."""
        self.iface = iface
        self.actions = []
        self.dock_widget = None

    # ── QGIS lifecycle ──────────────────────────────────────

    def initGui(self):
        """Create toolbar button, menu entry, and dock widget."""
        from .geo_toolbox_dockwidget import GeoToolboxDockWidget

        # ── Action / button ──
        icon = self._load_icon()
        action = self._make_action(
            icon_path=None,  # use QIcon below
            text="Geo Toolbox",
            callback=self._toggle_dock,
            parent=self.iface.mainWindow(),
        )
        if icon is not None:
            action.setIcon(icon)

        self.iface.addToolBarIcon(action)
        self.iface.addPluginToMenu("&Geo Toolbox", action)
        self.actions.append(action)

        # ── Dock widget ──
        self.dock_widget = GeoToolboxDockWidget(parent=self.iface.mainWindow())
        self.iface.addDockWidget(
            2,  # Qt.RightDockWidgetArea
            self.dock_widget,
        )
        self.dock_widget.hide()  # hidden by default

    def unload(self):
        """Remove toolbar icon, menu entry, and dock widget."""
        for action in self.actions:
            self.iface.removeToolBarIcon(action)
            self.iface.removePluginMenu("&Geo Toolbox", action)
        self.actions.clear()

        if self.dock_widget is not None:
            self.iface.removeDockWidget(self.dock_widget)
            self.dock_widget.deleteLater()
            self.dock_widget = None

    # ── internal ────────────────────────────────────────────

    def _toggle_dock(self):
        """Show or hide the dock widget."""
        if self.dock_widget is not None:
            self.dock_widget.setVisible(not self.dock_widget.isVisible())

    def _make_action(self, icon_path, text, callback, parent):
        """Create a QAction with duck-typed fallback for testing."""
        try:
            from qgis.PyQt.QtWidgets import QAction
            from qgis.PyQt.QtGui import QIcon
            action = QAction(text, parent)
            if icon_path:
                action.setIcon(QIcon(icon_path))
            action.triggered.connect(callback)
            return action
        except ImportError:
            # Test-only fallback: return a plain object
            class _FakeAction:
                def setIcon(self, _): pass
                triggered = type("Signal", (), {"connect": lambda s, cb: None})()
            return _FakeAction()

    def _load_icon(self):
        """Try loading the plugin icon; return None if unavailable."""
        if os.path.exists(ICON_PATH):
            try:
                from qgis.PyQt.QtGui import QIcon
                return QIcon(ICON_PATH)
            except ImportError:
                pass
        return None
