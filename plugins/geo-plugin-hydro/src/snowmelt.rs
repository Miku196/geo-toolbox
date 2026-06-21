/// 融雪径流模型 — 度日因子法
///
/// 寒冷地区水文过程：积雪累积 → 融雪 → 融水径流。
/// 与 SCS-CN 产流衔接：融水作为有效降雨输入。
///
/// 核心公式:
///   M = DDF × (T - Tbase)  when T > Tbase
///   M = 0                     when T ≤ Tbase
///
/// 其中:
///   M     = 融雪深度 (mm H2O / day)
///   DDF   = 度日因子 (mm/°C/day)，典型范围 2.0-6.0
///   T     = 日均气温 (°C)
///   Tbase = 融雪基准温度 (°C)，典型 0°C
///
/// 扩展模型: 积雪累积 / 冷含量 / 雨+雪划分 / 液态水持水能力
///
/// # 参考文献
/// Hock, R. (2003). Temperature index melt modelling in mountain areas.
/// Journal of Hydrology, 282(1-4), 104-115.
/// Anderson, E.A. (1973). National Weather Service River Forecast
/// System—Snow accumulation and ablation model. NOAA Tech Memo NWS HYDRO-17.
use serde::{Deserialize, Serialize};

// ─── 数据结构 ───

/// 融雪模型参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnowmeltParams {
    /// 度日因子 (mm/°C/day)，融雪速率
    pub ddf: f64,
    /// 融雪基准温度 (°C)
    pub t_base: f64,
    /// 雨/雪分界温度 (°C)。T < t_rain → 降雪; T ≥ t_rain → 降雨
    pub t_rain: f64,
    /// 积雪液态水持水能力 (水当量比例)，典型 0.05-0.10
    pub liquid_holding_capacity: f64,
    /// 冷含量因子 (mm/°C)，再冻结速率
    pub refreeze_factor: f64,
}

impl Default for SnowmeltParams {
    fn default() -> Self {
        Self {
            ddf: 3.5,
            t_base: 0.0,
            t_rain: 1.0,
            liquid_holding_capacity: 0.05,
            refreeze_factor: 0.05,
        }
    }
}

/// 单日融雪状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnowmeltDay {
    /// 日均气温 (°C)
    pub temp_c: f64,
    /// 日降水量 (mm)
    pub precip_mm: f64,
    /// 降雨量 (mm)
    pub rainfall_mm: f64,
    /// 降雪量 (mm 水当量)
    pub snowfall_mm: f64,
    /// 潜在融雪量 (mm 水当量)，公式值
    pub potential_melt_mm: f64,
    /// 实际融雪量 (mm 水当量)，受限于积雪量
    pub actual_melt_mm: f64,
    /// 再冻结量 (mm)
    pub refreeze_mm: f64,
    /// 液态水出流量 (mm) — 进入土壤的融水+降雨
    pub outflow_mm: f64,
    /// 当日末积雪水当量 (mm)
    pub swe_mm: f64,
}

/// 融雪模拟结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnowmeltResult {
    /// 逐日融雪状态
    pub days: Vec<SnowmeltDay>,
    /// 总降雨量
    pub total_rainfall_mm: f64,
    /// 总降雪量 (水当量)
    pub total_snowfall_mm: f64,
    /// 总融雪量
    pub total_melt_mm: f64,
    /// 总出流量 (降雨 + 融雪 → 土壤)
    pub total_outflow_mm: f64,
    /// 最大积雪水当量
    pub max_swe_mm: f64,
    /// 积雪覆盖天数
    pub snow_cover_days: usize,
    /// 融雪天数
    pub melt_days: usize,
}

// ─── 核心函数 ───

/// 度日因子融雪计算 (逐日)。
///
/// # 参数
/// * `temp_c` — 日均气温 (°C)
/// * `params` — 融雪模型参数
pub fn degree_day_melt(temp_c: f64, params: &SnowmeltParams) -> f64 {
    if temp_c > params.t_base {
        params.ddf * (temp_c - params.t_base)
    } else {
        0.0
    }
}

/// 雨/雪划分：日降水量分解为降雨和降雪。
pub fn partition_rain_snow(precip_mm: f64, temp_c: f64, t_rain: f64) -> (f64, f64) {
    if temp_c >= t_rain {
        (precip_mm, 0.0)
    } else {
        (0.0, precip_mm)
    }
}

