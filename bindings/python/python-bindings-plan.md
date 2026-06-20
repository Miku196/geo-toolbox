# Python Bindings 实施计划

> 基于 WASM crate (crates/geo-wasm) 的 API 镜像，用 PyO3 暴露给 Python。
> 目标：`pip install geo-toolbox` → Python 直接调用 Rust 引擎。

## Phase 1: 基础功能 (当前)

### 1.1 扩展 Cargo.toml 依赖
- `geo-carbon-math` — 碳核算引擎
- `geo-io` — 格式解析 (CSV/NMEA)
- `geo-vector` — 矢量运算
- `geo-stats` — 空间统计
- `geo-index` — Geohash 编码
- `geo-report` — 报告生成
- `geo-emission-factors` — 排放因子数据
- `proj` (optional) — CRS 变换
- `chrono` — 时间处理

### 1.2 CRS 变换 (`mod crs`)
- `CrsEngine` class: `list_all()` → list of CRS defs
- `CrsEngine.transform(from_epsg, to_epsg, x, y)` → (x, y)
- `CrsEngine.transform_batch(from_epsg, to_epsg, coords)` → list of (x, y)

### 1.3 空间运算 (`mod spatial`)
- `compute_area_sqm(geojson_geom)` → float
- `compute_bbox(geojson_geom)` → (west, south, east, north)
- `compute_centroid(geojson_geom)` → (x, y)
- `simplify_geometry(geojson_geom, epsilon)` → geojson_geom
- `convex_hull(geojson_geom)` → geojson_geom

### 1.4 输入输出 (`mod io`)
- `parse_csv_to_json(csv_text)` → list[dict]
- `generate_geojson(features)` → str
- `generate_excel(columns, rows, sheet_name)` → bytes
- `parse_nmea(sentence)` → dict
- `parse_nmea_batch(sentences)` → list[dict]

### 1.5 Geohash (`mod geohash`)
- `geohash_encode(lat, lon, precision)` → str
- `geohash_decode(hash)` → (lat_min, lat_max, lon_min, lon_max)
- `geohash_neighbors(hash)` → list[str]

## Phase 2: 碳核算

### 2.1 碳核算引擎 (`mod carbon`)
- `CarbonEngine.calculate(geojson_fc, factors_csv, year)` → dict (CarbonReport)
- `CarbonEngine.calculate_with_json_factors(geojson_fc, factors, year)` → dict
- `generate_carbon_report_md(report, aoi_name, auditor)` → str

### 2.2 排放因子 (`mod emission_factors`)
- `list_emission_factors()` → list[dict]
- `lookup_factor(region, category)` → dict

## Phase 3: 矢量分析 + 统计

### 3.1 矢量 (`mod vector`)
- `vector_buffer(geojson, distance_m)` → geojson
- `vector_intersect(geojson_a, geojson_b)` → geojson
- `vector_clip(geojson, clip_geom)` → geojson
- `vector_simplify(geojson, epsilon)` → geojson

### 3.2 统计 (`mod stats`)
- `zonal_stats(raster_data, zones, stats_list)` → dict
- `morans_i(values, weights)` → float

## Phase 4: 栅格 + 时序

### 4.1 栅格 (`mod raster`)
- `compute_ndvi(nir_band, red_band)` → ndvi_band
- `terrain_slope(dem_data)` → slope_data
- `terrain_aspect(dem_data)` → aspect_data

### 4.2 时序 (`mod temporal`)
- `mann_kendall(values)` → (trend, p_value)
- `sens_slope(values)` → float

---

## Python 类型标注 (py.typed + __init__.pyi)

```python
# geo_toolbox/__init__.pyi

from typing import Optional, TypedDict

class CrsDef(TypedDict):
    epsg: int
    name: str
    proj4: str
    unit: str

class CarbonReport(TypedDict):
    total_tco2e: float
    by_scope: dict[str, float]
    by_category: dict[str, float]
    by_landcover: dict[str, float]

class EmissionFactor(TypedDict):
    gas: str
    factor: float
    unit: str
    scope: str
    gwp: float

class CrsEngine:
    def list_all(self) -> list[CrsDef]: ...
    def transform(self, from_epsg: int, to_epsg: int, x: float, y: float) -> tuple[float, float]: ...
    def transform_batch(self, from_epsg: int, to_epsg: int, coords: list[float]) -> list[float]: ...

class CarbonEngine:
    def calculate(self, geojson: str | dict, factors_csv: str, year: int) -> CarbonReport: ...
    def calculate_with_json_factors(self, geojson: str | dict, factors: list[EmissionFactor], year: int) -> CarbonReport: ...

def compute_area_sqm(geojson_geom: str | dict) -> float: ...
def compute_bbox(geojson_geom: str | dict) -> tuple[float, float, float, float]: ...
def compute_centroid(geojson_geom: str | dict) -> tuple[float, float]: ...
def simplify_geometry(geojson_geom: str | dict, epsilon: float) -> dict: ...
def convex_hull(geojson_geom: str | dict) -> dict: ...
def parse_csv_to_json(csv_text: str) -> list[dict[str, object]]: ...
def generate_geojson(features: list[dict]) -> str: ...
def generate_excel(columns: list[str], rows: list[list[object]], sheet_name: str = ...) -> bytes: ...
def generate_carbon_report_md(report: CarbonReport, aoi_name: str, auditor: str) -> str: ...
def parse_nmea(sentence: str) -> dict: ...
def parse_nmea_batch(sentences: list[str]) -> list[dict]: ...
def geohash_encode(lat: float, lon: float, precision: int = ...) -> str: ...
def geohash_decode(hash: str) -> tuple[float, float, float, float]: ...
def geohash_neighbors(hash: str) -> list[str]: ...
def __version__() -> str: ...
```

## 测试策略

- 每个模块一个 `tests/test_*.py` 文件
- 使用 `pytest` + 固定测试数据
- CI: `maturin develop && pytest tests/`

## 构建打包

```bash
cd bindings/python
maturin build --release  # → .whl in target/wheels/
maturin publish           # → PyPI
```
