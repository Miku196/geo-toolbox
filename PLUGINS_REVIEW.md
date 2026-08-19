# geo-toolbox plugins/ 层只读代码评审

> 项目根 D:/geo，源码根 geo-toolbox/plugins/（tokensave 索引前缀 geo-toolbox/...）。本评审只读，未修改任何文件。覆盖 18 个 crate 逐文件通读（大文件 forestry.rs 942 行、coastal/ocean.rs 638 行、geohazard.rs 777 行均已通读）。所有问题均带 file:line 证据。

---

## 0. 分层总览与跨层共性问题

plugins/ 层共 18 个 crate：agri、atmosphere、carbon、climate、coastal、ecology、energy、forestry、geohazard、geomorph、groundwater、hydro、remote-sensing、seismology、socioeconomic、survey、urban、volcanology。

每个 crate 大体遵循同一范式：lib.rs（模块声明+重导出）+ config.rs（TOML/默认）+ 功能模块（纯 f64 数值函数）+ trait_impl.rs（Plugin 残余 + 可选 ProcessPlugin）+ tools.rs（geo_registry 宏注册）。Plugin 残余几乎所有 crate 都实现；**ProcessPlugin::execute（可调度的过程入口）覆盖极不均衡**——这是跨层第一问题。

跨层共性：
1. **ProcessPlugin::execute 缺失/畸形（跨层主问题）**：有 execute 的 12 个（carbon/climate/coastal/energy/forestry/geohazard/groundwater/hydro/survey/urban/agri/ecology）；完全无 execute（只有 Plugin）的 6 个（atmosphere、remote-sensing、seismology、socioeconomic、volcanology——只能靠 tools 调用，通用 Process 调度器无法驱动）；ecology 有 execute 但硬编码常量栅格。
2. registry tools.rs 与 trait_impl execute 双路径重复（energy/coastal/forestry/urban 的 mk 闭包+assess 各写一遍；coastal_shoreline 工具与 execute 完全重复）。
3. 同式多副本（生态 MUSLE×3、水文 SCS×3、Newmark×2、UTM/GK 复制等）。
4. JSON 参数一律 .unwrap_or(default) 静默兜底，与 schema 声明 required 矛盾 -> 缺失必填不报错、返回假数据（如 kriging cell_size=0 -> OOM 隐患）。
5. #![allow(missing_docs)] 遍布 12+ crate，文档门禁缺失。
6. 生产路径几乎无 unwrap/panic，但仍存在潜在 OOB/除零/NaN。

---

## 1. geo-plugin-climate

**职责一句话**：气候/气象——GCM 降尺度、IDF 曲线、干旱指数（SPI/SPEI/PDSI）、Kriging 插值。

**核心类型/入口**：ClimatePlugin（trait_impl.rs:7）；ProcessPlugin::execute 按 command 分发；tools.rs 注册 9 同名词条。

### ★ 关键问题判定：trait_impl.rs:123-125 的 Unimplemented 分支

`_ => Err(GeoError::Unimplemented(format!("unknown climate command: {command}")))`

**结论：这不是单纯防御性拒绝，而是既兜底、又掩盖了 2 个真实功能缺口。证据：**

(1) **错误变体语义错配**：geo_core::errors::GeoError::Unimplemented（core/geo-core/src/errors.rs:125-127）文档明确为 "Not-yet-implemented feature"。但本分支匹配的是未知命令字符串——本质是输入校验/调度失败；本项目自带惯例是用 GeoError::Validation（见 groundwater/trait_impl.rs:39）。用 Unimplemented 表达 "unknown climate command" 是把"调用者传了不存在的命令"误报成"功能未实现"。

(2) **掩盖真实功能缺口**：lib.rs:12 重导出 idf_return_period（idf.rs:117 完整实现）、lib.rs:14 重导出 simple_kriging（kriging.rs:269-276 完整实现，委托 ordinary_kriging）。这两个已完整实现、已公开导出的函数在 execute 分发器里都没有命令路由，climate/tools.rs:11-95 同样未注册。调用者经 execute 传 "simple_kriging" 只落入 Unimplemented；它们是有实现却无调度管线的死公开 API。

(3) **判定**：该分支主要是合理防御（未知输入返回错误不 panic），但 (a) 错误类型选错（应 Validation）；(b) 因两条已实现函数无路由而成为功能缺口的掩盖层。故属"半缺口"。

