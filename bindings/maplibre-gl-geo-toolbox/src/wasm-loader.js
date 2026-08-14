/**
 * wasm-loader.js — WASM 模块加载与引擎生命周期。
 *
 * 职责：动态 import wasm-pack 胶水脚本、执行其默认初始化，并实例化
 *       CrsEngine / CarbonEngine / TileEngine 三个引擎，返回封装对象。
 * 不依赖插件实例，便于独立测试与复用。
 *
 * 注意：产物 pkg/geo_wasm_bg.wasm 由 Rust 侧构建，本模块不做任何 pkg 改动。
 */


/**
 * Load the WASM engine. Call once before using compute* methods.
 *
 * @param {string} wasmPath  path to wasm-pack JS glue
 * @returns {Promise<{wasm: object, engine: {crs: object, carbon: object, tile: object}}>}
 */
export async function loadWasmEngine(wasmPath) {
  let wasm;
  let engine;
  try {
    wasm = await import(/* @vite-ignore */ wasmPath);
    await wasm.default();
    engine = {
      crs:    new wasm.CrsEngine(),
      carbon: new wasm.CarbonEngine(),
      tile:   new wasm.TileEngine(),
    };
  } catch (e) {
    console.error('geo-toolbox: WASM failed to load', e);
    throw e;
  }
  return { wasm, engine };
}
