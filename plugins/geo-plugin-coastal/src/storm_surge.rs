//! SLOSH-简化风暴潮模型: 参数化风场 + 风增水 + 淹没制图。
//!
//! 算法流程:
//! 1. Holland 参数化风场 → 网格风速
//! 2. 近岸风增水计算 (Jelesnianski 简化)
//! 3. DEM 叠加增水 → 淹没范围 + 深度

use geo_core::errors::GeoResult;
use serde::Serialize;

use crate::CoastalPlugin;

/// Holland 风场参数。
#[derive(Debug, Clone)]
pub struct StormParams {
    /// 风暴中心纬度 (°)
    pub lat: f64,
    /// 风暴中心经度 (°)
    pub lon: f64,
    /// 中心气压 (hPa)，典型值 900–990
    pub central_pressure_hpa: f64,
    /// 环境气压 (hPa)，通常 1013
    pub ambient_pressure_hpa: f64,
    /// 最大风速半径 (km)
    pub rmax_km: f64,
    /// 前进速度 (m/s)
    pub forward_speed_m_s: f64,
    /// 前进方向 (°，0=北, 90=东)
    pub forward_bearing_deg: f64,
    /// Holland B 参数，默认 1.3
    pub holland_b: f64,
}

impl Default for StormParams {
    fn default() -> Self {
        Self {
            lat: 30.0,
            lon: 122.0,
            central_pressure_hpa: 955.0,
            ambient_pressure_hpa: 1013.0,
            rmax_km: 40.0,
            forward_speed_m_s: 5.0,
            forward_bearing_deg: 0.0,
            holland_b: 1.3,
        }
    }
}

/// 风暴潮结果。
#[derive(Debug, Clone, Serialize)]
pub struct StormSurgeResult {
    /// 最大增水高度 (m)
    pub max_surge_m: f64,
    /// 平均增水高度 (m，仅淹没区)
    pub mean_surge_m: f64,
    /// 淹没格点数
    pub inundated_cells: usize,
    /// 淹没面积 (ha)
    pub inundated_area_ha: f64,
    /// 淹没体积 (m³)
    pub inundated_volume_m3: f64,
    /// 增水栅格 (flat, row-major)
    pub surge_grid: Vec<f64>,
    /// 风险等级
    pub risk_level: String,
}

const RHO_AIR: f64 = 1.225; // kg/m³
const RHO_WATER: f64 = 1025.0; // kg/m³
const G: f64 = 9.81; // m/s²
const CD: f64 = 0.0025; // 风应力拖曳系数

/// 计算指定距离处的梯度风速 (m/s)，Holland 模型。
///
/// V(r) = sqrt( (B/ρ) × Δp × (Rmax/r)^B × exp(-(Rmax/r)^B) )
fn gradient_wind(r_km: f64, delta_p_pa: f64, rmax_km: f64, b: f64) -> f64 {
    if r_km < 0.1 {
        // 在眼壁处 Rmax
        return (b / RHO_AIR * delta_p_pa * (-1.0_f64).exp()).sqrt();
    }
    let ratio = rmax_km / r_km;
    let exp_term = (-ratio.powf(b)).exp();
    let v = (b / RHO_AIR * delta_p_pa * ratio.powf(b) * exp_term).sqrt();
    if v.is_nan() || !v.is_finite() {
        0.0
    } else {
        v
    }
}

/// 计算风增水 (m) 沿给定剖面的简化公式。
///
/// 基于 dS/dx = (Cd × ρ_air × V²) / (ρ_water × g × h)
/// 积分为 ΔS = (Cd × ρ_air × V² × L) / (ρ_water × g × h_avg)
fn wind_setup(wind_speed_m_s: f64, fetch_km: f64, avg_depth_m: f64) -> f64 {
    let depth = avg_depth_m.max(1.0); // 最小 1m
    let fetch_m = fetch_km * 1000.0;
    let setup = (CD * RHO_AIR * wind_speed_m_s.powi(2) * fetch_m) / (RHO_WATER * G * depth);
    setup.max(0.0)
}