**其余问题**：
- idf.rs:72 idf_fit_params 跳过无效点时 nn 仍取全长 n，回归分母错误。
- idf.rs:98 ss_tot==0（定值强度）-> 1-ss_res/0 -> inf/NaN 无保护。
- kriging.rs:152-154 ordinary_kriging 对 cell_size==0 无校验：bbox 跨度/0 -> inf -> inf as usize 饱和到 usize::MAX -> **巨量分配/OOM 隐患**；bbox max>min 未校验。
- kriging.rs:221-223 奇异/共线点矩阵主元<1e-12 时 continue，垃圾权重静默输出。
- kriging.rs:188-259 每个格点内 Gauss-Jordan 重解 kriging 矩阵 -> O(cells·n^3)，应 LU 一次性分解+回代。
- kriging.rs:82 num_bins==0 -> 除零 + num_bins-1 usize 下溢 panic（tools.rs:91 允许 0）。
- kriging.rs:269-276 simple_kriging 只是 ordinary_kriging 别名，非真 simple kriging，误导+无路由。
- drought.rs:22-56 compute_spi 返回 Option，trait_impl.rs:79-84 直接 to_value 序列化 -> 数据不足返回 null 而非错误。
- drought.rs:141-145 Thornthwaite 热指数算错：在 12 个零数组上求和 (t/5)^1.514 共 12 次 = 单温×12，非 12 月热指数和。
- drought.rs:102-134 PDSI 是非标准手写水平衡（硬编码 0.5 耗竭因子、0.8 持久系数），非 Palmer CAFEC。
- drought.rs:69 compute_spei 硬编码纬度 30.0，而 compute_pdsi 收 lat 参数，API 不一致。
- gcm.rs:39-44 降水零历史比例强制 1.0（静默掩盖缺失），负 obs 不限幅。
- config.rs:9-12 base/projection_period_* 四个 u16 字段调度路径从未读取，死配置。

---

## 2. geo-plugin-coastal

**职责一句话**：海岸带——岸线变化、蓝碳核算、海平面上升淹没、风暴潮、海浪爬高、CVI。

**核心类型/入口**：CoastalPlugin（coastal.rs:146）；blue_carbon.rs 参照 IPCC 2013 Wetlands Supplement；ProcessPlugin::execute（trait_impl.rs:26-52）。

### ★ 关键问题：erosion_threshold_m 被放进 sea_level_rise_m 形参（语义错配，两个调用方都有）

- coastal.rs:177-187 assess_shoreline 最后一个形参是 sea_level_rise_m: f64。
- trait_impl.rs:49 传入 p["erosion_threshold_m"].as_f64().unwrap_or(1.0)；tools.rs:11 同样。
- 函数内部 sea_level_rise_m 只在 coastal.rs:217 用作**绝对 DEM 高程阈值**判淹没（elev < sea_level_rise_m -> inundated++），进而影响 :232 inundated_ha 与 :234 风险分级。
- **影响**：调用者传的"侵蚀阈值"(米)被当成"海平面高程"判淹没；而侵蚀判定 coastal.rs:211（NDVI 骤降）根本不用该阈值。结果：该参数对侵蚀零作用、对淹没起错误作用；默认 1.0 时所有 DEM<1m 像素被误计"已淹没"。

**其余**：
- coastal.rs:232 inundated_ha = inundated*0.01 硬编码 10m 像素->ha，与真实 cell_size 无关（trait_impl 读到 cols/rows 却未用）。
- storm_surge.rs:264 n_pts-1 空数组下溢 -> panic（debug）/OOB（release）。
- storm_surge.rs:157-169 经度距离用纬度 1°/111.32km 比例换算未乘 cos(lat)，远离赤道经向距离错误。
- storm_surge.rs:225 淹没体积=Σ浪高×面积（混淆浪高与淹没水深）；:206 把所有非陆地水格当淹没。
- wave_runup.rs:302 rc.max(0.0) 使淹没沙丘高流量兜底不可达：负 rc 被抹成 0 -> weir 分支 h=0.001 -> 返回 ~0.05 l/s/m，而 :306-311 的 q=100 兜底永不可达。
- ocean.rs:125-128 mhws/mlws/mhwn/mlwn 硬编码 0.0（文档称真实潮汐统计未算）；ocean.rs:120 空 predictions 均值除零。
- ocean.rs:417,423-428 _theta0、_kh 计算未用死代码；:442 kr=1.0 硬编码，折射/浅水项恒等。
- cvi.rs:95 "es carpment" 拼写错误（应为 escarpment），意图 match 落空。
- blue_carbon.rs、coastal.rs、wave_runup/slr 主体质量良好（IPCC 常数+测试完善）。
- tools.rs:6-7 引用 PluginCategory::Process 但 import 无（需 cargo check 确认）。
---

## 3. geo-plugin-forestry

**职责一句话**：林业碳汇——NDVI 碳储量变化、生长曲线/立地等级、模型校准验证、采伐模拟。

**核心类型/入口**：ForestryPlugin（forestry.rs:108）；GrowthModel 6 模型（Richards/Logistic/Korf/Gompertz/Weibull/Schumacher）；ProcessPlugin::execute（trait_impl.rs:27-55）。942 行中 665-942 为测试，核心逻辑约 662 行。

### ★ 关键问题：tools.rs:11 实参错位（area/volume 交换），与 execute 路径不一致

