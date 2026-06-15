//! InVEST 碳存储 + 水源涵养。
//!
//! 参考 InVEST 3.x 模型 (Natural Capital Project, Stanford)。
//!
//! ## 碳存储
//!
//! Total = C_above + C_below + C_soil + C_dead
//!
//! ## 水源涵养（产水量）
//!
//! 基于 Budyko 曲线：
//! `Y(x) = (1 - AET(x)/P(x)) × P(x)`
//! `AET/P = 1 + PET/P - (1 + (PET/P)^ω)^(1/ω)`
//! `ω = Z × AWC/P + 1.25`

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// 碳库密度（每类土地利用类型的碳密度 Mg/ha）
// ──────────────────────────────────────────────

/// 土地利用类型的四碳库密度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonPoolDensity {
    /// 地上生物量碳 (Mg C/ha)
    pub aboveground: f64,
    /// 地下生物量碳 (Mg C/ha)
    pub belowground: f64,
    /// 土壤有机碳 (Mg C/ha)
    pub soil: f64,
    /// 枯落物碳 (Mg C/ha)
    pub dead: f64,
}

impl CarbonPoolDensity {
    pub fn new(above: f64, below: f64, soil: f64, dead: f64) -> Self {
        Self {
            aboveground: above,
            belowground: below,
            soil,
            dead,
        }
    }

    pub fn total(&self) -> f64 {
        self.aboveground + self.belowground + self.soil + self.dead
    }
}

/// 默认碳库密度表（中国典型生态系统，单位 Mg C/ha）。
///
/// 来源：IPCC 2006 GL + 中国森林生态系统碳储量研究。
pub fn default_carbon_pools(landuse: &str) -> CarbonPoolDensity {
    match landuse {
        // ── 森林 ──
        "evergreen_broadleaf" | "常绿阔叶林" => CarbonPoolDensity::new(120.0, 24.0, 95.0, 5.0),
        "evergreen_needleleaf" | "常绿针叶林" => CarbonPoolDensity::new(80.0, 16.0, 80.0, 8.0),
        "deciduous_broadleaf" | "落叶阔叶林" => CarbonPoolDensity::new(90.0, 18.0, 85.0, 6.0),
        "mixed_forest" | "混交林" => CarbonPoolDensity::new(100.0, 20.0, 90.0, 6.0),
        "forest" | "林地" | "森林" => CarbonPoolDensity::new(100.0, 20.0, 88.0, 6.0),

        // ── 灌丛 ──
        "shrub" | "灌木" | "灌丛" => CarbonPoolDensity::new(15.0, 5.0, 60.0, 3.0),

        // ── 草地 ──
        "grassland" | "草原" | "草地" => CarbonPoolDensity::new(2.0, 4.0, 70.0, 2.0),
        "pasture" | "牧草" | "牧场" => CarbonPoolDensity::new(3.0, 5.0, 65.0, 2.0),

        // ── 农田 ──
        "cropland" | "耕地" | "农田" => CarbonPoolDensity::new(5.0, 1.0, 45.0, 1.0),
        "rice" | "水田" | "稻田" => CarbonPoolDensity::new(3.0, 1.0, 50.0, 1.0),
        "orchard" | "果园" => CarbonPoolDensity::new(25.0, 8.0, 50.0, 3.0),

        // ── 湿地 ──
        "wetland" | "湿地" => CarbonPoolDensity::new(10.0, 2.0, 200.0, 10.0),
        "mangrove" | "红树林" => CarbonPoolDensity::new(80.0, 30.0, 250.0, 15.0),

        // ── 城市 ──
        "urban" | "建设用地" | "城市" => CarbonPoolDensity::new(2.0, 0.0, 15.0, 0.0),
        "green_space" | "绿地" => CarbonPoolDensity::new(8.0, 3.0, 40.0, 2.0),

        // ── 裸地 ──
        "bare" | "裸地" => CarbonPoolDensity::new(0.0, 0.0, 5.0, 0.0),
        "mining" | "采矿用地" => CarbonPoolDensity::new(0.0, 0.0, 3.0, 0.0),
        "desert" | "荒漠" | "沙漠" => CarbonPoolDensity::new(0.0, 0.0, 2.0, 0.0),

        // ── 水体 ──
        "water" | "水体" | "水域" => CarbonPoolDensity::new(0.0, 0.0, 0.0, 0.0),

        _ => CarbonPoolDensity::new(5.0, 1.0, 45.0, 1.0), // 默认农田
    }
}

// ──────────────────────────────────────────────
// 碳存储评估
// ──────────────────────────────────────────────

