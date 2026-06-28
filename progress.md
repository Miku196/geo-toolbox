# Progress

## Status
In Progress — WASM & MapLibre 绑定补齐

## Tasks

### ✅ 完成 (2026-06-28)
- **WASM 架构重构** — 14/15 模块采用 `_inner` + `GeoResult<T>` 模式，原生测试通过
  - 模式：内部函数返回 `GeoResult<T>`，`#[wasm_bindgen]` 薄壳 `.map_err(JsValue::from_str)`
  - 涉及的模块：geohash, raster, vector, crs, carbon, tile, spatial, ingest, output, utils, debug
  - `storage.rs` 排除 (`#[cfg(target_arch = "wasm32")]`)
  - 结果：27/27 测试原生通过（之前 `test_invalid_decode` 因 JsValue::from_str panic 失败）

- **MapLibre 绑定补齐** — 5 组方法，511→817 行
  - NDVI 修复：`computeNdvi()` stub → 真 WASM 调用，+ `ndviDifference()`
  - Geohash (4)：`geohashEncode/Decode/Neighbors` + `bboxToGeohashes`
  - 矢量操作 (3)：`computeBuffer/Intersect/unionAll`
  - 空间分析 (5)：`computeArea/Bbox/Centroid` + `simplify/convexHull`
  - 栅格运算 (4)：`bandMath/Threshold` + `resample/computeZonalStats`
  - `init()` 现在存储 `this.#_wasm`（原始模块引用）用于自由函数调用

- **构建修复**
  - `geo_carbon_math::CarbonEngine` 缺少 `calculate_from_json_factors` → JSON→CSV 桥接实现
  - WASM 测试文件 `web.rs` 中 `band_add`→`bandAdd` 函数名更新

- **Warning 清零** — 19→0
  - ecology: `total_loss` 未使用 → `_total_loss`
  - hydro: `aquifer_thickness_m` 未使用 → `cargo fix`
  - socioeconomic: `mut` 不需要 → `cargo fix`
  - volcanology: non_snake_case (lava_viscosity_Pa_s) → `#![allow(non_snake_case)]`
  - wasm: bandAdd/Sub/Mul/Div → `#![allow(non_snake_case)]`
  - wasm: log_fn_call/FnGuard → `#![allow(dead_code)]`

- **全量回归** — `cargo check --workspace` 0 error 0 warning，`cargo test --workspace` 全绿

### 🔜 待办
- [ ] 更新 DEVPLAN.md WASM 生态章节
- [ ] 更新 ROADMAP.md 进度
- [ ] `wasm-pack build` 验证
- [ ] MapLibre 示例页面完善
- [ ] 提交代码

## Files Changed
参见 `git status` — 36 modified + 多个 untracked (WASM new: geohash, raster, vector, debug; MapLibre examples, vite.config.js)

## Notes

### 2026-06-28 — WASM & MapLibre binding completion
- WASM `_inner` 模式解决了原生测试障碍：之前 `#[wasm_bindgen]` 函数中 `JsValue::from_str()` 在 x86_64 上 panic
- `geo_carbon_math::CarbonEngine` API 只有 `calculate_from_geojson(csv)` 和 `calculate(json)`
  没有 `calculate_from_json_factors` — 在 WASM 层做了 JSON→CSV 格式转换桥接
- MapLibre 绑定采用厚包装模式：输入校验 + 默认值 + WASM 调用 + JSON 解析
- `init()` 现在存两份引用：`#engine` (struct 方法) + `#_wasm` (自由函数)
- `#[allow(non_snake_case)]` 用于 WASM export 函数 (bandAdd 等，需对齐 JS 命名)
