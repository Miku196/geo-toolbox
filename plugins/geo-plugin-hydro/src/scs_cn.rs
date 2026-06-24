//! SCS-CN — 径流曲线数法。
//!
//! 美国农业部 SCS (NRCS) 径流曲线数法估算降雨-径流。
//!
//! 核心公式：
//! `Q = (P - Ia)² / (P - Ia + S)`
//! `Ia = 0.2 × S`
//! `S = 25400/CN - 254`
//!
//! 其中 Q = 径流深 (mm), P = 降雨量 (mm), CN = 曲线数 (0-100)。

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// 水文土壤分组
// ──────────────────────────────────────────────

/// 水文土壤分组（HSG）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoilGroup {
    /// A — 低径流潜力（砂土、砾石，高入渗率）
    A,
    /// B — 中等入渗率（粉砂壤土）
    B,
    /// C — 缓慢入渗率（粘壤土）
    C,
    /// D — 高径流潜力（粘土，低入渗率）
    D,
}

impl SoilGroup {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().chars().next() {
            Some('a') => Self::A,
            Some('b') => Self::B,
            Some('c') => Self::C,
            Some('d') => Self::D,
            _ => Self::B, // 默认 B
        }
    }
}

// ──────────────────────────────────────────────
// 前期土壤湿度条件
// ──────────────────────────────────────────────

/// 前期土壤湿度条件（AMC）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AMC {
    /// AMC I — 干旱（前 5 天降雨 < 12.7mm）
    Dry,
    /// AMC II — 正常
    Normal,
    /// AMC III — 湿润（前 5 天降雨 > 27.9mm）
    Wet,
}

// ──────────────────────────────────────────────
// 径流评估结果
// ──────────────────────────────────────────────

/// SCS-CN 径流评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScsCnAssessment {
    /// CN 值（均值）
    pub cn_mean: f64,
    /// S 值 (mm)
    pub s_mm: f64,
    /// Ia 初损 (mm)
    pub ia_mm: f64,
    /// 总降雨量 (mm)
    pub rainfall_mm: f64,
    /// 净雨量 / 径流深 (mm)
    pub runoff_mm: f64,
    /// 径流系数
    pub runoff_coefficient: f64,
    /// 评估面积 (ha)
    pub area_ha: f64,
    /// 径流总量 (m³)
    pub total_runoff_volume_m3: f64,
    /// 各像素径流深 (mm)
    pub runoff_grid: Vec<f64>,
}

// ──────────────────────────────────────────────
// CN 查找表
// ──────────────────────────────────────────────

/// 典型土地利用的 CN-II 值（AMC II）。
///
/// 来源：NRCS TR-55 (1986)。
/// 值对应四种水文土壤分组 [A, B, C, D]。
fn cn_ii_for_landuse(landuse: &str) -> [f64; 4] {
    match landuse {
        // ── 城市区 ──
        "urban" | "商业" | "商业区" => [89.0, 92.0, 94.0, 95.0],
        "industrial" | "工业" | "工业区" => [81.0, 88.0, 91.0, 93.0],
        "residential_1_acre" | "低密度住宅" => [54.0, 70.0, 80.0, 85.0],
        "residential_1_2_acre" | "中密度住宅" => [61.0, 75.0, 83.0, 87.0],
        "residential_1_4_acre" | "高密度住宅" => [67.0, 80.0, 87.0, 91.0],
        "paved" | "铺装面" | "道路" => [98.0, 98.0, 98.0, 98.0],

        // ── 农业 / 耕地 ──
        "row_crops_straight" | "直行作物" => [67.0, 78.0, 85.0, 89.0],
        "row_crops_contoured" | "等高耕作" => [65.0, 76.0, 84.0, 88.0],
        "row_crops_terrace" | "梯田作物" => [63.0, 74.0, 82.0, 87.0],
        "small_grains_straight" | "直行细粮" => [65.0, 76.0, 84.0, 88.0],
        "small_grains_contoured" | "等高细粮" => [63.0, 75.0, 83.0, 87.0],
        "pasture_poor" | "劣质牧草" => [68.0, 79.0, 86.0, 89.0],
        "pasture_fair" | "一般牧草" => [49.0, 69.0, 79.0, 84.0],
        "pasture_good" | "优质牧草" => [39.0, 61.0, 74.0, 80.0],
        "cropland" | "耕地" | "农田" => [67.0, 78.0, 85.0, 89.0],

        // ── 林地 ──
        "forest_poor" | "疏林" | "劣质林地" => [45.0, 66.0, 77.0, 83.0],
        "forest_fair" | "一般林地" => [36.0, 60.0, 73.0, 79.0],
        "forest_good" | "森林" | "密林" | "优质林地" => [30.0, 55.0, 70.0, 77.0],
        "shrub" | "灌木" => [35.0, 56.0, 71.0, 78.0],

        // ── 水体 / 湿地 ──
        "water" | "水体" | "水域" => [100.0, 100.0, 100.0, 100.0],
        "wetland" | "湿地" => [100.0, 100.0, 100.0, 100.0],

        // ── 裸地 ──
        "bare" | "裸地" | "bareland" => [77.0, 86.0, 91.0, 94.0],
        "mining" | "采矿用地" => [80.0, 87.0, 92.0, 95.0],

        // ── 透水铺装 ──
        "pervious_paving" | "透水铺装" => [60.0, 74.0, 84.0, 87.0],

        _ => [67.0, 78.0, 85.0, 89.0], // 默认农田
    }
}

