# 中国自然灾害风险评估 — geo-toolbox 实战案例

使用 geo-toolbox + USGS + Natural Earth + Python GIS 构建的两条完整空间分析管线。

---

## 快速开始

```bash
cd examples/china-risk-assessment
pip install geopandas cartopy reportlab pyproj matplotlib

# 洪水高风险区评估
python flood_risk_pipeline.py

# 地震活动评估
python earthquake_pipeline.py

# 自包含 HTML 报告（地图 base64 内嵌 + 交互式 CRS 变换）
python self_contained_report.py
```

产物输出到 `./output/`。

---

## 文件结构

```
examples/china-risk-assessment/
├── README.md                   # 本文件
├── flood_risk_pipeline.py      # 洪水管线 (50KB)
├── earthquake_pipeline.py      # 地震管线 (38KB)
├── self_contained_report.py    # HTML 报告生成器
├── data/                       # 矢量数据
│   ├── ne_10m_admin_0_countries/  # Natural Earth 行政边界
│   ├── ne_10m_rivers/             # 全球河流中心线
│   ├── ne_10m_lakes/              # 全球湖泊面
│   ├── usgs_china_2026.geojson    # M3+ 地震事件 (440 条)
│   └── usgs_china_2026_m4.geojson # M4+ 地震事件 (434 条)
└── output/                     # 运行产物
    ├── china_flood_risk_2026.png
    ├── china_flood_risk_2026_regions.png
    ├── china_flood_risk_2026_stats.png
    ├── china_flood_high_risk_zones_2026.geojson
    ├── china_flood_risk_all_2026.geojson
    ├── 中国2026年洪水高风险区评估报告.pdf
    ├── 中国2026年洪水高风险区评估报告.html
    ├── china_seismic_2026.png
    ├── china_seismic_2026_regions.png
    ├── china_seismic_2026_stats.png
    ├── china_seismic_high_risk_2026.geojson
    ├── 中国2026年地震活动评估报告.pdf
    └── 2026年中国地震活动评估报告.html
```

---

## geo-toolbox 调用详情

每条管线运行时，geo-toolbox 执行为：

### 洪水管线

```
crs list                                    # 验证 7 个坐标系
crs transform --from 4326 --to 3857 ...     # 北京/上海/广州/成都/武汉/西安/哈尔滨
crs transform --from 4326 --to 32649 ...    # 成都 → UTM 49N
crs transform --from 4326 --to 3405  ...    # 成都 → 等积投影
                                             # 共 21 次坐标变换
```

### 地震管线

```
crs list                                    # 验证坐标系
crs transform --from 4326 --to 3857 ...     # 最大震级事件 3 向
crs transform --from 4326 --to 32650 ...
crs transform --from 4326 --to 3405  ...
crs transform --from 4326 --to 3857 ...     # Top10 地震批量变换
                                             # 共 30 次坐标变换
output geojson --from-file ...              # GeoJSON 压缩
```

> 当前 geo-toolbox 编译未启用 `proj` feature，变换由 pyproj fallback。启用后直接走 Rust PROJ FFI。

---

## 洪水风险模型

0.25°×0.25° 格网，覆盖中国全境 15,228 个有效格网。

| 风险因子 | 权重 | 来源 |
|----------|------|------|
| 河流缓冲区 (0-10km) | 0.30 | Natural Earth 10m |
| 河流缓冲区 (10-30km) | 0.20 | Natural Earth 10m |
| 历史洪水易发区 | 0.25 | DFO 1985-2025 |
| 2026 气候预估 | 0.15 | CMIP6 SSP5-8.5 |
| 地形海岸效应 | 0.10 | SRTM |

## 地震风险模型

识别 13 条中国主要地震带 (GB 18306-2015)，结合 440 次 M3+ 事件进行空间核密度估计。

| 风险因子 | 权重 | 来源 |
|----------|------|------|
| 地震带 PGA | ~0.45 | GB 18306-2015 |
| 事件密度 | ~0.40 | USGS 2026 |
| 最大震级 | ~0.15 | USGS 2026 |

---

## 评估结果

### 洪水 (2026)

| 风险等级 | 面积(万 km²) | 占比 | 主要分布 |
|----------|:----------:|:----:|----------|
| 极高风险 | 85.0 | 8.7% | 长江中下游、珠江三角洲、洞庭-鄱阳湖 |
| 高风险 | 133.0 | 13.7% | 淮河、海河、四川盆地、东南沿海 |
| 中风险 | 40.1 | 4.1% | 黄河下游、松花江、辽河 |
| 低/极低 | 714.7 | 73.5% | 青藏高原、西北干旱区 |

### 地震 (2026 Jan-Jun)

| 风险等级 | 面积(万 km²) | 占比 | 主要分布 |
|----------|:----------:|:----:|----------|
| 极高风险 | 71.6 | 7.4% | 南北地震带、天山、台湾 |
| 高风险 | 257.2 | 26.4% | 华北平原、汾渭、阿尔金-祁连 |
| 中风险 | 152.9 | 15.7% | 东南沿海、东北 |
| 低/极低 | 491.1 | 50.5% | 其余区域 |

M3+ 事件 440 次 | M4+ 事件 434 次 | 最大震级 M5.9 | 平均深度 30km

---

## 数据来源

| 数据 | 来源 | 许可 |
|------|------|------|
| 行政边界 | [Natural Earth 10m](https://www.naturalearthdata.com/) | Public Domain |
| 河流/湖泊 | [Natural Earth 10m](https://www.naturalearthdata.com/) | Public Domain |
| 地震目录 | [USGS Earthquake Catalog](https://earthquake.usgs.gov/fdsnws/event/1/) | Public Domain |
| 地震带 | GB 18306-2015 中国地震动参数区划图 | 国家标准 |
| 气候预估 | CMIP6 SSP5-8.5 | Open Access |

---

## 技术栈

| 层 | 工具 | 用途 |
|----|------|------|
| 坐标变换 | geo-toolbox (Rust) | CRS list + transform |
| 空间分析 | geopandas + shapely | 格网模型、缓冲区 |
| 制图 | cartopy + matplotlib | GIS 专题图（微软雅黑） |
| 报告 | reportlab | PDF 中文报告 |
| 交互 | 纯 JS CRS 变换 | 自包含 HTML 内嵌 |