/// 逐日融雪模拟。
///
/// 输入日气温和日降水量序列，模拟积雪积累-消融过程。
///
/// # 参数
/// * `temps_c` — 日均气温序列 (°C)
/// * `precip_mm` — 日降水量序列 (mm)
/// * `params` — 融雪参数
/// * `initial_swe` — 初始积雪水当量 (mm)
pub fn simulate_snowmelt(
    temps_c: &[f64],
    precip_mm: &[f64],
    params: &SnowmeltParams,
    initial_swe: f64,
) -> SnowmeltResult {
    let n = temps_c.len().min(precip_mm.len());
    let mut days = Vec::with_capacity(n);
    let mut swe = initial_swe;
    let mut max_swe = swe;
    let mut snow_days = 0usize;
    let mut melt_days = 0usize;
    let mut total_rain = 0.0;
    let mut total_snow = 0.0;
    let mut total_melt = 0.0;
    let mut total_outflow = 0.0;

    for i in 0..n {
        let temp = temps_c[i];
        let precip = precip_mm[i];

        // 雨雪划分
        let (rain, snow) = partition_rain_snow(precip, temp, params.t_rain);
        total_rain += rain;
        total_snow += snow;

        // 积雪增加
        swe += snow;

        // 潜在融雪
        let potential_melt = degree_day_melt(temp, params);
        let mut actual_melt = potential_melt.min(swe);

        // 再冻结：液态水在负温时重新冻结
        let mut refreeze = 0.0;
        if temp < params.t_base {
            refreeze = params.refreeze_factor * (params.t_base - temp);
            if refreeze > 0.0 {
                // 再冻结从积雪液态水含量中扣除
                let liquid_water = swe * params.liquid_holding_capacity;
                refreeze = refreeze.min(liquid_water);
                swe += refreeze;
            }
        }

        // 积雪消融
        swe -= actual_melt;

        // 液态水持水能力：雪层内可滞留部分液态水
        let liquid_capacity = swe * params.liquid_holding_capacity;
        let gross_outflow = rain + actual_melt;
        let outflow = if gross_outflow > liquid_capacity {
            gross_outflow - liquid_capacity
        } else {
            0.0
        };

        total_melt += actual_melt;
        total_outflow += outflow;

        if swe > max_swe {
            max_swe = swe;
        }
        if swe > 0.0 {
            snow_days += 1;
        }
        if actual_melt > 0.0 {
            melt_days += 1;
        }

        days.push(SnowmeltDay {
            temp_c: temp,
            precip_mm: precip,
            rainfall_mm: rain,
            snowfall_mm: snow,
            potential_melt_mm: potential_melt,
            actual_melt_mm: actual_melt,
            refreeze_mm: refreeze,
            outflow_mm: outflow,
            swe_mm: swe,
        });
    }

    SnowmeltResult {
        days,
        total_rainfall_mm: total_rain,
        total_snowfall_mm: total_snow,
        total_melt_mm: total_melt,
        total_outflow_mm: total_outflow,
        max_swe_mm: max_swe,
        snow_cover_days: snow_days,
        melt_days,
    }
}

/// 提取融雪后的有效降水序列 (降雨 + 融水 → 地表径流输入)。
///
/// 输出可直接作为 SCS-CN 产流模型的降雨输入。
pub fn effective_precipitation(result: &SnowmeltResult) -> Vec<f64> {
    result.days.iter().map(|d| d.outflow_mm).collect()
}

/// 提取有效降水 + 原始降雨的总径流输入序列。
///
/// 用于 SCS-CN 或单位线汇流，将融雪水加入降雨总量中。
pub fn combined_rainfall_melt(rainfall_daily: &[f64], melt_daily: &[f64]) -> Vec<f64> {
    let n = rainfall_daily.len().min(melt_daily.len());
    let mut combined = Vec::with_capacity(n);
    for i in 0..n {
        combined.push(rainfall_daily[i] + melt_daily[i]);
    }
    combined
}

// ─── 积雪参数典型值 ───

/// 常见地貌度日因子参考值 (Hock, 2003)。
pub fn default_ddf_for_terrain(terrain: &str) -> f64 {
    match terrain.to_lowercase().as_str() {
        "glacier" | "冰川" => 6.0,
        "snow" | "积雪" | "snowfield" => 4.5,
        "forest" | "森林" | "forest_clearing" => 2.5,
        "alpine" | "高山" | "tundra" | "冻原" => 3.5,
        _ => 3.5, // 默认开阔地
    }
}

// ─── 年融雪统计 ───

/// 年融雪水量平衡统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnowWaterBalance {
    /// 年总降雪 (mm)
    pub snowfall_mm: f64,
    /// 年总降雨 (mm)
    pub rainfall_mm: f64,
    /// 年总融雪 (mm)
    pub melt_mm: f64,
    /// 年总出流 (mm)
    pub outflow_mm: f64,
    /// 融雪径流比 (融雪 / 出流)
    pub snowmelt_fraction: f64,
    /// 年最大积雪 (mm SWE)
    pub max_swe_mm: f64,
    /// 积雪覆盖天数
    pub snow_days: usize,
}

