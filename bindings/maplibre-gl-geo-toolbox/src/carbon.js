/**
 * carbon.js — 碳汇核算。
 *
 * 依赖：ctx.engine.carbon（WASM CarbonEngine）、ctx.wasmModule()（面积回退）。
 * 职责：根据绘制 AOI 与地类因子表计算年碳汇量（tCO₂e / yr）。
 *
 * ctx 约定见 crs.js 顶部注释，额外用到：
 *   ctx.wasmModule()  返回 raw wasm 模块（自由函数），内含 computeArea
 */


/** IPCC Tier 1 默认地类碳汇因子表。 */
const DEFAULT_FACTORS = [
  { category: 'forest',       factor_value: 4.80,  source: 'IPCC Tier 1' },
  { category: 'grassland',    factor_value: 1.20,  source: 'IPCC Tier 1' },
  { category: 'cropland',     factor_value: 0.80,  source: 'IPCC Tier 1' },
  { category: 'wetland',      factor_value: 6.50,  source: 'IPCC Tier 1' },
  { category: 'settlement',   factor_value: 0.10,  source: 'IPCC Tier 1' },
  { category: 'shrubland',    factor_value: 2.30,  source: 'IPCC Tier 1' },
  { category: 'barren',       factor_value: 0.05,  source: 'IPCC Tier 1' },
  { category: 'water',        factor_value: 0.00,  source: 'IPCC Tier 1' },
];


/**
 * Compute carbon sink for a drawn AOI.
 *
 * @param {object}          ctx
 * @param {object}          aoi       GeoJSON Polygon Feature
 * @param {Object}          [params]
 * @param {string}          [params.landcover]  land-cover class for lookup
 * @param {Object[]|string} [params.factors]    custom factors (JSON array or JSON string)
 * @param {number}          [params.year]
 * @returns {Promise<object>}  { totalSink_tco2_yr, breakdown, year, aoiArea_ha }
 */
export async function computeCarbonSink(ctx, aoi, params = {}) {
  ctx.assertReady();

  const landcover = params.landcover || 'forest';
  const year = params.year || new Date().getFullYear();
  const factors = buildFactors(landcover, params.factors);

  let fc;
  if (aoi.type === 'FeatureCollection') {
    fc = aoi;
  } else if (aoi.type === 'Feature') {
    fc = { type: 'FeatureCollection', features: [aoi] };
  } else {
    fc = {
      type: 'FeatureCollection',
      features: [{ type: 'Feature', properties: { class: landcover }, geometry: aoi }],
    };
  }

  for (const f of fc.features) {
    if (!f.properties) f.properties = {};
    if (!f.properties.class) f.properties.class = landcover;
  }

  const geojsonStr = JSON.stringify(fc);
  const factorsJson = JSON.stringify(factors);

  let result;
  try {
    const json = ctx.engine.carbon.calculateWithJsonFactors(geojsonStr, factorsJson, year);
    result = JSON.parse(json);
  } catch (e) {
    console.error('carbon.calculateWithJsonFactors failed', e);
    throw e;
  }

  const areaHa = computeArea(ctx, aoi);

  return {
    totalSink_tco2_yr: result.total_tco2e ?? 0,
    breakdown:          result.breakdown ?? [],
    year,
    aoiArea_ha:         areaHa,
    raw:                result,
  };
}


/** 组装因子表：优先用户自定义；否则用默认表，缺地类则补一条 factor=1.0。 */
function buildFactors(landcover, customFactors) {
  let base;
  if (customFactors) {
    base = typeof customFactors === 'string' ? JSON.parse(customFactors) : customFactors;
  } else {
    base = [...DEFAULT_FACTORS];
  }
  const has = base.some(f => f.category === landcover);
  if (!has) base.push({ category: landcover, factor_value: 1.0, source: 'user' });
  return base;
}


/**
 * Compute area (hectares) for a GeoJSON feature — uses WASM, falls back to JS approx.
 */
function computeArea(ctx, aoi) {
  if (aoi.properties?.area_ha) return aoi.properties.area_ha;

  // Extract geometry object from AOI (Feature, FeatureCollection, or bare Geometry)
  let geom = aoi;
  if (aoi.type === 'FeatureCollection') geom = aoi.features?.[0]?.geometry ?? aoi;
  else if (aoi.type === 'Feature') geom = aoi.geometry ?? aoi;
  if (!geom || !geom.type || !geom.coordinates) return 100;

  // Try WASM computeArea for geodesic area
  try {
    const wasm = ctx.wasmModule();
    if (wasm && wasm.computeArea) {
      const result = JSON.parse(wasm.computeArea(JSON.stringify(geom)));
      if (result.area_ha > 0) return result.area_ha;
    }
  } catch (_) { /* fall through to JS approximation */ }

  // JS fallback: planar approximation
  const coords = geom.coordinates?.[0];
  if (!coords || coords.length < 3) return 100;
  const lons = coords.map(c => c[0]);
  const lats = coords.map(c => c[1]);
  const lonSpan = Math.max(...lons) - Math.min(...lons);
  const latSpan = Math.max(...lats) - Math.min(...lats);
  const midLat = (Math.max(...lats) + Math.min(...lats)) / 2;
  const degToM = 111_320 * Math.cos((midLat * Math.PI) / 180);
  const areaSqm = (lonSpan * degToM) * (latSpan * 111_320);
  return Math.round((areaSqm / 1e4) * 100) / 100;
}
