/// MapLibre GL map with AOI drawing and area calculation.
import maplibregl from 'maplibre-gl';

// OSM tile style (no API key needed, works offline once cached)
const MAP_STYLE = 'https://demotiles.maplibre.org/style.json';

export interface MapInstance {
  map: maplibregl.Map;
  drawPolygon: () => void;
  clearPolygon: () => void;
  getDrawnGeoJSON: () => object | null;
  getDrawnAreaHa: () => number;
}

export function initMap(container: string): MapInstance {
  const map = new maplibregl.Map({
    container,
    style: MAP_STYLE,
    center: [116.4, 39.9],
    zoom: 4,
    attributionControl: false,
  });

  map.addControl(new maplibregl.NavigationControl(), 'top-left');

  // Geolocate
  map.addControl(
    new maplibregl.GeolocateControl({
      positionOptions: { enableHighAccuracy: true },
      trackUserLocation: true,
      showUserHeading: true,
    }),
    'top-left'
  );

  let drawActive = false;
  let drawnPolygon: maplibregl.MapTouchEvent | null = null;
  const points: [number, number][] = [];
  const markers: maplibregl.Marker[] = [];
  let polygonSourceId = 'drawn-aoi';
  let polygonLayerId = 'drawn-aoi-layer';

  // Draw polygon handler
  function startDraw() {
    drawActive = true;
    points.length = 0;
    markers.forEach(m => m.remove());
    markers.length = 0;
    clearPolygon();
    const hint = document.getElementById('draw-hint');
    if (hint) hint.style.display = 'block';
    map.getCanvas().style.cursor = 'crosshair';
  }

  function clearPolygon() {
    if (map.getLayer(polygonLayerId)) map.removeLayer(polygonLayerId);
    if (map.getSource(polygonSourceId)) map.removeSource(polygonSourceId);
  }

  function updatePolygon() {
    if (points.length < 3) return;
    clearPolygon();
    map.addSource(polygonSourceId, {
      type: 'geojson',
      data: {
        type: 'Feature',
        geometry: {
          type: 'Polygon',
          coordinates: [[...points, points[0]]],
        },
        properties: {},
      },
    });
    map.addLayer({
      id: polygonLayerId,
      type: 'fill',
      source: polygonSourceId,
      paint: {
        'fill-color': '#2e7d32',
        'fill-opacity': 0.3,
        'fill-outline-color': '#1b5e20',
      },
    });
    const hint = document.getElementById('draw-hint');
    if (hint) hint.style.display = 'none';
  }

  map.on('click', (e) => {
    if (!drawActive) return;
    points.push([e.lngLat.lng, e.lngLat.lat]);
    const marker = new maplibregl.Marker({ color: '#2e7d32', draggable: false })
      .setLngLat(e.lngLat)
      .addTo(map);
    markers.push(marker);
    if (points.length >= 3) updatePolygon();
  });

  // Double-click to finish drawing
  map.on('dblclick', () => {
    if (!drawActive) return;
    drawActive = false;
    map.getCanvas().style.cursor = '';
    const hint = document.getElementById('draw-hint');
    if (hint) hint.style.display = 'none';
  });

  function getDrawnGeoJSON(): object | null {
    if (points.length < 3) return null;
    return {
      type: 'Polygon',
      coordinates: [[...points, points[0]]],
    };
  }

  function getDrawnAreaHa(): number {
    if (points.length < 3) return 0;
    // Shoelace formula for area in WGS84 degrees → approximate km² → ha
    let area = 0;
    for (let i = 0; i < points.length; i++) {
      const j = (i + 1) % points.length;
      area += points[i][0] * points[j][1];
      area -= points[j][0] * points[i][1];
    }
    area = Math.abs(area) / 2;
    // Convert degree² to km² (approximate at mid-latitude)
    const midLat = points.reduce((s, p) => s + p[1], 0) / points.length;
    const latRad = (midLat * Math.PI) / 180;
    const km2PerDeg2 = 111.32 * 111.32 * Math.cos(latRad);
    return area * km2PerDeg2 * 100; // km² → ha
  }

  return {
    map,
    drawPolygon: startDraw,
    clearPolygon: () => {
      points.length = 0;
      markers.forEach(m => m.remove());
      markers.length = 0;
      clearPolygon();
    },
    getDrawnGeoJSON,
    getDrawnAreaHa,
  };
}