/// InVEST 碳存储评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonStorageAssessment {
    /// 总碳储量 (Mg C)
    pub total_carbon_mg: f64,
    /// 地上生物量碳 (Mg C)
    pub aboveground_mg: f64,
    /// 地下生物量碳 (Mg C)
    pub belowground_mg: f64,
    /// 土壤有机碳 (Mg C)
    pub soil_mg: f64,
    /// 枯落物碳 (Mg C)
    pub dead_mg: f64,
    /// 碳密度均值 (Mg C/ha)
    pub carbon_density_mg_per_ha: f64,
    /// 面积 (ha)
    pub area_ha: f64,
    /// 像元数
    pub cells: usize,
    /// 各类土地的碳储量明细
    pub breakdown: Vec<LanduseCarbon>,
}

/// 单类土地的碳储量明细。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanduseCarbon {
    pub landuse: String,
    pub pixel_count: usize,
    pub area_ha: f64,
    pub carbon_mg: f64,
}

/// 计算 InVEST 碳存储。
///
/// # 参数
///
/// * `landuse_grid` — 土地利用栅格（每个像元的土地利用类型字符串）
/// * `cellsize_m` — 像元大小 (m)
/// * `pools` — 可选的自定义碳库参数表（键为 landuse 字符串）
pub fn assess_carbon_storage(
    landuse_grid: &[&str],
    cellsize_m: f64,
    pools: Option<&std::collections::HashMap<String, CarbonPoolDensity>>,
) -> CarbonStorageAssessment {
    let n = landuse_grid.len();
    let cell_area_ha = cellsize_m * cellsize_m / 10000.0;
    let area_ha = n as f64 * cell_area_ha;

    let mut total = 0.0_f64;
    let mut above = 0.0_f64;
    let mut below = 0.0_f64;
    let mut soil = 0.0_f64;
    let mut dead = 0.0_f64;

    let mut landuse_counts: std::collections::HashMap<String, (usize, f64)> =
        std::collections::HashMap::new();

    for &lu in landuse_grid {
        let pool = match pools {
            Some(ref map) => map
                .get(lu)
                .cloned()
                .unwrap_or_else(|| default_carbon_pools(lu)),
            None => default_carbon_pools(lu),
        };

        let cell_carbon = pool.total();
        total += cell_carbon;
        above += pool.aboveground;
        below += pool.belowground;
        soil += pool.soil;
        dead += pool.dead;

        let entry = landuse_counts.entry(lu.to_string()).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += cell_carbon;
    }

    let mut breakdown: Vec<LanduseCarbon> = landuse_counts
        .into_iter()
        .map(|(lu, (count, carbon))| LanduseCarbon {
            pixel_count: count,
            area_ha: count as f64 * cell_area_ha,
            carbon_mg: carbon,
            landuse: lu,
        })
        .collect();
    breakdown.sort_by(|a, b| {
        b.carbon_mg
            .partial_cmp(&a.carbon_mg)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    CarbonStorageAssessment {
        total_carbon_mg: total,
        aboveground_mg: above,
        belowground_mg: below,
        soil_mg: soil,
        dead_mg: dead,
        carbon_density_mg_per_ha: if area_ha > 0.0 { total / area_ha } else { 0.0 },
        area_ha,
        cells: n,
        breakdown,
    }
}

// ──────────────────────────────────────────────
// 水源涵养（产水量）
// ──────────────────────────────────────────────

/// 产水量评估输入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterYieldInput {
    /// 年降雨量 (mm/yr)
    pub precipitation: f64,
    /// 潜在蒸散发 (mm/yr)
    pub pet: f64,
    /// 植物有效含水率 (0-1)，基于土壤质地和根系深度
    pub available_water_content: f64,
    /// Zhang 系数（季节经验常数，1-30）
    pub z_coefficient: f64,
}

/// 产水量评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterYieldAssessment {
    /// 产水量 (mm/yr)
    pub water_yield_mm: f64,
    /// 实际蒸散发 (mm/yr)
    pub aet_mm: f64,
    /// 蒸散发比 AET/P
    pub aet_p_ratio: f64,
    /// 年降雨量 (mm)
    pub precipitation: f64,
    /// 潜在蒸散发 (mm)
    pub pet: f64,
    /// 面积 (ha)
    pub area_ha: f64,
    /// 总产水量 (m³/yr)
    pub total_water_yield_m3: f64,
    /// 各像元产水量 (mm/yr)
    pub yield_grid: Vec<f64>,
}

/// Budyko 曲线蒸散发计算。
///
/// `AET/P = 1 + PET/P - (1 + (PET/P)^ω)^(1/ω)`
pub fn budyko_aet_p_ratio(pet_p_ratio: f64, omega: f64) -> f64 {
    if pet_p_ratio <= 0.0 {
        return 0.0;
    }
    if omega <= 0.0 {
        return 1.0;
    }
    let term = 1.0 + pet_p_ratio.powf(omega);
    1.0 + pet_p_ratio - term.powf(1.0 / omega)
}