// ──────────────────────────────────────────────
// 核心函数
// ──────────────────────────────────────────────

/// 获取指定土地利用和水文土壤分组的 CN-II 值。
pub fn get_cn_ii(landuse: &str, soil_group: SoilGroup) -> f64 {
    let values = cn_ii_for_landuse(landuse);
    let idx = match soil_group {
        SoilGroup::A => 0,
        SoilGroup::B => 1,
        SoilGroup::C => 2,
        SoilGroup::D => 3,
    };
    values[idx]
}

/// 调整 CN 值用于不同前期土壤湿度条件。
///
/// 使用 Hawkins 公式：
/// - AMC I (干): CN_I = CN_II / (2.3 - 0.013 × CN_II)
/// - AMC III (湿): CN_III = CN_II / (0.43 + 0.0057 × CN_II)
pub fn adjust_cn_for_amc(cn_ii: f64, amc: AMC) -> f64 {
    match amc {
        AMC::Dry => cn_ii / (2.3 - 0.013 * cn_ii),
        AMC::Normal => cn_ii,
        AMC::Wet => cn_ii / (0.43 + 0.0057 * cn_ii),
    }
    .clamp(0.0, 100.0)
}

/// 计算潜在最大蓄水能力 S (mm)。
///
/// `S = 25400 / CN - 254`
pub fn compute_s(cn: f64) -> f64 {
    if cn <= 0.0 {
        return f64::INFINITY;
    }
    if cn >= 100.0 {
        return 0.0;
    }
    25400.0 / cn - 254.0
}

/// 计算初损 Ia (mm)。
///
/// `Ia = Ia_ratio × S`，标准默认 Ia_ratio = 0.2
pub fn compute_initial_abstraction(s: f64, ia_ratio: f64) -> f64 {
    (ia_ratio * s).max(0.0)
}

/// 计算单场降雨的径流深 (mm)。
///
/// `Q = (P - Ia)² / (P - Ia + S)` 当 P > Ia，否则 Q = 0
pub fn compute_runoff(rainfall_mm: f64, cn: f64, ia_ratio: f64) -> f64 {
    let s = compute_s(cn);
    compute_runoff_with_s(rainfall_mm, s, ia_ratio)
}

/// 使用已知 S 计算径流深。
pub fn compute_runoff_with_s(rainfall_mm: f64, s_mm: f64, ia_ratio: f64) -> f64 {
    if s_mm.is_infinite() {
        return 0.0;
    }
    let ia = compute_initial_abstraction(s_mm, ia_ratio);
    if rainfall_mm <= ia {
        return 0.0;
    }
    let numerator = (rainfall_mm - ia) * (rainfall_mm - ia);
    let denominator = rainfall_mm - ia + s_mm;
    if denominator <= 0.0 {
        return 0.0;
    }
    numerator / denominator
}

/// 栅格化的 SCS-CN 径流计算。
///
/// # 参数
///
/// * `rainfall_mm` — 单场降雨量 (mm)
/// * `cn_grid` — CN 栅格数组
/// * `cells` — 有效像元数
/// * `ia_ratio` — 初损比（默认 0.2）
pub fn compute_runoff_grid(rainfall_mm: f64, cn_grid: &[f64], ia_ratio: f64) -> Vec<f64> {
    cn_grid
        .iter()
        .map(|&cn| compute_runoff(rainfall_mm, cn, ia_ratio))
        .collect()
}

