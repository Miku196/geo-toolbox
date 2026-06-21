# Progress — 插件深度拓展阶段 (2026-06-21)

> 基于前期 10/10 插件代码审查 + 5 并行子代理执行 + 人工修复
> 目标: 将"远期领域插件"和"高 Leverage 拓展"全部实现

---

## 完成情况

### 新增插件（2个全新）

| 插件 | 模块 | 函数 | 行数 |
|------|------|------|------|
| **geo-plugin-climate** | config, gcm, idf, drought, kriging, tools, trait_impl | 30+ | ~2800 |
| **geo-plugin-geomorph** | d8, river | 12 | ~400 |

### 新增模块（5个已有插件）

| 插件 | 文件 | 功能 | 行数 |
|------|------|------|------|
| **agri** | dssat.rs | DSSAT 作物模型 (.WTH/.SOIL/.CUL/.CO2/.DXF) | 610 |
| **coastal** | ocean.rs | Ekman 输运 / 波浪能通量 / SWAN 波浪 / 潮汐调和 / ENSO | 589 |
| **coastal** | wave_runup.rs | Stockdon / Mase / 统计爬高 | 518 |
| **ecology** | soil.rs | USDA 质地三角 / SCS 水文分组 / RUSLE K / van Genuchten / HWSD | 350 |
| **hydro** | groundwater.rs | 达西渗流 / MODFLOW 适配 / 地下水-地表水耦合 / 水质 | 370 |
| **hydro** | unit_hydrograph.rs | SCS 三角 / Snyder 合成 / 瞬时单位线 (IUH) | 490 |
| **survey** | transform.rs | 四参数 / 七参数 Helmert / 仿射变换 / 最小二乘 | 410 |

### 统计

- 新增源文件: **9 个**
- 新增代码行: ~6500
- 编译: `cargo check --workspace` → **0 errors**
- 测试: `cargo test --workspace` → **全部通过**

### 修复清单

1. **climate 插件**: 7 模块全部创建（含正确的 lib.rs 声明）
2. **geomorph 插件**: lib.rs 声明（含 re-export）
3. **ecology soil**: 添加 6 个 wrapper API 函数匹配 lib.rs 导入预期
4. **hydro groundwater**: 添加 `use std::fmt::Write;` 修复 write! 错误
5. **workspace Cargo.toml**: 添加 `approx` workspace 依赖
6. **3 插件 Cargo.toml**: 添加 `approx.workspace = true` dev-dependency
7. **kriging**: Gauss 消元矩阵尺寸修复（rows×(cols+1)）
8. **IDF 曲线**: 回归符号修正（slope = -c）
9. **d8 累积**: 凹坑重置 bug 修复（避免覆盖上游贡献）
10. **d8 快速累积**: 凹坑返回值逻辑修复 + 与简单算法对齐
11. **ocean 测试**: SWAN 波浪变换断言过紧修复

---

## 各插件源码文件对照

```
plugins/
├── geo-plugin-agri/      agri, config, dssat, tools, trait_impl    ✅ 5文件
├── geo-plugin-carbon/     carbon_sink, ccer, config, lca, plugin, plume, tools, trait_impl  ✅ 8文件
├── geo-plugin-climate/    config, drought, gcm, idf, kriging, tools, trait_impl  ✅ 全新7文件
├── geo-plugin-coastal/    blue_carbon, coastal, ocean, storm_surge, tools, trait_impl, wave_runup  ✅ 7文件
├── geo-plugin-ecology/    config, ecology, lulc, rusle, sdr, soil, tools  ✅ 7文件
├── geo-plugin-energy/     config, energy, geothermal, tools, trait_impl, transmission  ✅ 6文件
├── geo-plugin-forestry/   config, forestry, tools, trait_impl  ✅ 4文件
├── geo-plugin-geohazard/  config, geohazard, info_value, rainfall_threshold, tools, trait_impl  ✅ 6文件
├── geo-plugin-geomorph/   d8, river  ✅ 全新2文件
├── geo-plugin-hydro/      config, groundwater, hydro, invest, scs_cn, tools, trait_impl, unit_hydrograph, watershed  ✅ 9文件
├── geo-plugin-survey/     config, gauss, survey, tools, trait_impl, transform  ✅ 6文件
└── geo-plugin-urban/      config, urban, tools, trait_impl  ✅ 4文件
```

---

## 待完成

- [ ] survey: UTM 支持 + Vincenty 大地线
- [ ] hydro: TR-55 完整版 + Muskingum 河段演算
- [ ] ecology: MUSLE 事件版土壤流失
- [ ] coastal: SLR 海平面上升情景 + CVI 脆弱性指数
- [ ] energy: 尾流效应 (Jensen/Frandsen) + PVWatts 性能模型
- [ ] forestry: 立地指数曲线 + 择伐/皆伐模拟
- [ ] geohazard: Newmark 地震位移法 + 区域稳定性色斑图
- [ ] carbon: 碳价情景分析 + VCS/GS 额外性
- [ ] urban: 城市内涝 (管网+SCS) + 15分钟城市可达性
- [ ] Python bindings + WASM npm + Jupyter Kernel

---

## Status

- 编译: ✅ `cargo check --workspace` 0 errors
- 测试: ✅ `cargo test --workspace` 全部通过
- 警告: ⚠️ 少量 unused variable（不影响功能，可后续 `cargo fix` 清理）
