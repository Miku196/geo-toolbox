use geo_core::errors::GeoResult;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Carbon emission calculation engine.
///
/// Delegates to [`geo_carbon_math::CarbonEngine`] for all computation.
#[wasm_bindgen]
pub struct CarbonEngine {
    inner: geo_carbon_math::CarbonEngine,
}

// ── Non-WASM methods (testable on native) ────────────────────────

impl CarbonEngine {
    fn calculate_inner(
        &self,
        geojson_str: &str,
        factors_csv: &str,
        year: u16,
    ) -> GeoResult<String> {
        let report = self
            .inner
            .calculate_from_geojson(geojson_str, factors_csv, year)
            .map_err(|e| geo_core::errors::GeoError::Other(e.to_string()))?;
        serde_json::to_string_pretty(&report).map_err(geo_core::errors::GeoError::Serde)
    }

    fn calculate_with_json_factors_inner(
        &self,
        geojson_str: &str,
        factors_json: &str,
        year: u16,
    ) -> GeoResult<String> {
        // Parse JSON factors and convert to CSV format for calculate_from_geojson
        let factors: Vec<serde_json::Value> = serde_json::from_str(factors_json).map_err(|e| {
            geo_core::errors::GeoError::Validation(format!("Invalid factors JSON: {e}"))
        })?;
        let mut csv = String::from("category,factor_value,source\n");
        for f in &factors {
            let cat = f["category"].as_str().unwrap_or("unknown");
            let val = f["factor_value"].as_f64().unwrap_or(0.0);
            let src = f["source"].as_str().unwrap_or("IPCC Tier 1");
            csv.push_str(&format!("{cat},{val},{src}\n"));
        }
        let report = self
            .inner
            .calculate_from_geojson(geojson_str, &csv, year)
            .map_err(geo_core::errors::GeoError::Other)?;
        serde_json::to_string_pretty(&report).map_err(geo_core::errors::GeoError::Serde)
    }

    fn calculate_with_factors_inner(
        &self,
        geojson_str: &str,
        year: u16,
        overrides_json: &str,
    ) -> GeoResult<String> {
        // IPCC Tier 1 默认排放因子 (tCO₂e/ha·yr)
        // 负值 = 碳汇 (吸收森林)
        const BASE_FACTORS: &[(&str, f64)] = &[
            ("forest", -5.0),
            ("grassland", 0.5),
            ("wetland", -0.3),
            ("cropland", 2.0),
            ("built_up", 1.0),
            ("water", 0.0),
            ("bare", 0.2),
        ];

        // 解析覆盖
        let overrides: HashMap<String, f64> =
            serde_json::from_str(overrides_json).map_err(|e| {
                geo_core::errors::GeoError::Validation(format!("overrides_json 解析失败: {e}"))
            })?;

        // 构建 CSV: 覆盖值替换基线值
        let mut csv = String::from("category,factor_value,source\n");
        for &(cat, val) in BASE_FACTORS {
            let final_val = overrides.get(cat).copied().unwrap_or(val);
            csv.push_str(&format!("{cat},{final_val},WASM-IPCC-Tier1\n"));
        }

        let report = self
            .inner
            .calculate_from_geojson(geojson_str, &csv, year)
            .map_err(geo_core::errors::GeoError::Other)?;
        serde_json::to_string_pretty(&report).map_err(geo_core::errors::GeoError::Serde)
    }
}

// ── WASM bindings ────────────────────────────────────────────────

#[wasm_bindgen]
impl CarbonEngine {
    /// Create a new carbon engine.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: geo_carbon_math::CarbonEngine::new(),
        }
    }

    /// Calculate carbon emissions from GeoJSON features and emission factors.
    ///
    /// **⚠️ 重要**: 输入的 GeoJSON 必须使用投影坐标系 (Projected CRS),
    /// 如 EPSG:3857 (Web Mercator) 或 UTM。经纬度坐标 (EPSG:4326)
    /// 将导致面积计算严重误差 — geo_carbon_math 的面积算法在纬度 >40° 时
    /// 可达 ±15%。请在调用前用 proj4js / turf.js 将 WGS84 坐标投影到 UTM。
    ///
    /// ## Parameters
    ///
    /// - `geojson_str`: GeoJSON FeatureCollection string with Polygon/MultiPolygon features.
    ///   Each feature must have `properties.class` (string) indicating landcover type.
    /// - `factors_csv`: CSV string with columns: `category,factor_value[,source]`.
    /// - `year`: Target year for the calculation.
    ///
    /// ## Returns
    ///
    /// JSON string representing [`geo_carbon_math::CarbonReport`].
    #[wasm_bindgen(js_name = calculate)]
    pub fn calculate(
        &self,
        geojson_str: &str,
        factors_csv: &str,
        year: u16,
    ) -> Result<String, JsValue> {
        self.calculate_inner(geojson_str, factors_csv, year)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate with JSON factors (alternative to CSV).
    ///
    /// - `factors_json`: JSON array of `{category, factor_value, source?}` objects.
    #[wasm_bindgen(js_name = calculateWithJsonFactors)]
    pub fn calculate_with_json_factors(
        &self,
        geojson_str: &str,
        factors_json: &str,
        year: u16,
    ) -> Result<String, JsValue> {
        self.calculate_with_json_factors_inner(geojson_str, factors_json, year)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate with runtime factor overrides.
    ///
    /// **⚠️ 重要**: 与 `calculate` 相同的投影坐标系警告适用。
    ///
    /// 以 IPCC Tier 1 默认因子为基线, `overrides_json` 中指定的类别
    /// 覆盖基线值, 未指定的类别保持默认。
    ///
    /// ## Parameters
    ///
    /// - `geojson_str`: GeoJSON FeatureCollection (投影坐标系)
    /// - `year`: 核算年份
    /// - `overrides_json`: 因子覆盖 JSON 对象, e.g.
    ///   `{"forest": -5.5, "grassland": 0.8}`
    ///   空对象 `{}` 等价于仅使用默认因子。
    ///
    /// ## Returns
    ///
    /// JSON string representing [`geo_carbon_math::CarbonReport`].
    ///
    /// ## JS Example
    ///
    /// ```javascript
    /// const engine = new CarbonEngine();
    /// const result = engine.calculateWithFactors(geojson, 2025, '{"forest": -7.0}');
    /// ```
    #[wasm_bindgen(js_name = calculateWithFactors)]
    pub fn calculate_with_factors(
        &self,
        geojson_str: &str,
        year: u16,
        overrides_json: &str,
    ) -> Result<String, JsValue> {
        self.calculate_with_factors_inner(geojson_str, year, overrides_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Default for CarbonEngine {
    fn default() -> Self {
        Self::new()
    }
}