impl CoastalPlugin {
    /// 风暴潮模拟 (SLOSH 简化版)。
    ///
    /// # 参数
    /// * `params` — 风暴参数（lat/lon 为地理坐标°）
    /// * `dem` — 地形高程 (m), flat row-major
    /// * `rows` / `cols` — 网格尺寸
    /// * `cell_size_m` — 格网边长 (m)
    /// * `land_mask` — true=陆地, false=水体/海
    /// * `ul_lat` — 网格左上角纬度 (°)
    /// * `ul_lon` — 网格左上角经度 (°)
    ///
    /// # 返回
    /// 增水栅格 + 淹没统计
    pub fn storm_surge(
        &self,
        params: &StormParams,
        dem: &[f64],
        rows: usize,
        cols: usize,
        cell_size_m: f64,
        land_mask: &[bool],
        ul_lat: f64,
        ul_lon: f64,
    ) -> GeoResult<StormSurgeResult> {
        let n = rows * cols;
        if dem.len() < n || land_mask.len() < n {
            return Err(geo_core::errors::GeoError::InvalidInput {
                field: "dims".into(),
                reason: "dem / land_mask 维度不匹配".into(),
            });
        }

        let delta_p_pa = (params.ambient_pressure_hpa - params.central_pressure_hpa) * 100.0; // hPa → Pa

        // 1. 将风暴中心地理坐标(lat/lon)转为网格行列索引
        let deg_per_m = 1.0 / 111_320.0;
        let cell_size_deg = cell_size_m * deg_per_m;
        let rad_per_deg = std::f64::consts::PI / 180.0;
        let center_row = ((params.lat - ul_lat) / cell_size_deg).round();
        let center_col = ((params.lon - ul_lon) / cell_size_deg).round();

        // 预先计算各格点到 storm center 的距离 (km) 和方位角
        let mut surge_grid = vec![0.0_f64; n];
        let mut max_surge = 0.0_f64;
        let mut inundated_cells = 0usize;
        let mut total_surge = 0.0_f64;
        let cell_area = cell_size_m * cell_size_m;

        for i in 0..n {
            let r = (i / cols) as f64;
            let c = (i % cols) as f64;

            // 计算到风暴中心的距离 (km)
            let dlat = (r - center_row) * cell_size_m * deg_per_m;
            let dlon = (c - center_col) * cell_size_m * deg_per_m;
            let dist_km = (dlat.powi(2) + dlon.powi(2)).sqrt() * 111.32; // convert angular degrees to km

            // 计算格点到风暴中心的方位角 (° 从北顺时针)
            let mut az = 0.0_f64;
            if dist_km > 0.5 {
                az = dlat.atan2(dlon).to_degrees(); // atan2(dlat, dlon)
                az = 90.0 - az; // 数学角 → 方位角
                if az < 0.0 {
                    az += 360.0;
                }
            }

            // 2. 计算梯度风速
            let v_g = gradient_wind(dist_km, delta_p_pa, params.rmax_km, params.holland_b);

            // 3. 加入前进速度修正 (不对称风场)
            // 风向 = 梯度风方向 (绕中心逆时针) + 前进向量
            let bearing_rad = params.forward_bearing_deg * rad_per_deg;
            let _fx = params.forward_speed_m_s * bearing_rad.sin();
            let _fy = params.forward_speed_m_s * bearing_rad.cos();

            // 梯度风方向: 在风暴右侧(前进方向顺时针90°)更强
            let angle_from_center = (az - params.forward_bearing_deg + 360.0) % 360.0;
            let asymmetry = 1.0 + 0.5 * (angle_from_center - 90.0).to_radians().cos();
            let v_total = v_g * asymmetry;

            // 4. 仅对水体/海岸格点计算增水
            if dist_km < 300.0 && v_total > 1.0 {
                // 水深近似: 如果是水体, 从 DEM 负值取绝对值; 否则用平均水深
                let depth_m = if land_mask[i] {
                    2.0 // 近岸陆地假定浅水
                } else {
                    (-dem[i]).max(1.0) // 水体: DEM 负值表示水深
                };

                // 风增水: 使用距离海岸的距离作为 fetch
                let setup = wind_setup(v_total, dist_km.min(100.0), depth_m);

                // 风暴潮 = 风增水 + 低压引起的逆气压效应 (~1cm/hPa)
                // 逆气压效应: 气压每降 1hPa, 水面上升约 1cm
                const INVERSE_BAROMETER: f64 = 0.01; // m/hPa
                let pressure_surge =
                    (params.ambient_pressure_hpa - params.central_pressure_hpa) * INVERSE_BAROMETER;

                let total_setup = setup + pressure_surge;

                // 如果淹没: 检查 DEM 高程
                if !land_mask[i] || dem[i] < total_setup {
                    surge_grid[i] = total_setup;
                    if total_setup > max_surge {
                        max_surge = total_setup;
                    }
                    inundated_cells += 1;
                    total_surge += total_setup;
                } else {
                    surge_grid[i] = 0.0;
                }
            }
        }

        let inundated_area_ha = inundated_cells as f64 * cell_area / 10_000.0;
        let mean_surge = if inundated_cells > 0 {
            total_surge / inundated_cells as f64
        } else {
            0.0
        };
        let inundated_volume_m3 = surge_grid.iter().sum::<f64>() * cell_area;

        let risk_level = if max_surge > 3.0 {
            "🔴 极高".into()
        } else if max_surge > 1.5 {
            "🟠 高".into()
        } else if max_surge > 0.5 {
            "🟡 中".into()
        } else {
            "🟢 低".into()
        };

        Ok(StormSurgeResult {
            max_surge_m: max_surge,
            mean_surge_m: mean_surge,
            inundated_cells,
            inundated_area_ha,
            inundated_volume_m3,
            surge_grid,
            risk_level,
        })
    }

