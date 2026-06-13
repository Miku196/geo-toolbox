use crate::config::SurveyConfig;
use serde::{Deserialize, Serialize};

/// 网格法土方计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarthworkResult {
    /// 挖方量（m³）。
    pub cut_volume_m3: f64,
    /// 填方量（m³，松土体积）。
    pub fill_volume_m3: f64,
    /// 压实后填方量（m³）。
    pub compacted_fill_m3: f64,
    /// 净土方量（+ = 盈余，- = 不足）。
    pub net_volume_m3: f64,
    /// 网格尺寸（m）。
    pub grid_size_m: f64,
    /// 单元格数。
    pub cell_count: usize,
}

/// 断面法土方计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSectionResult {
    pub cut_volume_m3: f64,
    pub fill_volume_m3: f64,
    pub total_length_m: f64,
    pub section_count: usize,
}

/// 平差结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustmentResult {
    /// 收敛标志。
    pub converged: bool,
    /// 迭代次数。
    pub iterations: u32,
    /// 平差后坐标。
    pub adjusted_points: Vec<[f64; 2]>,
    /// 单位权中误差(m)。
    pub rmse_m: f64,
}

/// 高程点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevationPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// 综合测绘评估。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyAssessment {
    pub earthwork: Option<EarthworkResult>,
    pub adjustment: Option<AdjustmentResult>,
    pub cross_section: Option<CrossSectionResult>,
}

// ════════════════════════════════════════════════════════════════

pub struct SurveyPlugin {
    config: SurveyConfig,
}

impl SurveyPlugin {
    pub fn new(config: SurveyConfig) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &SurveyConfig {
        &self.config
    }

    // ── 网格法 ──
    pub fn grid_earthwork(
        &self,
        existing_elevation: &[f64],
        design_elevation: f64,
        _grid_cols: usize,
        _grid_rows: usize,
    ) -> EarthworkResult {
        let gs = self.config.earthwork.grid_size_m;
        let cell_area = gs * gs;
        let mut cut = 0.0_f64;
        let mut fill = 0.0_f64;
        let cell_count = existing_elevation.len();

        for &elev in existing_elevation.iter() {
            let diff = design_elevation - elev;
            if diff < 0.0 {
                cut += diff.abs() * cell_area;
            } else {
                fill += diff * cell_area;
            }
        }

        let loose = self.config.earthwork.loose_factor;
        let compacted = fill / loose;
        let net = cut - compacted;

        EarthworkResult {
            cut_volume_m3: cut,
            fill_volume_m3: fill,
            compacted_fill_m3: compacted,
            net_volume_m3: net,
            grid_size_m: gs,
            cell_count,
        }
    }

    /// 简化的多边块土方量（面积×高差），用于非规则区域。
    pub fn polygonal_earthwork(&self, polygons: &[(f64, f64)]) -> f64 {
        polygons
            .iter()
            .map(|(area_ha, height_diff_m)| area_ha * 10000.0 * height_diff_m)
            .sum::<f64>()
            .abs()
    }

    // ── 断面法 ──
    pub fn cross_section_earthwork(
        &self,
        sections: &[(f64, f64)], // (cut_area_m2, fill_area_m2)
        distances: &[f64],       // distance between sections (len = sections.len() - 1)
    ) -> CrossSectionResult {
        let mut cut_vol = 0.0_f64;
        let mut fill_vol = 0.0_f64;
        let n = sections.len();
        if n < 2 || distances.len() < n - 1 {
            return CrossSectionResult {
                cut_volume_m3: 0.0,
                fill_volume_m3: 0.0,
                total_length_m: 0.0,
                section_count: n,
            };
        }
        for i in 0..n - 1 {
            let dist = distances[i];
            // 平均断面法：V = (A_i + A_{i+1}) / 2 * dist
            cut_vol += (sections[i].0 + sections[i + 1].0) / 2.0 * dist;
            fill_vol += (sections[i].1 + sections[i + 1].1) / 2.0 * dist;
        }
        let total_len: f64 = distances.iter().sum();
        CrossSectionResult {
            cut_volume_m3: cut_vol,
            fill_volume_m3: fill_vol,
            total_length_m: total_len,
            section_count: n,
        }
    }

