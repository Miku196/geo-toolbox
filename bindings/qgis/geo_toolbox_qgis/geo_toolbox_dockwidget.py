"""Geo Toolbox QGIS Dock Widget — interactive tool execution panel.

Duck-typed: uses PyQt5/PyQt6 via try/except so the module is import-safe.
"""

import json as _json
import traceback as _traceback


# ── Qt imports (duck-typed — safe to import outside QGIS) ─────

try:
    from qgis.PyQt.QtWidgets import (
        QDockWidget,
        QWidget,
        QVBoxLayout,
        QHBoxLayout,
        QLabel,
        QComboBox,
        QPlainTextEdit,
        QPushButton,
        QMessageBox,
        QSplitter,
        QFrame,
    )
    from qgis.PyQt.QtCore import Qt
    _HAS_QT = True
except ImportError:
    _HAS_QT = False


# ── Tool metadata — mirrors core-layer tool registry ───────────

# Static copy so the dropdown populates even when geo-toolbox is
# not installed. Updated dynamically if the package is available.
_BUILTIN_TOOLS = [
    # ── CRS ──
    {"name": "crs_transform", "desc": "Transform coordinates between CRSes"},
    {"name": "crs_list", "desc": "List supported CRS codes"},
    {"name": "auto_detect_utm_zone", "desc": "Auto-detect UTM zone"},
    {"name": "auto_detect_gk_zone", "desc": "Auto-detect Gauss-Krüger 3° zone"},
    # ── Carbon ──
    {"name": "calculate", "desc": "Compute carbon stock / sink / emission"},
    # ── Tile ──
    {"name": "latlon_to_tile", "desc": "Lat/lon → tile XYZ"},
    {"name": "tile_to_latlon", "desc": "Tile XYZ → lat/lon bounds"},
    {"name": "tile_url", "desc": "Generate XYZ tile URL"},
    {"name": "mvt_encode", "desc": "Encode GeoJSON → MVT tile"},
    # ── Spatial ──
    {"name": "compute_area_sqm", "desc": "Compute area (m²)"},
    {"name": "compute_bbox", "desc": "Compute bounding box"},
    {"name": "compute_centroid", "desc": "Compute centroid"},
    {"name": "simplify_geometry", "desc": "Simplify geometry (Ramer-Douglas-Peucker)"},
    {"name": "convex_hull", "desc": "Compute convex hull"},
    {"name": "vector_buffer", "desc": "Buffer a geometry"},
    {"name": "vector_intersect", "desc": "Intersect two geometries"},
    # ── IO ──
    {"name": "parse_csv_to_json", "desc": "Parse CSV → JSON"},
    {"name": "generate_geojson", "desc": "Generate GeoJSON"},
    {"name": "generate_excel", "desc": "Generate XLSX"},
    {"name": "generate_carbon_report_md", "desc": "Generate carbon report (Markdown)"},
    # ── Ingest ──
    {"name": "parse_nmea", "desc": "Parse NMEA sentence"},
    {"name": "parse_nmea_batch", "desc": "Batch-parse NMEA sentences"},
    {"name": "validate_coord", "desc": "Validate coordinate"},
    {"name": "validate_gps_fix", "desc": "Validate GPS fix quality"},
    {"name": "validate_sensor_reading", "desc": "Validate sensor reading"},
    # ── Geohash ──
    {"name": "geohash_encode", "desc": "Encode lat/lon → geohash"},
    {"name": "geohash_decode", "desc": "Decode geohash → lat/lon"},
    {"name": "geohash_neighbors", "desc": "Get geohash neighbors"},
    # ── Temporal ──
    {"name": "date_diff", "desc": "Date difference in days"},
    {"name": "season_of", "desc": "Season of a date"},
    # ── Stats ──
    {"name": "basic_stats", "desc": "Compute basic statistics (mean, std, etc.)"},
    {"name": "zonal_stats", "desc": "Zonal statistics"},
    # ── Report ──
    {"name": "generate_report", "desc": "Generate a structured report"},
]