- assess_carbon_stock 签名（forestry.rs:279-290）：(aoi_name, aoi_geojson, base_red, base_nir, assess_red, assess_nir, base_year, assess_year, sample_volume_m3_ha, forest_area_ha)。
- trait_impl.rs:52-53 传参**正确**：baseline_volume_m3_ha->sample_volume_m3_ha, baseline_area_ha->forest_area_ha。
- **tools.rs:11 传参顺序交换**：assess_carbon_stock(..., year_old, year_new, baseline_area_ha, baseline_volume_m3_ha) —— baseline_area_ha 被当 sample_volume_m3_ha、baseline_volume_m3_ha 被当 forest_area_ha。**registry 碳储量工具与 execute 结果不同**（面积与蓄积量互换），真实正确性 bug。

**其余**：
- site_classification（forestry.rs:144-198）文档声称迭代收敛，实为无迭代：iterations: max_iter.min(1)（:196）、growth_rate 0.04、shape 0.76 硬编码；asymptotes 只是 avg_h*1.05（:187），近似与文档不符。
- calibrate_growth_model RMSE（:515-524,:559-568）用 .sqrt()/n 应为 sqrt(sum/n)，因 n 恒定影响对比不影响选参；上报 rmse 正确。
- find_interpolated_height（:613-618）无空数组保护，空 ages 索引 panic。
- 生长模型/校准/立地模块整体扎实（黄金分割、网格搜索、R2/AIC/BIC），942 行大文件非空壳。

---

## 4. geo-plugin-carbon（旗舰）

**职责一句话**：碳核算——IPCC Tier1/2 排放因子、5 库储量、CCER/VCS、碳价/收入、高斯烟羽->AOD->PM2.5。

**核心类型/入口**：CarbonPlugin（plugin.rs:11）封 geo-carbon-math CarbonEngine + 可选 EfDatabase；plugin.rs 固有方法 + trait_impl.rs 仅实现 Plugin/ProcessPlugin 委托 Cp::load（干净分层，无冲突）。

**问题**：
- carbon_sink.rs:100-112 vs 131-142 **聚合不自洽**：total_tco2e 只用全局 mean_ndvi，by_polygon 用各自 poly_ndvi_ratio，二者之和不等；:135 每多边形面积 = healthy_area_ha/features.len() 等分（非几何面积）。
- carbon_sink.rs:9,101 把经验回归 AGB=135.53×NDVI-16.76 标为 "IPCC Tier 2"（误导归因），注释自承 from literature。
- carbon_price.rs:52-58 year_end_price 每轮从 start 重算只反映最后一年，与 annual_revenues off-by-one。
- tools.rs:76 carbon_vcs_additionality 恒传空 [] evidence -> overall_pass 恒 false、score 恒 0，工具永远过不了（stub）。
- vcs_gs.rs:5-7 project_type/baseline_scenario 收而不用（仅回显 JSON）。
- plume_ext.rs:115-148 AOD->PM2.5 的 aod_ratio 默认 0.025 注释写 μg/m³ 却被 *1000-2 背景扣除，魔法常数无来源；与 atmosphere/aod_pm25.rs 跨 crate 重复。
- plume_ext.rs 的边界层/感热/潜热（硬编码 70%/50% RH）属 atmosphere 职责却在 carbon 里。
- lca.rs:94-100 测试改全局 env BRIGHTWAY2_URL -> 并行测试竞争。

---

## 5. geo-plugin-energy

**职责一句话**：新能源选址（光/风/地热/PV/输电廊道/尾流）。

**核心类型/入口**：EnergyPlugin（energy.rs:47）、GeothermalAssessment、TurbineParams、transmission/wake 模块；trait_impl execute 仅太阳能。

**问题**：
- **crate 内双重模型**：energy.rs:292-382 TurbineSpec+compute_turbine_power+compute_cp 与 turbine.rs:35-177 TurbineParams+power_curve 重复；energy.rs:221-285 weibull_fit+gamma_approx 与 turbine.rs:187-210 AEP 重复。
- energy.rs:90-95,141,209 坡度 = flat 数组相邻像素 atan(|elev[i]-elev[i-1]|/10)——行边界跨行比较、非二维、10m 硬编码；:141/:209 硬编码 *0.01 ha（假定 10m 像素），其他分辨率下 suitable_area_ha 错。
- energy.rs:98 rad_factor=(rad/2000).min(1.0) 魔法 2000（doc 却说 1500）。
- transmission.rs:245 代价加权 (1+avg_cost*5.0)/200，:313-314 把代价加权距离当几何长度算面积/造价——成本与物理单位混用。
- transmission.rs:66-79 slope_cost 在 5° 不连续（5->7.5），影响 Dijkstra。
- geothermal.rs:89 lcoe power<=0 时 f64::INFINITY（序列化 null/inf）。
- tools/execute 全 .unwrap_or(default) 无范围校验。
---

## 6. geo-plugin-groundwater

**职责一句话**：抽水试验（Cooper–Jacob/Theis）、补给（水平衡/Turc/氯）、水位趋势、比容量->T。

**核心类型/入口**：free 函数 + GroundwaterPlugin（groundwater.rs:361）薄转发；trait_impl execute 仅 action==recharge。

