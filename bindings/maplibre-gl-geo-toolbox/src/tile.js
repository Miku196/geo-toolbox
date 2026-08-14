/**
 * tile.js — 地图矢量切片（MVT）编码与图层注册。
 *
 * 依赖：ctx.map（MapLibre map）、ctx.engine.tile（WASM TileEngine.encodeMvt）。
 * 职责：把 GeoJSON 编码为 MVT 切片 URL、添加到地图为 vector source，并注册默认
 *       fill 图层。
 *
 * ctx 约定见 crs.js 顶部注释。
 */


/**
 * Add a vector source from GeoJSON and display as MVT tiles.
 * @param {object} ctx
 * @param {string} sourceId     MapLibre source id
 * @param {object|string} geojson
 * @param {number} [maxZoom=14]
 */
export async function addMvtSource(ctx, sourceId, geojson, maxZoom = 14) {
  ctx.assertReady();
  if (ctx.map.getSource(sourceId)) {
    ctx.map.removeSource(sourceId);
  }
  const data = typeof geojson === 'string' ? JSON.parse(geojson) : geojson;
  const tiles = await geojsonToMvt(ctx, data, maxZoom);

  ctx.map.addSource(sourceId, {
    type: 'vector',
    tiles,
    minzoom: 0,
    maxzoom: maxZoom,
  });
}


/**
 * Add default fill layer for a vector source.
 * @param {object} ctx
 * @param {string} sourceId   MapLibre source id
 * @param {string} [layerId]  layer id (default: sourceId + '-fill')
 */
export function addFillLayer(ctx, sourceId, layerId) {
  const id = layerId || `${sourceId}-fill`;
  if (ctx.map.getLayer(id)) return;
  ctx.map.addLayer({
    id,
    type: 'fill',
    source: sourceId,
    'source-layer': 'default',
    paint: {
      'fill-color': '#088',
      'fill-opacity': 0.3,
      'fill-outline-color': '#044',
    },
  });
}


/**
 * Encode a GeoJSON FeatureCollection to MVT tiles.
 * Tile-level encoding via WASM TileEngine.encodeMvt.
 */
async function geojsonToMvt(ctx, geojson, maxZoom) {
  ctx.assertReady();
  const fc = typeof geojson === 'string' ? JSON.parse(geojson) : geojson;
  const fcStr = JSON.stringify(fc);
  const tiles = [];
  for (let z = 0; z <= maxZoom; z++) {
    try {
      const mvtBytes = ctx.engine.tile.encodeMvt('default', fcStr, 0, 0, z, 4096);
      const blob = new Blob([mvtBytes], { type: 'application/vnd.mapbox-vector-tile' });
      tiles.push(URL.createObjectURL(blob));
    } catch (e) {
      console.warn(`MVT encoding failed at z=${z}`, e);
    }
  }
  if (tiles.length === 0) {
    return [`data:application/json,${encodeURIComponent(fcStr)}`];
  }
  return tiles;
}