/// 年融雪水量平衡汇总。
pub fn annual_water_balance(result: &SnowmeltResult) -> SnowWaterBalance {
    SnowWaterBalance {
        snowfall_mm: result.total_snowfall_mm,
        rainfall_mm: result.total_rainfall_mm,
        melt_mm: result.total_melt_mm,
        outflow_mm: result.total_outflow_mm,
        snowmelt_fraction: if result.total_outflow_mm > 0.0 {
            result.total_melt_mm / result.total_outflow_mm
        } else {
            0.0
        },
        max_swe_mm: result.max_swe_mm,
        snow_days: result.snow_cover_days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_degree_day_melt() {
        let params = SnowmeltParams::default();
        assert_relative_eq!(degree_day_melt(5.0, &params), 17.5); // 3.5 * 5
        assert_relative_eq!(degree_day_melt(-2.0, &params), 0.0);
        assert_relative_eq!(degree_day_melt(0.0, &params), 0.0);
    }

    #[test]
    fn test_custom_ddf() {
        let params = SnowmeltParams {
            ddf: 5.0,
            ..Default::default()
        };
        assert_relative_eq!(degree_day_melt(3.0, &params), 15.0);
    }

    #[test]
    fn test_partition_rain_snow() {
        assert_eq!(partition_rain_snow(10.0, 2.0, 1.0), (10.0, 0.0));
        assert_eq!(partition_rain_snow(10.0, 0.0, 1.0), (0.0, 10.0));
        assert_eq!(partition_rain_snow(10.0, 1.0, 1.0), (10.0, 0.0)); // boundary
    }

    #[test]
    fn test_simple_melt() {
        // 5天：第1-2天寒冷降雪，第3-5天升温融雪
        let temps = vec![-5.0, -2.0, 3.0, 5.0, 8.0];
        let precip = vec![10.0, 15.0, 0.0, 0.0, 0.0];
        let params = SnowmeltParams::default();

        let result = simulate_snowmelt(&temps, &precip, &params, 0.0);

        assert_eq!(result.days.len(), 5);
        // 第1天: -5°C, 10mm snow → swe=10 + refreeze 0.25
        assert_relative_eq!(result.days[0].swe_mm, 10.25);
        assert_relative_eq!(result.days[0].snowfall_mm, 10.0);
        // 第2天: -2°C, 15mm snow → swe=25.25 + refreeze 0.1
        // 累积: 10.25 (day1) + 15 (snow) + 0.05*2 (refreeze) = 25.35
        assert_relative_eq!(result.days[1].swe_mm, 25.35);
        // 第3天: 3°C, melt=10.5 → swe=14.5
        assert_relative_eq!(result.days[2].actual_melt_mm, 10.5);
        // 第5天: 8°C → melt=28, swe should be depleted
        assert!(result.days[4].swe_mm < 5.0);
        assert!(result.total_melt_mm > 0.0);
        assert!(result.snow_cover_days >= 2);
    }

    #[test]
    fn test_refreeze() {
        // Day 1: snow accumulation, Day 2: melt, Day 3: refreeze
        let temps = vec![-3.0, 5.0, -2.0];
        let precip = vec![20.0, 0.0, 0.0];
        let params = SnowmeltParams::default();
        let result = simulate_snowmelt(&temps, &precip, &params, 0.0);
        // Day 2: melt 17.5mm, swe: 20-17.5=2.5
        assert!(result.days[1].actual_melt_mm > 0.0);
        // Day 3: potential refreeze = 0.05*(0-(-2)) = 0.1mm
        assert!(result.days[2].refreeze_mm >= 0.0);
    }

    #[test]
    fn test_effective_precipitation() {
        let temps = vec![5.0, 5.0, 5.0]; // all melt
        let precip = vec![0.0, 0.0, 0.0];
        let params = SnowmeltParams {
            ddf: 5.0,
            ..Default::default()
        };
        let result = simulate_snowmelt(&temps, &precip, &params, 100.0);
        let eff = effective_precipitation(&result);
        assert!(eff.iter().sum::<f64>() > 0.0);
    }

    #[test]
    fn test_combined_rainfall_melt() {
        let rainfall = vec![10.0, 0.0, 5.0];
        let melt = vec![0.0, 20.0, 5.0];
        let combined = combined_rainfall_melt(&rainfall, &melt);
        assert_relative_eq!(combined[0], 10.0);
        assert_relative_eq!(combined[1], 20.0);
        assert_relative_eq!(combined[2], 10.0);
    }

    #[test]
    fn test_ddf_terrain() {
        assert_relative_eq!(default_ddf_for_terrain("glacier"), 6.0);
        assert_relative_eq!(default_ddf_for_terrain("森林"), 2.5);
        assert_relative_eq!(default_ddf_for_terrain("unknown"), 3.5);
    }

    #[test]
    fn test_water_balance() {
        let temps = vec![-5.0, 3.0, 8.0];
        let precip = vec![20.0, 0.0, 0.0];
        let params = SnowmeltParams::default();
        let result = simulate_snowmelt(&temps, &precip, &params, 0.0);
        let balance = annual_water_balance(&result);
        assert_relative_eq!(balance.snowfall_mm, 20.0);
        assert!(balance.melt_mm > 0.0);
        assert!(balance.snowmelt_fraction > 0.0);
    }
}