**问题**：
- groundwater.rs:134 `let _gamma = -0.5772156649015329f64.ln()`——对欧拉常数再取 ln，数学无意义死代码（下方 :135 正确用 -0.577-ln(u)）。
- groundwater.rs:332 趋势正斜率标注 "Rising (water table declining)"（深度向下为正）：反转语义正确但反直觉；:344-345 投影无上限外推。
- trait_impl.rs:28-40 execute **只支持 recharge**，其余全返回 Validation（Unknown action），是插件大部分功能的 stub 调度。
- turc_et ×30.0（假定每月固定 30 天）近似无说明。
- groundwater.rs:104-106 储水系数的 t0 单位转换 /(60*24*r*r) 晦涩且无数量级测试。

---

## 7. geo-plugin-hydro

**职责一句话**：水文——D8 汇流、SCS-CN/TR-55 径流、单位线、马斯京根演算、淹没、融雪、InVEST 碳/水、MODFLOW 适配。

**核心类型/入口**：HydroPlugin（hydro.rs:47，new(config) + with_modflow_generator）；trait_impl execute->assess。

### ★ 关键问题：trait_impl.rs:20 panic!（latent 但最该修）

`fn new(_config) -> Self { panic!("HydroPlugin must be constructed via HydroPlugin::new(config, modflow), not Plugin::new()") }`

- **消息过期/错误**：真实构造器是 HydroPlugin::new(config)（hydro.rs:53，单参）+ 独立 .with_modflow_generator(box)（:61），**不存在 new(config, modflow) 双参**。
- **可达性**：全库检索无任何 <P as Plugin>::new 调用（tools.rs:46 用 HydroPlugin::new(Default::default())），当前是 latent panic；但任何未来通用插件加载器走 Plugin::new 就崩宿主而非返回错误。
- **应对**：这里完全可直接 Self::new(config)（modflow 默认 None），无需 panic。生产代码唯一 panic 点，最该优先改。

**其余（大量重复水文）**：
- SCS 单位线两套：hydro.rs:325 scs_unit_hydrograph（t_peak=0.6tc+0.5、qp=0.208AQ/tp）vs unit_hydrograph.rs:150-190 scs_uh（tp=0.6tc、qp=(pf/2321)AQ/tp）——参数化不一致。
- D8 表 d8_dr/dc/diag 在 hydro.rs:90-119 与 :266-295 两处复制。
- SCS-CN S/Ia/runoff 三处实现：scs_cn.rs:38-242、tr55.rs:217-247、groundwater.rs:233-242。
- hydro.rs:376 assess 把 peak_discharge_m3s*3600 当体积传淹没分析——峰流当 1h 径流体积，夸大淹没。
- hydro.rs:181 estimate_inundation_area 硬编码深度 0.3m、从不读 config.flood 的 return_period/manning。
- tools.rs:56 hydro_runoff 假定 1-impervious 全草，无土地比例校验。
- hydro.rs:49,56,61 modflow_generator 字段只写不读，MODFLOW DI seam 惰性。

---

## 8. geo-plugin-geohazard

**职责一句话**：滑坡+泥石流危险性/涌出、Newmark 位移、降雨阈值、信息量统计。777 行 geohazard.rs 多为真实数学非空壳。

**核心类型/入口**：GeohazardPlugin（geohazard.rs:133）；trait_impl execute 按 task 分发 landslide/debris_flow/默认综合。

**问题**：
- **info_value.rs 死代码且未编译**：lib.rs:3-8 只声明 config/geohazard/newmark/rainfall_threshold/tools/trait_impl，没有 info_value——整门信息量敏感性方法不可达。
- **Newmark 两套不同公式**：geohazard.rs:536（插件方法，tools 用的 geohazard_newmark）vs newmark.rs:8（free fn）——同一输入 FS/slope/PGA 因路径不同得不同位移。
- geohazard.rs:358 factor_of_safety 平坦时返回 99.0 哨兵值，泄漏进公开 FS 并硬编码进 Newmark（if FS>=99.0）。
- geohazard.rs:508-518 涌出风险按 50/200/500/1000m 绝对距离分档，无尺度归一。
- config.rs:234 有 total_weight() 助手却从不强制：六权重可任意求和，坏配置静默出垃圾敏感性。
- geohazard.rs:334 estimate_volume 把体积(㎡)与密度混成质量。
---

## 9. geo-plugin-urban

**职责一句话**：城市规划——容积率/高度、NLCD 分类、日照阴影、UHI、通风廊道、内涝、15 分钟生活圈。整体干净、小、良好测试。