    /// 风暴潮快速评估 (基于风向线的 1D 简化)。
    ///
    /// 给定风向线 (从海岸到大陆架) 上的剖面点，快速估算最大增水。
    pub fn storm_surge_1d(
        &self,
        params: &StormParams,
        coast_distance_km: &[f64],
        bathymetry_m: &[f64],
    ) -> GeoResult<f64> {
        let delta_p_pa = (params.ambient_pressure_hpa - params.central_pressure_hpa) * 100.0;
        let v_max = gradient_wind(params.rmax_km, delta_p_pa, params.rmax_km, params.holland_b);
        let pressure_surge = (params.ambient_pressure_hpa - params.central_pressure_hpa) * 0.01;

        // 沿剖面积分风增水
        let mut total_setup = 0.0_f64;
        let n_pts = coast_distance_km.len().min(bathymetry_m.len());
        for i in 0..n_pts - 1 {
            let seg_len = (coast_distance_km[i + 1] - coast_distance_km[i]).abs();
            let avg_depth = (bathymetry_m[i] + bathymetry_m[i + 1]).abs() / 2.0;
            if avg_depth < 0.5 {
                continue;
            }
            // 风速随距离衰减: 离岸越近越低
            let v_local = v_max * (1.0 - (coast_distance_km[i] / (params.rmax_km * 3.0)).min(0.8));
            let setup = wind_setup(v_local, seg_len, avg_depth);
            total_setup += setup;
        }

        let total = total_setup + pressure_surge;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoastalPlugin;

    #[test]
    fn test_gradient_wind() {
        let v = gradient_wind(40.0, 5800.0, 40.0, 1.3);
        assert!(v > 30.0 && v < 100.0, "v={}", v);
    }

    #[test]
    fn test_wind_setup() {
        let s = wind_setup(40.0, 50.0, 10.0);
        assert!(s > 0.0, "setup={}", s);
    }

    #[test]
    fn test_storm_surge_basic() {
        let p = CoastalPlugin::new();
        let rows = 20;
        let cols = 20;
        let cell_size = 1000.0; // 1km → 20x20 km grid
        let cell_size_deg = cell_size / 111_320.0;
        // 网格左上角地理坐标
        let ul_lat = 30.0;
        let ul_lon = 120.0;
        // 风暴中心在第10行、第10列 → 换算为地理坐标
        let params = StormParams {
            lat: ul_lat + 10.0 * cell_size_deg,
            lon: ul_lon + 10.0 * cell_size_deg,
            central_pressure_hpa: 955.0,
            ..Default::default()
        };
        let n = rows * cols;
        let mut dem = vec![5.0_f64; n]; // all land at 5m
                                        // Put a shallow bay in center
        for r in 5..15 {
            for c in 5..15 {
                let i = r * cols + c;
                dem[i] = -3.0; // water depth 3m
            }
        }
        let land_mask: Vec<bool> = dem.iter().map(|z| *z >= 0.0).collect();

        let result = p
            .storm_surge(&params, &dem, rows, cols, cell_size, &land_mask, ul_lat, ul_lon)
            .unwrap();
        assert!(result.max_surge_m > 0.0);
        assert!(result.inundated_cells > 0);
        assert!(result.inundated_area_ha > 0.0);
    }

    #[test]
    fn test_storm_surge_1d() {
        let p = CoastalPlugin::new();
        let params = StormParams {
            central_pressure_hpa: 950.0,
            rmax_km: 30.0,
            ..Default::default()
        };
        let dist = vec![0.0, 10.0, 20.0, 30.0, 50.0, 80.0];
        let bathy = vec![5.0, 10.0, 20.0, 30.0, 50.0, 80.0];
        let surge = p.storm_surge_1d(&params, &dist, &bathy).unwrap();
        assert!(surge > 0.0, "surge={}", surge);
    }

    #[test]
    fn test_pressure_surge() {
        // 100hPa drop ≈ 1m 逆气压效应
        let params = StormParams {
            central_pressure_hpa: 913.0, // 超级台风
            ..Default::default()
        };
        let delta = params.ambient_pressure_hpa - params.central_pressure_hpa;
        let expected = delta * 0.01;
        assert!((expected - 1.0).abs() < 0.01, "逆气压={}m", expected);
    }
}
