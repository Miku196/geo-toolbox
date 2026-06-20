use geo_carbon_math::{EmissionFactor, EmissionScope, GasFactor, GreenhouseGas};

/// China-specific emission factors.
///
/// Sources:
/// - Grid emission factors: 全国电网排放因子 (2023), Ministry of Ecology and Environment
/// - Provincial parameters: 省级温室气体清单编制指南
pub struct ChinaEfDb;

impl ChinaEfDb {
    // ── Grid Emission Factors (tCO₂/MWh) ──

    /// 全国电网平均排放因子 2023 (tCO₂/MWh)
    pub const CN_NATIONAL_2023: f64 = 0.5703;

    /// 华北区域电网 2023 (tCO₂/MWh) — 京津冀/晋/蒙
    pub const CN_NORTH_2023: f64 = 0.7204;
    /// 东北区域 2023 — 辽/吉/黑
    pub const CN_NORTHEAST_2023: f64 = 0.6780;
    /// 华东区域 2023 — 沪/苏/浙/皖/闽
    pub const CN_EAST_2023: f64 = 0.5850;
    /// 华中区域 2023 — 豫/鄂/湘/赣/川/渝
    pub const CN_CENTRAL_2023: f64 = 0.4803;
    /// 南方区域 2023 — 粤/桂/滇/黔/琼
    pub const CN_SOUTH_2023: f64 = 0.3907;
    /// 西北区域 2023 — 陕/甘/青/宁/新
    pub const CN_NORTHWEST_2023: f64 = 0.5012;
    /// 西藏（独立电网）2023
    pub const CN_TIBET_2023: f64 = 0.1500;

    /// Historical grid emission factors for trend analysis.
    pub const CN_NATIONAL_2020: f64 = 0.6101;
    pub const CN_NATIONAL_2015: f64 = 0.6950;
    pub const CN_NATIONAL_2010: f64 = 0.8130;

    /// Get China grid emission factor by region name (case-insensitive).
    pub fn grid_for_region(region: &str, year: u16) -> f64 {
        let regional = match region.to_lowercase().as_str() {
            // 华北
            "north" | "华北" | "beijing" | "北京" | "tianjin" | "天津" | "hebei" | "河北"
            | "shanxi" | "山西" | "innermongolia" | "内蒙古" => Self::CN_NORTH_2023,
            // 东北
            "northeast" | "东北" | "liaoning" | "辽宁" | "jilin" | "吉林" | "heilongjiang"
            | "黑龙江" => Self::CN_NORTHEAST_2023,
            // 华东
            "east" | "华东" | "shanghai" | "上海" | "jiangsu" | "江苏" | "zhejiang" | "浙江"
            | "anhui" | "安徽" | "fujian" | "福建" => Self::CN_EAST_2023,
            // 华中
            "central" | "华中" | "henan" | "河南" | "hubei" | "湖北" | "hunan" | "湖南"
            | "jiangxi" | "江西" | "sichuan" | "四川" | "chongqing" | "重庆" => {
                Self::CN_CENTRAL_2023
            }
            // 南方
            "south" | "南方" | "guangdong" | "广东" | "guangxi" | "广西" | "yunnan" | "云南"
            | "guizhou" | "贵州" | "hainan" | "海南" => Self::CN_SOUTH_2023,
            // 西北
            "northwest" | "西北" | "shaanxi" | "陕西" | "gansu" | "甘肃" | "qinghai" | "青海"
            | "ningxia" | "宁夏" | "xinjiang" | "新疆" => Self::CN_NORTHWEST_2023,
            // 西藏
            "tibet" | "西藏" => Self::CN_TIBET_2023,
            _ => Self::CN_NATIONAL_2023,
        };
        // Year-based adjustment: older years use historical factors
        if year <= 2010 {
            Self::CN_NATIONAL_2010
        } else if year <= 2015 {
            Self::CN_NATIONAL_2015
        } else if year <= 2020 {
            Self::CN_NATIONAL_2020
        } else {
            regional
        }
    }

    /// Full EmissionFactor for China grid electricity consumption.
    pub fn china_grid_electricity(kwh: f64, province: &str, year: u16) -> EmissionFactor {
        let ef_tco2_per_mwh = Self::grid_for_region(province, year);
        let ef_tco2_per_kwh = ef_tco2_per_mwh / 1000.0;
        let total = kwh * ef_tco2_per_kwh;

        EmissionFactor {
            category: "electricity".into(),
            subcategory: Some("china_grid".into()),
            source: "MEE_2023".into(),
            region: Some(province.into()),
            factor_value: total,
            unit: "tCO₂".into(),
            valid_from_year: year as i32,
            valid_to_year: None,
            gas_factors: vec![GasFactor::land_use(GreenhouseGas::CO2, total, "tCO₂")],
            uncertainty_pct: Some(15.0),
            scope: Some(EmissionScope::Scope2),
            activity_type: Some("purchased_electricity".into()),
            fuel_type: None,
            ncv_override: None,
            cc_override: None,
            ox_override: None,
            grid_ef: Some(ef_tco2_per_mwh),
        }
    }

    // ── China-specific fuel parameters ──