**问题**：
- urban.rs:281 assess 硬编码 solar_analysis(avg_h, 30.0)——邻距 30m 固定，日照合规判定忽略实际布局。
- urban.rs:125-128 纯 NDVI 输入永不返回 urban 类别（只出 GreenSpace/BareSoil），类别静默塌缩。
- urban.rs:233-258 ventilation：density>=0.6 时 z0 饱和 -> vi = 1-0.06h/0.06h = 0.0，指标在该区间退化为常数。
- urban_flood.rs:73-82 vs 173-182 同一三级风险分类复制两遍。
- urban.rs:139 land_use_stats 用 max(len) 而非 zip，NDVI/impervious 长度不一时错配配对。

---

## 10. geo-plugin-geomorph

**职责一句话**：纯地学库——D8 流向/累积、河流提取、Strahler 级、河谷断面。确认无 trait_impl/config/Plugin（但 tools.rs:6 仍 register_plugin，非纯无框架）。

**问题**：
- **river.rs:55-56,81-82,283 未检查符号->usize 转换**：(r as isize + D8_DR[d]) as usize 在顶行朝北排水时负 isize 回绕成超大 usize -> 越界索引（真实 DEM 边界单元 panic/OOB）。d8.rs 显式检查 <0，river.rs 没有——同 crate 两种处理。
- d8.rs:84-133 与 137-237 两个公开 flow accumulation 不同约定（下游遍历+1 vs 上游集水区）；简单版 O(n²) 且坑/边缘单元格被高估。
- d8.rs:247-281 d8_flow_direction_filled 只填单格坑，JD style 名不副实，真实扁平区仍为坑。
- river.rs:293-297 河段在每次 order 变化处截断（Strahler 阶沿程不降->过高切分）。

---

## 11. geo-plugin-agri

**职责一句话**：农业——CASA NDVI->LAI->NPP 估产、土壤评级、USLE/土壤碳、灌溉、DSSAT 输入。

**核心类型/入口**：AgriPlugin（agri.rs:7，含 Box<dyn DssatGenerator>）；trait_impl execute 在 agri.rs:341-365。

**问题**：
- **agri.rs:318-322 Plugin::new 无条件 panic!**（非测试生产代码）——同 hydro，通用加载器会崩；而真实 AgriPlugin::new 其实能建 None 生成器，trait 版不该 panic。
- agri.rs:240 与 dssat.rs:139 (-tan(lat)*tan(decl)).acos() 未夹到 [-1,1]，高纬/冬季 -> NaN 传播（.max(0.0) 挡不住 NaN）。
- tools.rs:69 agri_lai 单位/语义颠倒：estimate_npp 已返回每日 NPP(gC/m2/day)，工具却 npp_gcm2_day: npp/120、npp_gcm2_season: npp——把日值除 120 当日、把日值当季。
- agri.rs:80-113 fallback 名不副实：两种估产方法无条件各算再平均（:109），无真兜底。
- agri.rs:236-248 与 dssat.rs:134-146 重复地外辐射数学。
- dssat.rs:105-113 切片索引无长度校验（<12 就 OOB panic）；降雨重分配 |sin(julian*7)|*2.0 不守恒月总量。
- dssat.rs 自由函数在运行时图全是死代码（仅测试用），真正生成委托注入 DssatGenerator（tools 用 Noop 返空串）。
- soil.rs:100-104 K 因子 TODO；soil.rs:37 a/13.0 硬编码 13 t/ha/mm。
- lib.rs:10 过期注释（DSSAT 类型经 geo_core::traits 导入——实际没有）。

---

## 12. geo-plugin-atmosphere

**职责一句话**：大气——边界层（ABL/热通量/莫宁-奥布霍夫）、Pasquill-Gifford 高斯烟羽、AOD550->PM2.5->AQI。

**核心类型/入口**：AtmospherePlugin（trait_impl.rs）；**确认无 ProcessPlugin**（只有 Plugin）；tools.rs 注册 4 工具。

**问题**：
- **MISSING ProcessPlugin::execute（确认）**：trait_impl.rs:1-42 只实现 Plugin。agri 和 survey 都有 execute，atmosphere 是这三 crate 里唯一缺 execute 的，通用 Process 管线无法调度它——主不一致点，最该补。
- trait_impl.rs:15-17 load(_path) 忽略路径返回默认配置 stub。
- aod_pm25.rs:58-71 pm25_to_aqi：mean_aqi 对逐点 u32 AQI 取整均值，classification 用 mean_pm25 派生——两个 mean 驱动不同输出不一致。
- aod_pm25.rs:41 hazardous 分支增幅封顶 200 -> pm25>500.4 时 AQI>500 未 clamp 500。
- boundary_layer.rs:121-127 湿度仅饱和水汽压+硬编码 RH(0.70/0.50)；:125 .max(0.0) 使 LHF 永不为负（蒸发/凝结符号丢失）。
- boundary_layer.rs:93-94 friction_velocity 用 (ln(10/z0)-psi_m).max(0.1)：z0>=~10e 时 u* 爆大。
- dispersion.rs:33-54 km<->m 双重换算脆弱；tools.rs:24,43 非法 stability 串静默回退 D（s.chars().next().unwrap_or(D)）。
- tools.rs:12-35 filter_map 静默丢非数值元素、无判空。
---

