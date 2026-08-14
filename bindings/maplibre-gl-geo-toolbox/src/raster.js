/**
 * raster.js — 栅格波段运算。
 *
 * 依赖：ctx.wasmModule()（raw wasm 模块的 bandAdd/bandSub/bandMul/bandDiv/
 *       bandThreshold/resampleNearest/resampleCubic/computeZonalStats 自由函数）。
 * 职责：双波段算术、阈值分割、重采样、分区统计。
 *
 * ctx 约定见 crs.js 顶部注释。
 */


/**
 * Band arithmetic on two same-size Float64 arrays.
 * @param {object} ctx
 * @param {Float64Array|number[]} a   band A data
 * @param {Float64Array|number[]} b   band B data
 * @param {Object} opts
 * @param {'add'|'sub'|'mul'|'div'} [opts.op='add']   operation
 * @param {number} [opts.rows=1]
 * @param {number} [opts.cols]
 * @returns {Object} {data, rows, cols, nodata}
 */
export function bandMath(ctx, a, b, opts = {}) {
  ctx.assertReady();
  const aArr = Array.from(a);
  const bArr = Array.from(b);
  if (aArr.length !== bArr.length) {
    throw new Error('bandMath: arrays must have same length');
  }
  const rows  = opts.rows || 1;
  const cols  = opts.cols || aArr.length;
  const wasmMod = ctx.wasmModule();
  let json;
  switch (opts.op || 'add') {
    case 'sub': json = wasmMod.bandSub(aArr,  rows, cols, bArr, rows, cols); break;
    case 'mul': json = wasmMod.bandMul(aArr,  rows, cols, bArr, rows, cols); break;
    case 'div': json = wasmMod.bandDiv(aArr,  rows, cols, bArr, rows, cols); break;
    default:    json = wasmMod.bandAdd(aArr,  rows, cols, bArr, rows, cols); break;
  }
  return JSON.parse(json);
}


/**
 * Threshold a raster band.
 * @param {object} ctx
 * @param {Float64Array|number[]} data   pixel values
 * @param {number} rows
 * @param {number} cols
 * @param {number} threshold
 * @returns {Object} {data, rows, cols, nodata}
 */
export function bandThreshold(ctx, data, rows, cols, threshold) {
  ctx.assertReady();
  const arr = Array.from(data);
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.bandThreshold(arr, rows, cols, threshold);
  return JSON.parse(json);
}


/**
 * Resample raster data to new dimensions.
 * @param {object} ctx
 * @param {Float64Array|number[]} data   pixel values
 * @param {number} srcRows
 * @param {number} srcCols
 * @param {number} dstRows
 * @param {number} dstCols
 * @param {Object} [opts]
 * @param {'nearest'|'cubic'} [opts.method='nearest']
 * @param {number} [opts.nodata=null]
 * @returns {number[]} resampled pixel array
 */
export function resample(ctx, data, srcRows, srcCols, dstRows, dstCols, opts = {}) {
  const arr = Array.from(data);
  const nodata = opts.nodata ?? null;
  const wasmMod = ctx.wasmModule();
  if (opts.method === 'cubic') {
    return wasmMod.resampleCubic(arr, srcRows, srcCols, dstRows, dstCols, nodata);
  }
  return wasmMod.resampleNearest(arr, srcRows, srcCols, dstRows, dstCols, nodata);
}


/**
 * Compute zonal statistics.
 * @param {object} ctx
 * @param {Float64Array|number[]} values  pixel values
 * @param {Uint32Array|number[]}  zones   zone IDs (1-indexed)
 * @param {number} numZones
 * @param {number} [nodata=null]
 * @returns {{zones: Array<{count,min,max,mean,stddev,sum}>}}
 */
export function computeZonalStats(ctx, values, zones, numZones, nodata = null) {
  ctx.assertReady();
  const valArr = Array.from(values);
  const zoneArr = Array.from(zones);
  if (valArr.length !== zoneArr.length) {
    throw new Error('computeZonalStats: values and zones must have same length');
  }
  const wasmMod = ctx.wasmModule();
  const json = wasmMod.computeZonalStats(valArr, zoneArr, numZones, nodata);
  return JSON.parse(json);
}