/// 计算 ω（植物水利用的非物理参数）。
///
/// `ω = Z × AWC/P + 1.25`
pub fn compute_omega(awc: f64, precipitation: f64, z_coefficient: f64) -> f64 {
    if precipitation <= 0.0 {
        return 1.25;
    }
    (z_coefficient * awc / precipitation + 1.25).max(1.0)
}

/// 计算单像元产水量 (mm/yr)。
pub fn compute_water_yield(precipitation: f64, pet: f64, awc: f64, z_coefficient: f64) -> f64 {
    if precipitation <= 0.0 {
        return 0.0;
    }
    let pet_p = pet / precipitation;
    let omega = compute_omega(awc, precipitation, z_coefficient);
    let aet_p = budyko_aet_p_ratio(pet_p, omega).clamp(0.0, 1.0);
    let yield_mm = (1.0 - aet_p) * precipitation;
    yield_mm.max(0.0)
}

/// 计算栅格产水量。
pub fn compute_water_yield_grid(
    precipitation: &[f64],
    pet: &[f64],
    awc: &[f64],
    z_coefficient: f64,
    cells: usize,
) -> Vec<f64> {
    let mut yields = vec![0.0; cells];
    for i in 0..cells {
        let p = precipitation.get(i).copied().unwrap_or(0.0);
        let e = pet.get(i).copied().unwrap_or(0.0);
        let a = awc.get(i).copied().unwrap_or(0.0);
        yields[i] = compute_water_yield(p, e, a, z_coefficient);
    }
    yields
}

/// 完整的 InVEST 产水量评估。
pub fn assess_water_yield(
    precipitation: &[f64],
    pet: &[f64],
    awc: &[f64],
    z_coefficient: f64,
    cellsize_m: f64,
) -> WaterYieldAssessment {
    let cells = precipitation.len().min(pet.len()).min(awc.len());
    let yields = compute_water_yield_grid(precipitation, pet, awc, z_coefficient, cells);
    let area_ha = cells as f64 * cellsize_m * cellsize_m / 10000.0;

    let avg_p = if cells > 0 {
        precipitation.iter().take(cells).sum::<f64>() / cells as f64
    } else {
        0.0
    };
    let avg_wy = if cells > 0 {
        yields.iter().sum::<f64>() / cells as f64
    } else {
        0.0
    };
    let avg_pet = if cells > 0 {
        pet.iter().take(cells).sum::<f64>() / cells as f64
    } else {
        0.0
    };
    let avg_awc = if cells > 0 {
        awc.iter().take(cells).sum::<f64>() / cells as f64
    } else {
        0.0
    };

    let omega = compute_omega(avg_awc, avg_p, z_coefficient);
    let pet_p = if avg_p > 0.0 { avg_pet / avg_p } else { 0.0 };
    let aet_p = budyko_aet_p_ratio(pet_p, omega).clamp(0.0, 1.0);
    let aet_mm = aet_p * avg_p;
    let total_volume = avg_wy / 1000.0 * cells as f64 * cellsize_m * cellsize_m;

    WaterYieldAssessment {
        water_yield_mm: avg_wy,
        aet_mm,
        aet_p_ratio: aet_p,
        precipitation: avg_p,
        pet: avg_pet,
        area_ha,
        total_water_yield_m3: total_volume,
        yield_grid: yields,
    }
}

// ──────────────────────────────────────────────
// 综合评估：碳 + 水
// ──────────────────────────────────────────────

/// InVEST 综合评估结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestAssessment {
    pub carbon: CarbonStorageAssessment,
    pub water_yield: WaterYieldAssessment,
}

/// 完整的 InVEST 碳+水综合评估。
pub fn assess_invest(
    landuse_grid: &[&str],
    precipitation: &[f64],
    pet: &[f64],
    awc: &[f64],
    z_coefficient: f64,
    cellsize_m: f64,
) -> InvestAssessment {
    let carbon = assess_carbon_storage(landuse_grid, cellsize_m, None);
    let water = assess_water_yield(precipitation, pet, awc, z_coefficient, cellsize_m);
    InvestAssessment {
        carbon,
        water_yield: water,
    }
}