## 13. geo-plugin-seismology

**职责一句话**：地震——PGA/PGV 衰减、PSHA、地震目录工具。

**入口**：SeismologyPlugin（trait_impl.rs:5）**确认无 ProcessPlugin**（仅 Plugin）。

**问题**：
- **无 ProcessPlugin::execute（确认）**。
- ground_motion.rs **站点放大被用两次**：PGA 已放大，pgv_from_pga 又对已放大的 PGA 再乘 amp（High）。
- psha.rs psha_hazard_curve 把 1/rp 当 exceedance_probability 回显，而非计算值。
- 站点 PGA/config 字段部分为死配置。

---

## 14. geo-plugin-socioeconomic

**职责一句话**：社会经济——可达性、土地利用变化、人口。

**入口**：SocioeconomicPlugin（trait_impl.rs）**确认无 ProcessPlugin**（仅 Plugin）。

**问题**：
- **无 ProcessPlugin::execute（确认）**。
- accessibility.rs:127-129 multi_city_accessibility 每个像元内层循环重跑完整 Dijkstra -> O(n2*origins) 性能灾难。
- load(_path) stub；整个 config 死/未接线。

---

## 15. geo-plugin-volcanology

**职责一句话**：火山——火山灰扩散、危险区划、熔岩流。

**入口**：VolcanologyPlugin（trait_impl.rs）**确认无 ProcessPlugin**（仅 Plugin）。

**问题**：
- **无 ProcessPlugin::execute（确认）**。
- lava_flow.rs:182 cooling_time 单位 Pa·s/(m3/s) 荒谬、测试注释物理反向（High）。
- load stub + 死 config + 未用 _slope_degrees；网格模式输入未校验。

---

## 16. geo-plugin-ecology

**职责一句话**：生态——RUSLE/MUSLE 侵蚀、生态服务价值、栖息地、物种、LULC、SDR 泥沙。

**入口**：EcologyPlugin（ecology.rs，注意 trait_impl 就在 ecology.rs:402-449，无独立 trait_impl.rs 文件）；tools.rs 注册。

### ★ 关键问题：execute 硬编码常量栅格

`red = RasterBand::new("B4", 100, 100, vec![0.05; 10000], -999.0);` 与 `nir = RasterBand::new("B8", 100, 100, vec![0.50; 10000], ...)`（ecology.rs:422-423），且基线/评估两期共用同一 band（:430-431）。

- **execute 根本不解析 params 里的栅格**，基线=评估期常量栅格 -> 恢复评估结果与输入无关、恒为固定值。与 energy/coastal/forestry execute 会读 p[k] 数组形成鲜明对比，是 execute 路径真实 stub。

**其余**：
- **MUSLE 三元重复**：musle.rs（完整 MusleResult+assess_musle）、rusle/musle.rs:16 compute_musle_sediment（mm/mm/h 网格版）、sdr.rs:90 musle_event（m3/m3s 标量版）+ :106 musle_return_periods——三个文件各实现一套 11.8*(Q*qp)^0.56*K*LS*C*P，单位约定不同（m3·m3/s vs mm·mm/h）。musle.rs 未被 lib.rs 重导出。
- ecoservice.rs:32-47 carbon_sequestration_service 面积不守恒：只往目标类别加迁移量，不从未源类别减。
- RandomForest predict_one 短输入可 panic；usda_texture_class 除零+冗余分支。
- rusle/factors 等质量尚可，测试覆盖良好。
---

## 17. geo-plugin-survey

**职责一句话**：测绘——网格/断面/TIN 土方、导线平差、Gauss-Krüger/UTM/Vincenty/基准转换。

**入口**：SurveyPlugin（survey.rs:61）；trait_impl execute（trait_impl.rs:24-39）。

**问题**：
- **vincenty.rs:150-151 多余 .cos()**：cos2_sigma_m 已是 cos 再 .cos() 一次 -> 目标纬度被污染；测试容忍 ±3° 所以通过。
- **transform.rs:279-287 Helmert 通配零参数**：(CGCS2000, _)|(_, CGCS2000) -> 零平移，把 CGCS2000<->Xian80/Beijing54 当恒等（实有 ~20/160m 位移）；而 Molodensky 对这些对返回 None——同一基准对两表不一致（High）。
- survey.rs:74-108 grid_earthwork 忽略 _grid_cols/_grid_rows：任意 4x5/20 个值同体积，形状被丢弃。
- survey.rs:152-170 TIN 按输入顺序每 3 点成三角（非真 Delaunay）；:110-117 与 :152-170 的 .abs() 使挖/填符号丢失（净方量恒正）。
- survey.rs:173-223 adjusted_points 返回 [x, 0.0] 二维占位（实为一维平差）。
- survey.rs:226-239 + trait_impl.rs:29-39：assess/execute 只产出 earthwork，adjustment/cross_section 恒 None，通用 Process 路径对平差/断面不可达。
- gauss.rs:103-119 zone6 用 u16 未夹取（对比 zone3 有 1..=120 夹），负经度 as u16 回绕。
- gauss.rs:395-422 auto_detect_zone 最多跑 180 次 GK 逆算，先猜 3° 带歧义。
- utm.rs vs gauss.rs 同一 transverse-Mercator 级数跨模块复制（UTM 只 WGS84、GK 多椭球）。
- utm.rs:160-163 过期矛盾注释（zone 43 vs 48，测试对注错）。
- gauss.rs:77 椭球 label CGCS2000 (GRS80) 不准（CGCS2000 != GRS80）。
- tools.rs:1 拼写 -?Survey；survey tools.rs:40-67 椭球匹配块复制 3 遍。