    /// Default net calorific value for Chinese raw coal (GJ/t).
    /// Source: 中国能源统计年鉴 2023.
    pub const CN_RAW_COAL_NCV: f64 = 20.908; // GJ/t
    /// Default carbon content for Chinese raw coal (tC/GJ).
    pub const CN_RAW_COAL_CC: f64 = 0.026;
    /// Default oxidation rate.
    pub const CN_RAW_COAL_OX: f64 = 0.95;

    /// Default diesel NCV for China (GJ/t).
    pub const CN_DIESEL_NCV: f64 = 42.652;
    /// Default carbon content for Chinese diesel (tC/GJ).
    pub const CN_DIESEL_CC: f64 = 0.020;

    /// Default gasoline NCV for China (GJ/t).
    pub const CN_GASOLINE_NCV: f64 = 43.070;
    pub const CN_GASOLINE_CC: f64 = 0.019;

    // ── Cement clinker EF for China ──

    /// China average cement clinker EF (tCO₂/t clinker).
    /// Source: 中国水泥协会 2023 — slightly lower than global due to alternative raw materials.
    pub const CN_CEMENT_CLINKER: f64 = 0.490;

    // ── Provincial land-use carbon density factors (tCO₂e/ha/yr) ──

    /// Forest carbon sink by province (tCO₂e/ha/yr).
    /// Source: 第九次全国森林资源清查 (2014-2018).
    const CN_FOREST_BY_PROVINCE: &[(&str, f64)] = &[
        ("北京", -6.2),
        ("天津", -5.8),
        ("河北", -5.0),
        ("山西", -4.2),
        ("内蒙古", -4.5),
        ("辽宁", -5.5),
        ("吉林", -6.0),
        ("黑龙江", -6.8),
        ("上海", -5.0),
        ("江苏", -5.2),
        ("浙江", -6.5),
        ("安徽", -5.8),
        ("福建", -8.0),
        ("江西", -7.5),
        ("山东", -4.8),
        ("河南", -5.0),
        ("湖北", -6.5),
        ("湖南", -7.5),
        ("广东", -7.0),
        ("广西", -8.5),
        ("海南", -9.0),
        ("重庆", -6.0),
        ("四川", -6.8),
        ("贵州", -7.0),
        ("云南", -8.5),
        ("西藏", -3.0),
        ("陕西", -5.0),
        ("甘肃", -3.5),
        ("青海", -2.5),
        ("宁夏", -3.0),
        ("新疆", -4.0),
    ];

    /// Get province-specific forest carbon sink factor (tCO₂e/ha/yr).
    /// Falls back to IPCC default −5.0 if province not found.
    pub fn forest_sink_for_province(province: &str) -> f64 {
        Self::CN_FOREST_BY_PROVINCE
            .iter()
            .find(|(p, _)| p == &province)
            .map(|(_, v)| *v)
            .unwrap_or(-5.0)
    }

    /// Build full EmissionFactor for China forest carbon sink.
    pub fn china_forest_sink(area_ha: f64, province: &str, year: u16) -> EmissionFactor {
        let ef_per_ha = Self::forest_sink_for_province(province);
        let total = ef_per_ha * area_ha;
        EmissionFactor {
            category: "land_use".into(),
            subcategory: Some("forest_sink_china".into()),
            source: "NFI_2019".into(),
            region: Some(province.into()),
            factor_value: total,
            unit: "tCO₂e".into(),
            valid_from_year: year as i32,
            valid_to_year: None,
            gas_factors: vec![GasFactor::land_use(GreenhouseGas::CO2, total, "tCO₂e")],
            uncertainty_pct: Some(25.0),
            scope: Some(EmissionScope::Scope1),
            activity_type: Some("forest_management".into()),
            fuel_type: None,
            ncv_override: None,
            cc_override: None,
            ox_override: None,
            grid_ef: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_national() {
        let ef = ChinaEfDb::grid_for_region("national", 2023);
        assert!((ef - 0.5703).abs() < 1e-4);
    }

    #[test]
    fn test_grid_province() {
        let ef = ChinaEfDb::grid_for_region("广东", 2023);
        assert!((ef - 0.3907).abs() < 1e-4);
    }

    #[test]
    fn test_grid_english_region() {
        let ef = ChinaEfDb::grid_for_region("sichuan", 2023);
        assert!((ef - 0.4803).abs() < 1e-4);
    }

    #[test]
    fn test_grid_historical() {
        let ef_2010 = ChinaEfDb::grid_for_region("national", 2010);
        assert!((ef_2010 - 0.8130).abs() < 1e-4);
        let ef_2023 = ChinaEfDb::grid_for_region("national", 2023);
        assert!(ef_2023 < ef_2010, "Grid should decarbonise over time");
    }

    #[test]
    fn test_china_electricity() {
        let ef = ChinaEfDb::china_grid_electricity(1000.0, "广东", 2023);
        assert_eq!(ef.scope, Some(EmissionScope::Scope2));
        assert!(ef.factor_value > 0.0);
    }

    #[test]
    fn test_forest_sink_by_province() {
        let fujian = ChinaEfDb::forest_sink_for_province("福建");
        assert!((fujian - (-8.0)).abs() < 1e-6);
        let unknown = ChinaEfDb::forest_sink_for_province("Nonexistent");
        assert!((unknown - (-5.0)).abs() < 1e-6, "Should fallback to IPCC");
    }
}
