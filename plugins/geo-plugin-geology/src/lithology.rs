use serde::{Deserialize, Serialize};

/// 岩性类型。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LithologyClass {
    /// 冲积层 / 松散沉积
    Alluvium,
    /// 粘土
    Clay,
    /// 砂岩
    Sandstone,
    /// 粉砂岩
    Siltstone,
    /// 石灰岩
    Limestone,
    /// 白云岩
    Dolomite,
    /// 页岩
    Shale,
    /// 花岗岩
    Granite,
    /// 玄武岩
    Basalt,
    /// 片麻岩
    Gneiss,
    /// 片岩
    Schist,
    /// 大理岩
    Marble,
    /// 石英岩
    Quartzite,
    /// 未分类
    Unknown,
}

impl LithologyClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Alluvium => "alluvium",
            Self::Clay => "clay",
            Self::Sandstone => "sandstone",
            Self::Siltstone => "siltstone",
            Self::Limestone => "limestone",
            Self::Dolomite => "dolomite",
            Self::Shale => "shale",
            Self::Granite => "granite",
            Self::Basalt => "basalt",
            Self::Gneiss => "gneiss",
            Self::Schist => "schist",
            Self::Marble => "marble",
            Self::Quartzite => "quartzite",
            Self::Unknown => "unknown",
        }
    }
}

/// 岩性分类结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LithologyResult {
    pub class: String,
    pub density_kgm3: f64,
    pub cohesion_kpa: f64,
    pub friction_angle_deg: f64,
    pub description: String,
}

/// 从地质图编码推断岩性。
pub fn lithology_from_code(code: &str) -> LithologyClass {
    match code {
        "Q" | "Qal" => LithologyClass::Alluvium,
        "Qh" | "Qp" => LithologyClass::Clay,
        "K" | "Ss" => LithologyClass::Sandstone,
        "Si" => LithologyClass::Siltstone,
        "C" | "Lm" => LithologyClass::Limestone,
        "D" | "Dl" => LithologyClass::Dolomite,
        "S" | "Sh" => LithologyClass::Shale,
        "γ" | "Gr" => LithologyClass::Granite,
        "β" | "Ba" => LithologyClass::Basalt,
        "Gn" => LithologyClass::Gneiss,
        "Sc" => LithologyClass::Schist,
        "Mb" => LithologyClass::Marble,
        "Qz" => LithologyClass::Quartzite,
        _ => LithologyClass::Unknown,
    }
}

/// 综合岩性分类 (如含风化程度等)。
pub fn classify_lithology(codes: &[String]) -> Vec<LithologyResult> {
    codes
        .iter()
        .map(|c| {
            let lc = lithology_from_code(c);
            let (density, cohesion, friction) = engineering_parameters(&lc);
            LithologyResult {
                class: lc.as_str().into(),
                density_kgm3: density,
                cohesion_kpa: cohesion,
                friction_angle_deg: friction,
                description: format!("{} (code: {})", lc.as_str(), c),
            }
        })
        .collect()
}

/// 岩性工程参数 (密度 kPa, 内聚力 kPa, 内摩擦角 deg)。
pub fn engineering_parameters(lc: &LithologyClass) -> (f64, f64, f64) {
    match lc {
        LithologyClass::Alluvium => (1800.0, 0.0, 28.0),
        LithologyClass::Clay => (1900.0, 20.0, 15.0),
        LithologyClass::Sandstone => (2400.0, 200.0, 35.0),
        LithologyClass::Siltstone => (2300.0, 150.0, 30.0),
        LithologyClass::Limestone => (2600.0, 500.0, 40.0),
        LithologyClass::Dolomite => (2700.0, 400.0, 38.0),
        LithologyClass::Shale => (2200.0, 100.0, 25.0),
        LithologyClass::Granite => (2650.0, 1000.0, 45.0),
        LithologyClass::Basalt => (2800.0, 800.0, 42.0),
        LithologyClass::Gneiss => (2700.0, 600.0, 40.0),
        LithologyClass::Schist => (2600.0, 300.0, 35.0),
        LithologyClass::Marble => (2700.0, 500.0, 38.0),
        LithologyClass::Quartzite => (2650.0, 700.0, 42.0),
        LithologyClass::Unknown => (2500.0, 100.0, 30.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lithology_code() {
        assert_eq!(lithology_from_code("Q"), LithologyClass::Alluvium);
        assert_eq!(lithology_from_code("γ"), LithologyClass::Granite);
        assert_eq!(lithology_from_code("Lm"), LithologyClass::Limestone);
        assert_eq!(lithology_from_code("NOT_A_CODE"), LithologyClass::Unknown);
    }

    #[test]
    fn test_classify_multiple() {
        let codes = vec!["γ".into(), "Q".into(), "Ss".into()];
        let results = classify_lithology(&codes);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].class, "granite");
        assert_eq!(results[1].class, "alluvium");
        assert_eq!(results[2].class, "sandstone");
    }

    #[test]
    fn test_engineering_params() {
        let (d, c, f) = engineering_parameters(&LithologyClass::Granite);
        assert!((d - 2650.0).abs() < 0.01);
        assert!((c - 1000.0).abs() < 0.01);
        assert!((f - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_lithology_str() {
        assert_eq!(LithologyClass::Limestone.as_str(), "limestone");
        assert_eq!(LithologyClass::Unknown.as_str(), "unknown");
    }
}