---

## 18. geo-plugin-remote-sensing

**职责一句话**：遥感——辐射/TOA/DOS 大气校正、云掩膜、NDVI、简化 InSAR。

**入口**：RemoteSensingPlugin（trait_impl.rs:6）**确认无 ProcessPlugin**（仅 Plugin）。

**问题**：
- **无 ProcessPlugin::execute（确认）**——只能走 tools，Process 管线不能驱动。
- trait_impl.rs:15-17 load(_path) stub 忽略路径返回默认配置。
- insar.rs:24-51 coherence 是实值 Pearson 相关（Sum ms / sqrt(Sum m2 Sum s2)），非复相干幅度 <S1S2*> ——把输入当真值幅值，丢失相位，非真 InSAR 相干（High）。
- insar.rs:55-84 wrapped_phase 无 phase_diff 时 atan2((m-s)/(m+s),(m*s)/max) 非物理临时公式。
- insar.rs:129-150 所谓简化 Goldstein 枝切法实为行列向积分+相干阈值，非真支切；anomaly_count 重复计数。
- radiometric.rs:43 硬编码 esun 数组与波段数不匹配时静默回退 1500。
- radiometric.rs:61-65 _bias 未用参数贯穿公共链；:107-116 quick_atmospheric_correction 只 dos_correction 冗余包装（#[allow(unused_variables)]）。
- insar.rs:80 config unwrap_tolerance/phase_sigma 声明但从不使用。

---

# 问题总表（跨层精选）

