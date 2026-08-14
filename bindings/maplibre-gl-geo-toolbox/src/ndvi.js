/**
 * ndvi.js — NDVI（归一化植被指数）计算。
 *
 * 依赖：ctx.wasmModule()（raw wasm 模块的 computeNdvi / ndviDifference 自由函数）。
 * 职责：基于红波段 + 近红外波段数据计算 NDVI 及两期差异。
 *
 * ctx 约定见 crs.js 顶部注释。
 */


/**
 * Compute NDVI from red and NIR raster data.
 *
 * @param {object}            ctx
 * @param {string|Float64Array} red   red band data or source id
 * @param {Float64Array} [nir]        NIR band data
 * @param {Object} [opts]
 * @param {number} [opts.rows]        raster rows
 * @param {number} [opts.cols]        raster cols
 * @returns {Promise<Object>} JSON-parsed NDVI result
 */
export async function computeNdvi(ctx, red, nir, opts = {}) {
  ctx.assertReady();
  const wasmMod = ctx.wasmModule();

  const redData  = Array.isArray(red) ? new Float64Array(red) : red;
  const nirData  = Array.isArray(nir) ? new Float64Array(nir) : nir;
  const rows     = opts.rows || 1;
  const cols     = opts.cols || redData.length;

  if (!redData || !nirData) {
    throw new Error('computeNdvi: red and nir band data required (Float64Array or number[])');
  }
  if (redData.length !== nirData.length) {
    throw new Error('computeNdvi: red and nir arrays must have same length');
  }

  const redVec  = Array.from(redData);
  const nirVec  = Array.from(nirData);
  const actualRows = rows;
  const actualCols = rows * cols === redData.length ? cols : redData.length;

  try {
    const json = wasmMod.computeNdvi(redVec, nirVec, actualRows, actualCols);
    return JSON.parse(json);
  } catch (e) {
    console.error('computeNdvi: WASM call failed', e);
    throw e;
  }
}


/**
 * Compute NDVI difference between two time points.
 *
 * @param {object}       ctx
 * @param {Float64Array} prev  previous NDVI data
 * @param {number} prevRows
 * @param {number} prevCols
 * @param {Float64Array} curr  current NDVI data
 * @param {number} currRows
 * @param {number} currCols
 * @returns {Promise<Object>}
 */
export function ndviDifference(ctx, prev, prevRows, prevCols, curr, currRows, currCols) {
  ctx.assertReady();
  const wasmMod = ctx.wasmModule();
  const prevVec = Array.from(prev);
  const currVec = Array.from(curr);
  try {
    const json = wasmMod.ndviDifference(prevVec, prevRows, prevCols, currVec, currRows, currCols);
    return JSON.parse(json);
  } catch (e) {
    console.error('ndviDifference: WASM call failed', e);
    throw e;
  }
}