    // ── TIN 法：三角形棱柱体积 ──
    pub fn tin_earthwork(&self, points: &[ElevationPoint], design_z: f64) -> f64 {
        // 每 3 个点组成三角形棱柱
        if points.len() < 3 {
            return 0.0;
        }
        let n_tri = points.len() / 3;
        let mut volume = 0.0_f64;
        for i in 0..n_tri {
            let idx = i * 3;
            let a = &points[idx];
            let b = &points[idx + 1];
            let c = &points[idx + 2];
            // 三角形面积 (Shoelace 2D)
            let area = 0.5 * ((a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)).abs());
            let avg_z = (a.z + b.z + c.z) / 3.0;
            volume += area * (design_z - avg_z).abs();
        }
        volume
    }

    // ── 控制网平差（简化最小二乘 1D） ──
    pub fn control_network_adjustment(
        &self,
        observations: &[(f64, f64)], // (measured, weight)
        initial: f64,
    ) -> AdjustmentResult {
        let max_iter = self.config.adjustment.max_iterations;
        let threshold = self.config.adjustment.convergence_threshold;
        let mut x = initial;
        let mut converged = false;
        let mut iter = 0;

        for _i in 0..max_iter {
            iter = _i + 1;
            let mut sum_wr = 0.0_f64;
            let mut sum_w = 0.0_f64;
            for (obs, w) in observations {
                let residual = obs - x;
                sum_wr += w * residual;
                sum_w += w;
            }
            if sum_w == 0.0 {
                break;
            }
            let dx = sum_wr / sum_w;
            if dx.abs() < threshold {
                converged = true;
                break;
            }
            x += dx;
        }

        let mut sq_sum = 0.0_f64;
        let mut w_sum = 0.0_f64;
        for (obs, w) in observations {
            let r = obs - x;
            sq_sum += w * r * r;
            w_sum += w;
        }
        let rmse = if w_sum > 0.0 {
            (sq_sum / w_sum).sqrt()
        } else {
            0.0
        };

        AdjustmentResult {
            converged,
            iterations: iter,
            adjusted_points: vec![[x, 0.0]],
            rmse_m: rmse,
        }
    }

    // ── 综合评估 ──
    pub fn assess(
        &self,
        existing_elevation: &[f64],
        design_elevation: f64,
        grid_cols: usize,
        grid_rows: usize,
    ) -> SurveyAssessment {
        let ew = self.grid_earthwork(existing_elevation, design_elevation, grid_cols, grid_rows);
        SurveyAssessment {
            earthwork: Some(ew),
            adjustment: None,
            cross_section: None,
        }
    }
}

// ═══ 测试 ═══
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PluginMeta, SurveyConfig};

    fn default_plugin() -> SurveyPlugin {
        SurveyPlugin::new(SurveyConfig {
            plugin: PluginMeta {
                name: "survey".into(),
                version: "0.2".into(),
                description: "test".into(),
            },
            adjustment: Default::default(),
            earthwork: Default::default(),
            contour: Default::default(),
        })
    }

    #[test]
    fn test_grid_earthwork() {
        let p = default_plugin();
        // 2x2 grid, design=10, actual=[9,11,8,12]
        let elev = vec![9.0, 11.0, 8.0, 12.0];
        let r = p.grid_earthwork(&elev, 10.0, 2, 2);
        assert!(r.cut_volume_m3 > 0.0);
        assert!(r.fill_volume_m3 > 0.0);
        assert_eq!(r.cell_count, 4);
        // cell_area=100m², cut: (11-10)*100 + (12-10)*100 = 300
        // fill: (10-9)*100 + (10-8)*100 = 300
        // compacted fill = 300 / 1.15 ≈ 260.87
        // net = 300 - 260.87 = 39.13
        assert!((r.cut_volume_m3 - 300.0).abs() < 0.1);
        assert!((r.fill_volume_m3 - 300.0).abs() < 0.1);
        assert!((r.compacted_fill_m3 - 260.87).abs() < 1.0);
        assert!(r.net_volume_m3 > 0.0);
    }

    #[test]
    fn test_polygonal_earthwork() {
        let p = default_plugin();
        let v = p.polygonal_earthwork(&[(1.0, 2.0), (0.5, -1.0)]);
        assert!((v - 15000.0).abs() < 1.0);
    }

    #[test]
    fn test_cross_section() {
        let p = default_plugin();
        let sections = vec![(10.0, 5.0), (12.0, 3.0), (8.0, 6.0)];
        let distances = vec![20.0, 15.0];
        let r = p.cross_section_earthwork(&sections, &distances);
        assert!((r.cut_volume_m3 - 370.0).abs() < 1.0);
        assert!((r.fill_volume_m3 - 147.5).abs() < 1.0);
        assert!((r.total_length_m - 35.0).abs() < 0.1);
    }

    #[test]
    fn test_tin_earthwork() {
        let p = default_plugin();
        let pts = vec![
            ElevationPoint {
                x: 0.0,
                y: 0.0,
                z: 5.0,
            },
            ElevationPoint {
                x: 10.0,
                y: 0.0,
                z: 4.0,
            },
            ElevationPoint {
                x: 0.0,
                y: 10.0,
                z: 6.0,
            },
        ];
        let v = p.tin_earthwork(&pts, 10.0);
        assert!(v > 0.0);
    }

    #[test]
    fn test_adjustment() {
        let p = default_plugin();
        let obs = vec![(100.1, 1.0), (99.9, 1.0), (100.0, 2.0)];
        let r = p.control_network_adjustment(&obs, 100.0);
        assert!(r.converged);
        assert!((r.adjusted_points[0][0] - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_assess() {
        let p = default_plugin();
        let elev = vec![9.0, 11.0, 8.0, 12.0];
        let a = p.assess(&elev, 10.0, 2, 2);
        assert!(a.earthwork.is_some());
    }
}
