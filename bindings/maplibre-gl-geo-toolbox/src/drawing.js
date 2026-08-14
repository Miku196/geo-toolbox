/**
 * drawing.js — 手绘多边形 AOI 绘制交互。
 *
 * 依赖：ctx.map（MapLibre map）、ctx.drawing（绘制状态 get/set）。
 * 职责：在地图上点选顶点绘制多边形，双击/右键/Enter 闭合（触发 map 'aoi-drawn'
 *       事件，payload { aoi }），Esc 取消；并注册/清理预览图层与事件监听。
 *
 * ctx 约定见 crs.js 顶部注释，额外用到：
 *   ctx.drawing  可读写字段，持有 { active, vertices, cancel, finish, _onKey, ... }
 */


/**
 * Enable freehand polygon drawing on the map.
 * Emits `aoi-drawn` on the map with `{ aoi: GeoJSON Feature }`.
 * @param {object} ctx
 */
export function enableDrawing(ctx) {
  if (ctx.drawing) return;

  const map = ctx.map;
  const canvas = map.getCanvas();
  const state = { active: false, vertices: [], markerLayer: null };
  ctx.drawing = state;

  // ----- Preview source & layer -----
  map.addSource('__geo_draw_preview', {
    type: 'geojson',
    data: { type: 'FeatureCollection', features: [] },
  });
  map.addLayer({
    id: '__geo_draw_preview_line',
    type: 'line',
    source: '__geo_draw_preview',
    paint: { 'line-color': '#ff6600', 'line-width': 2, 'line-dasharray': [4, 2] },
  });
  map.addLayer({
    id: '__geo_draw_preview_vertices',
    type: 'circle',
    source: '__geo_draw_preview',
    paint: { 'circle-radius': 5, 'circle-color': '#ff6600' },
  });

  const previewSrc = map.getSource('__geo_draw_preview');

  function updatePreview() {
    const verts = state.vertices;
    if (verts.length === 0) {
      previewSrc.setData({ type: 'FeatureCollection', features: [] });
      return;
    }
    const lineCoords = [...verts, verts[0]];
    const features = [
      { type: 'Feature', geometry: { type: 'LineString', coordinates: lineCoords }, properties: {} },
      ...verts.map(c => ({ type: 'Feature', geometry: { type: 'Point', coordinates: c }, properties: {} })),
    ];
    previewSrc.setData({ type: 'FeatureCollection', features });
  }

  const onClick = (e) => {
    if (!state.active) return;
    const pt = e.lngLat;
    state.vertices.push([pt.lng, pt.lat]);
    updatePreview();
  };
  const onDblClick = () => { if (state.active && state.vertices.length >= 3) finishDrawing(); };
  const onContextMenu = (e) => {
    e.preventDefault();
    if (state.active && state.vertices.length >= 3) finishDrawing();
  };
  const onMouseMove = (_e) => {};
  const onKeyDown = (e) => {
    if (!state.active) return;
    if (e.key === 'Escape') cancelDrawing();
    else if (e.key === 'Enter' && state.vertices.length >= 3) finishDrawing();
    else if (e.key === 'Backspace' && state.vertices.length > 0) { state.vertices.pop(); updatePreview(); }
  };

  const finishDrawing = () => {
    state.active = false;
    canvas.style.cursor = '';
    const coords = [...state.vertices, state.vertices[0]];
    const aoi = {
      type: 'Feature',
      properties: { drawnAt: new Date().toISOString() },
      geometry: { type: 'Polygon', coordinates: [coords] },
    };
    map.fire('aoi-drawn', { aoi });
  };
  const cancelDrawing = () => {
    state.active = false;
    state.vertices = [];
    canvas.style.cursor = '';
    updatePreview();
  };

  state.active = true;
  state.vertices = [];
  canvas.style.cursor = 'crosshair';

  map.on('click', onClick);
  map.on('dblclick', onDblClick);
  map.on('contextmenu', onContextMenu);
  map.on('mousemove', onMouseMove);

  ctx.drawing = state;
  ctx.drawing.cancel = cancelDrawing;
  ctx.drawing.finish = finishDrawing;
  ctx.drawing._onKey = onKeyDown;
  ctx.drawing._onClick = onClick;
  ctx.drawing._onDblClick = onDblClick;
  ctx.drawing._onCtx = onContextMenu;
  ctx.drawing._onMove = onMouseMove;

  window.addEventListener('keydown', onKeyDown);
  console.log('geo-toolbox: drawing enabled — click to add vertices, double-click to finish, Esc to cancel');
}


/**
 * Cancel current drawing session and remove listeners.
 * @param {object} ctx
 */
export function disableDrawing(ctx) {
  const d = ctx.drawing;
  if (!d) return;
  d.cancel?.();
  window.removeEventListener('keydown', d._onKey);
  ctx.map.off('click', d._onClick);
  ctx.map.off('dblclick', d._onDblClick);
  ctx.map.off('contextmenu', d._onCtx);
  ctx.map.off('mousemove', d._onMove);
  try { ctx.map.removeLayer('__geo_draw_preview_vertices'); } catch (_) {}
  try { ctx.map.removeLayer('__geo_draw_preview_line'); }    catch (_) {}
  try { ctx.map.removeSource('__geo_draw_preview'); }        catch (_) {}
  ctx.drawing = null;
  ctx.map.getCanvas().style.cursor = '';
}