// ──────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_carbon_pools_default() {
        let pool = default_carbon_pools("forest");
        assert!(approx_eq(pool.total(), 214.0, 1e-6));

        let pool2 = default_carbon_pools("water");
        assert!(approx_eq(pool2.total(), 0.0, 1e-6));

        let pool3 = default_carbon_pools("unknown");
        assert!(pool3.total() > 0.0);
    }

    #[test]
    fn test_assess_carbon_storage() {
        let landuse = vec!["forest", "forest", "cropland", "water"];
        let result = assess_carbon_storage(&landuse, 30.0, None);

        assert_eq!(result.cells, 4);
        // 面积 = 4 * 900/10000 = 0.36 ha
        assert!(approx_eq(result.area_ha, 0.36, 1e-6));
        // 碳储量 = 2*214 + 52 + 0 = 480 Mg C
        assert!(approx_eq(result.total_carbon_mg, 480.0, 1e-6));
        assert!(approx_eq(
            result.carbon_density_mg_per_ha,
            480.0 / 0.36,
            1e-3
        ));

        // 明细应有 3 类土地
        assert_eq!(result.breakdown.len(), 3);
    }

    #[test]
    fn test_assess_carbon_storage_custom() {
        let landuse = vec!["urban", "green"];
        let mut pools = HashMap::new();
        pools.insert(
            "urban".to_string(),
            CarbonPoolDensity::new(1.0, 0.0, 10.0, 0.0),
        );
        pools.insert(
            "green".to_string(),
            CarbonPoolDensity::new(10.0, 2.0, 30.0, 1.0),
        );

        let result = assess_carbon_storage(&landuse, 30.0, Some(&pools));
        // urban=11, green=43 → total=54
        assert!(approx_eq(result.total_carbon_mg, 54.0, 1e-6));
    }

    #[test]
    fn test_budyko() {
        // PET/P = 1 → AET/P should be < 1 but > 0
        let ratio = budyko_aet_p_ratio(1.0, 3.0);
        assert!(ratio > 0.0 && ratio < 1.0);

        // PET/P → ∞ → AET/P → 1
        let ratio2 = budyko_aet_p_ratio(100.0, 3.0);
        assert!(ratio2 > 0.9);

        // PET/P = 0 → AET/P = 0
        let ratio3 = budyko_aet_p_ratio(0.0, 3.0);
        assert!(approx_eq(ratio3, 0.0, 1e-6));
    }

    #[test]
    fn test_omega() {
        // Z=5, AWC=0.1, P=1000 → ω = 5*100/1000 + 1.25 = 0.5 + 1.25 = 1.75
        let w = compute_omega(100.0, 1000.0, 5.0);
        assert!(approx_eq(w, 1.75, 1e-6));

        // P=0 → ω=1.25
        let w2 = compute_omega(100.0, 0.0, 5.0);
        assert!(approx_eq(w2, 1.25, 1e-6));
    }

    #[test]
    fn test_water_yield_single() {
        // P=1000, PET=500, AWC=100, Z=5
        // ω = 5*100/1000 + 1.25 = 1.75
        // PET/P = 0.5
        // AET/P ≈ 0.5 → Y = (1-0.5)*1000 = ~500
        let wy = compute_water_yield(1000.0, 500.0, 100.0, 5.0);
        assert!(wy > 0.0 && wy < 1000.0);
        assert!(wy > 500.0 && wy < 750.0);
    }

    #[test]
    fn test_water_yield_zero_rainfall() {
        assert!(approx_eq(
            compute_water_yield(0.0, 500.0, 100.0, 5.0),
            0.0,
            1e-6
        ));
    }

    #[test]
    fn test_assess_water_yield() {
        let p = vec![1000.0, 800.0, 600.0, 400.0];
        let pet = vec![500.0, 400.0, 300.0, 200.0];
        let awc = vec![100.0, 100.0, 100.0, 100.0];

        let result = assess_water_yield(&p, &pet, &awc, 5.0, 30.0);

        assert_eq!(result.yield_grid.len(), 4);
        assert!(result.water_yield_mm > 0.0);
        assert!(result.total_water_yield_m3 > 0.0);
        assert!(result.aet_p_ratio > 0.0 && result.aet_p_ratio < 1.0);
    }

    #[test]
    fn test_invest_assessment() {
        let landuse = vec!["forest", "cropland", "cropland", "forest"];
        let p = vec![1000.0; 4];
        let pet = vec![400.0; 4];
        let awc = vec![100.0; 4];

        let result = assess_invest(&landuse, &p, &pet, &awc, 5.0, 30.0);

        assert!(result.carbon.total_carbon_mg > 0.0);
        assert!(result.water_yield.water_yield_mm > 0.0);
    }

    #[test]
    fn test_carbon_pool_total() {
        let pool = CarbonPoolDensity::new(10.0, 20.0, 30.0, 40.0);
        assert!(approx_eq(pool.total(), 100.0, 1e-6));
        assert!(approx_eq(pool.aboveground, 10.0, 1e-6));
    }
}
