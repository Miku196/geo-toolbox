/**
 * crs.js — CRS 坐标参考系重投影。
 *
 * 依赖：ctx.map（MapLibre map）、ctx.engine.crs（WASM CrsEngine）。
 * 职责：把 MapLibre 上的矢量 source 从一个 EPSG 已编码坐标系重投影到另一个。
 *
 * ctx 约定（index.js 构造并注入，见 index.js 顶部注释）：
 *   ctx.map             MapLibre map 实例
 *   ctx.engine          WASM 引擎容器（{ crs, carbon, tile }）
 *   ctx.assertReady()   若 WASM 未加载则抛错
 */


/**
 * Reproject a vector source from one CRS to another.
 * @param {object} ctx  plugin 上下文
 * @param {string} sourceId  MapLibre source id
 * @param {{from: number, to: number}} crs  EPSG codes
 */
export async function transformLayer(ctx, sourceId, { from, to }) {
  ctx.assertReady();
  const source = ctx.map.getSource(sourceId);
  if (!source) throw new Error(`Source "${sourceId}" not found`);
  const data = collectSourceData(source);
  const transformed = transformCoords(ctx, data, from, to);
  updateSource(ctx, sourceId, transformed);
}


/** 收集 source 的 GeoJSON 数据；非 geojson source 返回空 FeatureCollection。 */
function collectSourceData(source) {
  if (source.type === 'geojson' && source._data) return source._data;
  return { type: 'FeatureCollection', features: [] };
}


/** 若 source 支持 setData，则将重投影后的数据写回。 */
function updateSource(ctx, sourceId, data) {
  const source = ctx.map.getSource(sourceId);
  if (source?.setData) source.setData(data);
}


/** 遍历 FeatureCollection，对每个坐标应用 (lon,lat)=>(lon',lat') 变换。 */
function transformCoords(ctx, geojson, from, to) {
  ctx.assertReady();
  const features = geojson.features;
  if (!features) return geojson;
  for (const f of features) {
    if (f.geometry?.coordinates) {
      f.geometry.coordinates = walkCoords(f.geometry.coordinates, (lon, lat) => {
        const pt = ctx.engine.crs.transform(lon, lat, from, to);
        return [pt[0], pt[1]];
      });
    }
  }
  return geojson;
}


/** 递归遍历坐标数组：叶子为 [lon, lat] 数字对时应用 fn。 */
function walkCoords(coords, fn) {
  if (Array.isArray(coords) && typeof coords[0] === 'number') {
    return fn(coords[0], coords[1]);
  }
  return coords.map(c => walkCoords(c, fn));
}