| 等级 | 文件:行 | 问题 | 原因 |
|---|---|---|---|
| Critical | climate/kriging.rs:152-154 | cell_size==0 -> inf -> as usize 饱和 -> OOM 级巨量分配；bbox 非退化未校验 | 缺校验 |
| Critical | ecology/ecology.rs:422-431 | execute 硬编码常量栅格、基线=评估期 | execute 未解析任何栅格输入 |
| High | forestry/tools.rs:11 | assess_carbon_stock 实参 area/volume 互换，registry 工具碳值与 execute 不同 | tools 与 trait_impl 传参不一致 |
| High | coastal/trait_impl.rs:49 & tools.rs:11 | erosion_threshold_m 传进 sea_level_rise_m 槽；内部当绝对 DEM 高程阈值判淹没 | 位置参数语义错配 |
| High | climate/trait_impl.rs:123 | 未知命令用 Unimplemented 兜底：错误变体语义错配(应 Validation)+掩盖 simple_kriging/idf_return_period 已实现却无路由的缺口 | 防耗尽+漏配管线 |
| High | hydro/trait_impl.rs:20 | Plugin::new 无条件 panic!，消息引用不存在的 new(config,modflow)；通用加载器崩宿主 | 过期构造契约滥用 panic |
| High | agri/agri.rs:318-322 | Plugin::new 无条件 panic!（非测试生产代码） | trait 构造器 stub |
| High | geohazard/lib.rs:3-8 | info_value.rs 未声明 -> 信息量模型死代码且未编译 | 模块未接线 |
| High | geohazard/geohazard.rs:536 vs newmark.rs:8 | 两套不同 Newmark 公式 -> 同输入不同位移 | 重复且分歧 |
| High | seismology/ground_motion.rs | 站点放大被用两次（PGA 已放大再乘 amp） | 双重放大 |
| High | survey/vincenty.rs:150-151 | cos(cos2_sigma_m) 多余 cos 污染目标纬度 | 冗余 cos |
| High | survey/transform.rs:279-287 | Helmert 通配把 CGCS2000<->Xian80/54 当恒等，与 Molodensky 表不一致 | 宽 match 臂 |
| High | geomorph/river.rs:55-56,81-82,283 | (isize+D8_DR) as usize 负回绕 -> 边界 OOB；d8.rs 有检查 river.rs 没有 | 缺边界检查 |
| High | atmosphere/trait_impl.rs:1-42 | MISSING ProcessPlugin::execute（agri/survey 有）——Process 管线无法调度 | 缺 execute |
| High | remote-sensing/insar.rs:24-51 | coherence 实相关非复相干，phase 丢 | 输入当真值幅值 |
| High | volcanology/lava_flow.rs:182 | cooling_time 单位 Pa·s/(m3/s) 荒谬、测试注释物理反向 | 单位错乱 |
| Med | energy/energy.rs:90-95,141,209 | 坡度 flat 数组相邻跨行非二维；硬编码 10m+0.01ha | 无 cell_size/非 2D |
| Med | carbon/tools.rs:76 | vcs_additionality 恒传空 [] -> 恒 false 恒 0 永不过 | stub wiring |
| Med | carbon/carbon_sink.rs:100-142 | total 全局 mean、by_polygon 各自 ratio 之和不等；面积等分 | 聚合不自洽 |
| Med | ecology（musle.rs/rusle-musle.rs/sdr.rs） | MUSLE 三元重复、单位约定不同 | 三套复制 |
| Med | hydro/hydro.rs:376 | peak_Q*3600 当径流体积 -> 夸大淹没 | 峰流当体积 |
| Med | hydro（scs_cn/tr55/groundwater） | SCS-CN 三处实现；SCS-UH 两套 | 重复 |
| Med | coastal/storm_surge.rs:264 | n_pts-1 空数组下溢 -> panic/OOB | 缺判空 |
| Med | coastal/wave_runup.rs:302 | rc.max(0.0) 使淹没沙丘高流量兜底不可达 | 负 rc 提前丢弃 |
| Med | coastal/ocean.rs:125-128 | mhws/mlws/mhwn/mlwn 硬编码 0（文档称真实统计） | 占位 stub |
| Med | survey/survey.rs:74-108,152-170 | grid 忽略行列；TIN 输入序三角+.abs() 符号丢失 | 简化实现 |
| Med | survey/survey.rs:226-239 | assess/execute 只出 earthwork，平差/断面不可达 | 死分支 |
| Med | carbon/plume_ext.rs:115-148 | AOD->PM2.5 与 atmosphere 重复；魔法 *1000-2 | 跨 crate 重复+无来源常数 |
| Med | agri/tools.rs:69 | npp 日/季单位+÷120 颠倒 | 单位传导错 |
| Med | agri/agri.rs:240 dssat.rs:139 | acos() 未夹 -> 高纬冬季 NaN | 缺域夹取 |
| Med | socioeconomic/accessibility.rs:127 | 内层循环重跑 Dijkstra -> O(n2*origins) | 算法嵌套 |
| Med | urban/urban.rs:281 | assess 硬编码日照邻距 30m；NDVI 单输入塌缩类别 | 硬编码管线 |
| Med | groundwater/trait_impl.rs:39 | execute 只支持 recharge，其余全 Validation 拒绝 | stub 调度 |
| Med | groundwater/groundwater.rs:134 | _gamma=(-0.577...).ln() 数学无意义死代码 | 复制粘贴 |
| Med | climate/kriging.rs:221-223 | 奇异矩阵主元 continue -> 垃圾权重静默 | 无奇异性检测 |
| Med | climate/kriging.rs:188-259 | 每格点重解 kriging 矩阵 -> O(cells*n3) | 未 LU 分解复用 |
| Med | climate/idf.rs:72 | 跳过点后 nn 仍全长 -> 回归分母错 | nn 未用实点计数 |

---

# plugins 层最值得改的前 5

1. **forestry/tools.rs:11 实参错位（area<->volume）** —— 直接产生错误碳/蓄积结果且与 execute 分道，真正的数据正确性 bug，改动小收益大。
2. **hydro/trait_impl.rs:20 与 agri/agri.rs:318-322 的 Plugin::new panic** —— 生产代码 panic 会让任何通用插件加载器崩宿主，且 hydro 消息引用不存在的构造器；改成 Self::new(config) 即可。
3. **ecology/ecology.rs:422-431 execute 硬编码常量栅格** —— execute 路径完全失真，应像 energy/coastal/forestry 那样从 p[k] 解析栅格。
4. **climate/trait_impl.rs:123 Unimplemented->Validation + 补 simple_kriging/idf_return_period 管线** —— 先改错误变体语义，再补两条已实现函数的命令路由，消除被 Unimplemented 掩盖的功能缺口。
5. **geomorph/river.rs 边界负数->usize 回绕 OOB** —— 真实 DEM 边界单元可致越界读，潜在崩溃/UB；与 d8.rs 统一边界检查。

（并列候选：coastal erosion_threshold_m 语义错配；seismology 站点放大双重应用；survey vincenty 多余 cos；geohazard info_value 死模块。）

---

# 评审方法说明

核心文件（climate/trait_impl.rs 及其 Unimplemented 分支、forestry.rs、coastal/blue_carbon.rs、carbon/plugin.rs、hydro/trait_impl.rs+hydro.rs、ecology.rs execute、climate gcm/idf/kriging/drought、coastal.rs）由主评审逐行通读并核实证据；其余 14 个 crate 由 6 个并行只读子评审逐文件通读后合并，关键结论（ecology execute stub、hydro panic、info_value 死模块、MUSLE 三元重复、forestry tools 实参错位、coastal 语义错配等）已在主评审处二次核验。所有 file:line 均指向当前磁盘文件。