class GeoToolboxDockWidget(QDockWidget if _HAS_QT else object):
    """Dockable panel for executing geo-toolbox operations."""

    def __init__(self, parent=None):
        if _HAS_QT:
            super().__init__("Geo Toolbox", parent)
            self.setObjectName("GeoToolboxDock")
            self.setAllowedAreas(Qt.LeftDockWidgetArea | Qt.RightDockWidgetArea)
            self.setMinimumWidth(360)
            self._build_ui()
            self._refresh_tools()
        else:
            # Test-safe: store kwargs so tests can inspect them
            self._parent = parent

    # ── UI construction ─────────────────────────────────────

    def _build_ui(self):
        """Lay out: tool combo | params editor | [Execute] | result viewer."""
        central = QWidget()
        self.setWidget(central)

        layout = QVBoxLayout(central)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(6)

        # ── Tool selector ──
        tool_row = QHBoxLayout()
        tool_row.addWidget(QLabel("Tool:"))
        self.combo_tool = QComboBox()
        self.combo_tool.setEditable(False)
        self.combo_tool.setMinimumWidth(200)
        self.combo_tool.setToolTip("Select a geo-toolbox operation")
        tool_row.addWidget(self.combo_tool, 1)
        layout.addLayout(tool_row)

        # ── Params editor ──
        layout.addWidget(QLabel("Parameters (JSON):"))
        self.editor_params = QPlainTextEdit()
        self.editor_params.setPlaceholderText('{"geojson_geom": "...", ...}')
        self.editor_params.setMaximumHeight(120)
        self.editor_params.setTabChangesFocus(False)
        layout.addWidget(self.editor_params)

        # ── Execute + status ──
        btn_row = QHBoxLayout()
        self.btn_execute = QPushButton("▶ Execute")
        self.btn_execute.setMinimumHeight(32)
        self.btn_execute.clicked.connect(self._on_execute)
        btn_row.addWidget(self.btn_execute, 1)

        self.label_status = QLabel("")
        self.label_status.setStyleSheet("color: #888;")
        btn_row.addWidget(self.label_status)
        layout.addLayout(btn_row)

        # ── Separator ──
        sep = QFrame()
        sep.setFrameShape(QFrame.HLine)
        sep.setFrameShadow(QFrame.Sunken)
        layout.addWidget(sep)

        # ── Result viewer ──
        layout.addWidget(QLabel("Result:"))
        self.result_viewer = QPlainTextEdit()
        self.result_viewer.setReadOnly(True)
        self.result_viewer.setPlaceholderText("Result will appear here…")
        layout.addWidget(self.result_viewer, 1)

    # ── Tool list refresh ───────────────────────────────────

    def _refresh_tools(self):
        """Populate tool dropdown from static list, then enrich from
        live geo-toolbox if installed."""
        if not _HAS_QT:
            return

        self.combo_tool.clear()
        tool_names = set()

        for t in _BUILTIN_TOOLS:
            self.combo_tool.addItem(f"{t['name']}  — {t['desc']}", t["name"])
            tool_names.add(t["name"])

        # Try to append any tools from the live registry we don't have
        try:
            from geo_toolbox._geo_toolbox import Geo
            geo = Geo()
            live = _json.loads(geo.list_tools())
            for t in live:
                if t.get("name") and t["name"] not in tool_names:
                    desc = t.get("description", "")
                    self.combo_tool.addItem(f"{t['name']}  — {desc}", t["name"])
        except Exception:
            pass  # geo-toolbox not installed — use static list

    # ── Actions ─────────────────────────────────────────────

    def _on_execute(self):
        """Run the selected tool with the provided JSON params."""
        if not _HAS_QT:
            return

        tool_idx = self.combo_tool.currentIndex()
        if tool_idx < 0:
            self._show_error("No tool selected.")
            return

        tool_name = self.combo_tool.itemData(tool_idx)
        params_text = self.editor_params.toPlainText().strip()

        # Validate JSON
        try:
            params = _json.loads(params_text) if params_text else {}
        except _json.JSONDecodeError as e:
            self._show_error(f"Invalid JSON: {e}")
            return

        # Execute
        self.label_status.setText(f"Running {tool_name}…")
        self.label_status.setStyleSheet("color: #0366d6;")
        self.btn_execute.setEnabled(False)
        try:
            # Re-serialize to match geo.call() signature (takes JSON string)
            result_json = self._geo_call(tool_name, _json.dumps(params))
            self._show_result(result_json)
            self.label_status.setText("Done ✓")
            self.label_status.setStyleSheet("color: #28a745;")
        except Exception as e:
            self._show_error(f"{type(e).__name__}: {e}")
            self.label_status.setText("Failed ✗")
            self.label_status.setStyleSheet("color: #dc3545;")
        finally:
            self.btn_execute.setEnabled(True)

    def _geo_call(self, tool_name: str, params_json: str) -> str:
        """Call geo-toolbox. Tries registry first, then direct function."""
        # Primary path: Geo().call() registry
        try:
            from geo_toolbox._geo_toolbox import Geo
            return Geo().call(tool_name, params_json)
        except ImportError:
            pass
        except Exception:
            pass

        # Fallback: try the free functions exposed in geo_toolbox
        try:
            import geo_toolbox

            # Map tool_name → function on the module
            fn = getattr(geo_toolbox, tool_name, None)
            if fn is None:
                raise RuntimeError(
                    f"Tool '{tool_name}' not found. "
                    "Install geo-toolbox: pip install geo-toolbox"
                )

            params = _json.loads(params_json) if params_json else {}
            result = fn(**params) if isinstance(params, dict) else fn(params)
            return _json.dumps({"ok": result}, default=str)
        except ImportError:
            raise RuntimeError(
                "geo-toolbox is not installed. "
                "Run: pip install geo-toolbox"
            )

    # ── UI helpers ──────────────────────────────────────────

    def _show_result(self, text: str):
        """Display formatted result."""
        if not _HAS_QT:
            return
        try:
            parsed = _json.loads(text)
            pretty = _json.dumps(parsed, ensure_ascii=False, indent=2)
        except (_json.JSONDecodeError, TypeError):
            pretty = text
        self.result_viewer.setPlainText(pretty)

    def _show_error(self, message: str):
        """Display error in result area."""
        if not _HAS_QT:
            return
        self.result_viewer.setPlainText(f"ERROR: {message}\n\n{_traceback.format_exc()}")