/// 完整的 SCS-CN 径流评估。
///
/// # 参数
///
/// * `landuse_grid` — 土地利用代码栅格
/// * `soil_group_grid` — 水文土壤分组代码栅格（'A'/'B'/'C'/'D'）
/// * `rainfall_mm` — 降雨量 (mm)
/// * `amc` — 前期土壤湿度条件
/// * `cellsize_m` — 像元大小 (m)
/// * `ia_ratio` — 初损比（默认 0.2）
pub fn assess_runoff(
    landuse_grid: &[&str],
    soil_group_grid: &[SoilGroup],
    rainfall_mm: f64,
    amc: AMC,
    cells: usize,
    cellsize_m: f64,
    ia_ratio: f64,
) -> ScsCnAssessment {
    // 计算 CN 栅格
    let cn_grid: Vec<f64> = landuse_grid
        .iter()
        .zip(soil_group_grid.iter())
        .map(|(lu, sg)| get_cn_ii(lu, *sg))
        .collect();

    // AMC 修正
    let cn_amc: Vec<f64> = cn_grid
        .iter()
        .map(|&cn| adjust_cn_for_amc(cn, amc))
        .collect();

    // S + Ia
    let s = compute_s(cn_amc.iter().sum::<f64>() / cn_amc.len().max(1) as f64);
    let ia = compute_initial_abstraction(s, ia_ratio);

    // 径流栅格
    let runoff_mm_grid = compute_runoff_grid(rainfall_mm, &cn_amc, ia_ratio);

    // 统计
    let cn_mean = if cells > 0 {
        cn_amc.iter().sum::<f64>() / cells as f64
    } else {
        0.0
    };
    let runoff_mean = if cells > 0 {
        runoff_mm_grid.iter().sum::<f64>() / cells as f64
    } else {
        0.0
    };
    let area_ha = cells as f64 * cellsize_m * cellsize_m / 10000.0;
    let total_runoff_vol = runoff_mean / 1000.0 * cells as f64 * cellsize_m * cellsize_m;
    let runoff_coef = if rainfall_mm > 0.0 {
        runoff_mean / rainfall_mm
    } else {
        0.0
    };

    ScsCnAssessment {
        cn_mean,
        s_mm: s,
        ia_mm: ia,
        rainfall_mm,
        runoff_mm: runoff_mean,
        runoff_coefficient: runoff_coef,
        area_ha,
        total_runoff_volume_m3: total_runoff_vol,
        runoff_grid: runoff_mm_grid,
    }
}

