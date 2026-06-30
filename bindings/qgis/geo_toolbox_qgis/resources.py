"""Geo Toolbox QGIS Plugin — icon resources.

This module provides a minimal SVG icon for the QGIS plugin toolbar button.
SVG icon: stylised globe with grid lines and a leaf (carbon/geo theme).
"""

# ── SVG icon (inline) ──────────────────────────────────────────
# 24×24 px, suitable for QGIS toolbar at standard DPI.

ICON_SVG = """\
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#2d8c5c"/>
      <stop offset="100%" stop-color="#1a6b42"/>
    </linearGradient>
  </defs>
  <!-- globe circle -->
  <circle cx="12" cy="12" r="10" fill="none" stroke="url(#g)" stroke-width="2"/>
  <!-- grid: equator -->
  <line x1="2" y1="12" x2="22" y2="12" stroke="url(#g)" stroke-width="1" opacity="0.6"/>
  <!-- grid: prime meridian -->
  <line x1="12" y1="2" x2="12" y2="22" stroke="url(#g)" stroke-width="1" opacity="0.6"/>
  <!-- grid: diagonal 1 -->
  <ellipse cx="12" cy="12" rx="10" ry="4" fill="none" stroke="url(#g)" stroke-width="0.7" opacity="0.5"/>
  <!-- grid: diagonal 2 -->
  <ellipse cx="12" cy="12" rx="4" ry="10" fill="none" stroke="url(#g)" stroke-width="0.7" opacity="0.5"/>
  <!-- leaf accent -->
  <path d="M14 5 C16 3, 22 6, 14 14 Z" fill="#3cb371" opacity="0.7"/>
  <path d="M14 5 C12 7, 8 10, 14 14" fill="none" stroke="#1a6b42" stroke-width="0.8"/>
</svg>"""


def icon_path():
    """Return the filesystem path to icon.svg, creating it if needed."""
    import os
    here = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(here, "icon.svg")
    if not os.path.exists(path):
        with open(path, "w", encoding="utf-8") as f:
            f.write(ICON_SVG)
    return path


if __name__ == "__main__":
    print(icon_path())