// ──────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_get_cn_ii() {
        // 森林 B 组 ≈ 55
        let cn = get_cn_ii("forest_good", SoilGroup::B);
        assert!(approx_eq(cn, 55.0, 1e-6));

        // 水体 100
        let cn_w = get_cn_ii("water", SoilGroup::A);
        assert!(approx_eq(cn_w, 100.0, 1e-6));

        // 裸地 D 组
        let cn_bare = get_cn_ii("bare", SoilGroup::D);
        assert!(approx_eq(cn_bare, 94.0, 1e-6));

        // 未知 → 默认农田
        let cn_def = get_cn_ii("unknown_landuse", SoilGroup::C);
        assert!(approx_eq(cn_def, 85.0, 1e-6)); // 农田 C 组
    }

    #[test]
    fn test_adjust_cn() {
        // AMC II → 不变
        assert!(approx_eq(adjust_cn_for_amc(70.0, AMC::Normal), 70.0, 1e-6));

        // AMC I → 减小
        let dry = adjust_cn_for_amc(70.0, AMC::Dry);
        assert!(dry < 70.0);
        assert!(dry > 40.0);

        // AMC III → 增大
        let wet = adjust_cn_for_amc(70.0, AMC::Wet);
        assert!(wet > 70.0);
        assert!(wet < 100.0);
    }

    #[test]
    fn test_compute_s() {
        // CN=100 → S=0
        assert!(approx_eq(compute_s(100.0), 0.0, 1e-6));
        // CN=0 → ∞
        assert!(compute_s(0.0).is_infinite());
        // CN=50 → S=25400/50 - 254 = 508 - 254 = 254
        assert!(approx_eq(compute_s(50.0), 254.0, 1e-6));
        // CN=70 → S=25400/70 - 254 ≈ 362.857 - 254 ≈ 108.857
        assert!(approx_eq(compute_s(70.0), 25400.0 / 70.0 - 254.0, 1e-6));
    }

    #[test]
    fn test_compute_runoff() {
        // CN=70 → S≈108.86, Ia≈21.77
        // P=100mm > Ia → Q = (100-21.77)² / (100-21.77+108.86) ≈ 6118/187.09 ≈ 32.7
        let q = compute_runoff(100.0, 70.0, 0.2);
        assert!(q > 20.0 && q < 50.0);

        // P < Ia → Q=0
        let q2 = compute_runoff(10.0, 70.0, 0.2);
        assert_eq!(q2, 0.0);

        // CN=100 → S=0, Ia=0 → Q=P
        let q3 = compute_runoff(50.0, 100.0, 0.2);
        assert!(approx_eq(q3, 50.0, 1e-6));

        // CN=0 → S=∞ → Q=0
        let q4 = compute_runoff(100.0, 0.0, 0.2);
        assert_eq!(q4, 0.0);
    }

    #[test]
    fn test_soil_group_from_str() {
        assert_eq!(SoilGroup::from_str("A"), SoilGroup::A);
        assert_eq!(SoilGroup::from_str("a"), SoilGroup::A);
        assert_eq!(SoilGroup::from_str("B"), SoilGroup::B);
        assert_eq!(SoilGroup::from_str("C"), SoilGroup::C);
        assert_eq!(SoilGroup::from_str("D"), SoilGroup::D);
        assert_eq!(SoilGroup::from_str("unknown"), SoilGroup::B);
    }

    #[test]
    fn test_assess_runoff() {
        let n = 4;
        let landuse = vec!["forest_good", "cropland", "urban", "water"];
        let soil: Vec<SoilGroup> = vec![SoilGroup::A, SoilGroup::B, SoilGroup::C, SoilGroup::D];

        let landuse_refs: Vec<&str> = landuse.iter().copied().collect();

        let result = assess_runoff(&landuse_refs, &soil, 100.0, AMC::Normal, n, 30.0, 0.2);

        assert!(result.cn_mean > 0.0 && result.cn_mean <= 100.0);
        assert!(result.runoff_mm > 0.0);
        assert!(result.runoff_coefficient > 0.0 && result.runoff_coefficient <= 1.0);
        assert!(result.total_runoff_volume_m3 > 0.0);
        assert_eq!(result.runoff_grid.len(), n);

        // 水体 CN=100 → Q=P
        assert!(approx_eq(result.runoff_grid[3], 100.0, 1e-6));
    }

    #[test]
    fn test_assess_runoff_dry_vs_wet() {
        let n = 2;
        let landuse: Vec<&str> = vec!["cropland", "forest_good"];
        let soil = vec![SoilGroup::B, SoilGroup::B];

        let dry = assess_runoff(&landuse, &soil, 50.0, AMC::Dry, n, 30.0, 0.2);
        let wet = assess_runoff(&landuse, &soil, 50.0, AMC::Wet, n, 30.0, 0.2);

        assert!(wet.runoff_mm > dry.runoff_mm);
        assert!(wet.cn_mean >= dry.cn_mean);
    }

    #[test]
    fn test_compute_runoff_grid() {
        let cn = vec![70.0, 80.0, 90.0, 100.0];
        let runoff = compute_runoff_grid(100.0, &cn, 0.2);

        // 高 CN → 高径流
        assert!(runoff[0] < runoff[1]);
        assert!(runoff[1] < runoff[2]);
        // CN=100 → Q=P
        assert!(approx_eq(runoff[3], 100.0, 1e-6));
    }

    #[test]
    fn test_different_ia_ratios() {
        // Ia=0 → 全部降雨产流
        let q0 = compute_runoff(100.0, 70.0, 0.0);
        // Ia=0.2 → 有初损
        let q2 = compute_runoff(100.0, 70.0, 0.2);
        assert!(q0 > q2);
        // Ia=0 时 Q = P²/(P+S) ≈ 47.9 对 CN=70
    }

    #[test]
    fn test_cn_ii_for_landuse_known_values() {
        assert_eq!(cn_ii_for_landuse("urban"), [89.0, 92.0, 94.0, 95.0]);
        assert_eq!(cn_ii_for_landuse("森林"), [30.0, 55.0, 70.0, 77.0]);
        assert_eq!(cn_ii_for_landuse("优质牧草"), [39.0, 61.0, 74.0, 80.0]);
        assert_eq!(cn_ii_for_landuse("道路"), [98.0, 98.0, 98.0, 98.0]);
        assert_eq!(cn_ii_for_landuse("耕地"), [67.0, 78.0, 85.0, 89.0]);
        assert_eq!(cn_ii_for_landuse("unknown_xyz"), [67.0, 78.0, 85.0, 89.0]);
        assert_eq!(cn_ii_for_landuse("forest_good"), [30.0, 55.0, 70.0, 77.0]);
        assert_eq!(cn_ii_for_landuse("paved"), [98.0, 98.0, 98.0, 98.0]);
    }
}
