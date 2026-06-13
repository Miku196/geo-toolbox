use crate::config::HydroConfig;
use serde::{Deserialize, Serialize};

/// 汇流计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAccumulation {
    /// 上游汇流格点数。
    pub flow_accum: Vec<u32>,
    /// 集水面积(ha)。
    pub catchment_area_ha: f64,
    pub rows: usize,
    pub cols: usize,
    pub cell_size_m: f64,
}

/// 径流计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunoffResult {
    /// 径流系数。
    pub runoff_coefficient: f64,
    /// 洪峰流量(m³/s)，推理公式法。
    pub peak_discharge_m3s: f64,
    /// 设计降雨强度(mm/h)。
    pub rainfall_intensity_mmh: f64,
}

/// 淹没分析结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InundationResult {
    pub inundation_area_m2: f64,
    pub avg_depth_m: f64,
    pub max_depth_m: f64,
    pub risk_zone_ha: f64,
}

/// 综合水文评估。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydroAssessment {
    pub flow: Option<FlowAccumulation>,
    pub runoff: RunoffResult,
    pub inundation: InundationResult,
}

// ════════════════════════════════════════════════════════════════

pub struct HydroPlugin {
    config: HydroConfig,
}

impl HydroPlugin {
    pub fn new(config: HydroConfig) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &HydroConfig {
        &self.config
    }

    // ── D8 汇流方向 + 集水区 ──
    pub fn flow_accumulation(
        &self,
        dem: &[f64],
        rows: usize,
        cols: usize,
        cell_size_m: f64,
    ) -> FlowAccumulation {
        let n = rows * cols;
        if dem.len() < n {
            return FlowAccumulation {
                flow_accum: vec![],
                catchment_area_ha: 0.0,
                rows,
                cols,
                cell_size_m,
            };
        }
        let threshold = self.config.catchment.slope_threshold;

        // D8 directions: NE, E, SE, S, SW, W, NW, N
        let d8_dr: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
        let d8_dc: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let d8_diag: [f64; 8] = [1.414, 1.0, 1.414, 1.0, 1.414, 1.0, 1.414, 1.0];

        let idx = |r: usize, c: usize| -> usize { r * cols + c };

        let mut flow_dir: Vec<Option<usize>> = vec![None; n]; // index of downslope neighbor
        let mut flow_accum: Vec<u32> = vec![1; n]; // each cell starts with 1 unit of flow

        for r in 0..rows {
            for c in 0..cols {
                let cur = idx(r, c);
                let mut max_slope = threshold;
                let mut best = None;
                for d in 0..8 {
                    let nr = r as isize + d8_dr[d];
                    let nc = c as isize + d8_dc[d];
                    if nr < 0 || nr >= rows as isize || nc < 0 || nc >= cols as isize {
                        continue;
                    }
                    let neighbor = idx(nr as usize, nc as usize);
                    let diff = dem[cur] - dem[neighbor];
                    let slope: f64 = diff / (cell_size_m * d8_diag[d]);
                    if slope > max_slope {
                        max_slope = slope;
                        best = Some(neighbor);
                    }
                }
                flow_dir[cur] = best;
            }
        }

        // Topological sort: sort cells by descending elevation
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            dem[b]
                .partial_cmp(&dem[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for &cell in &order {
            if let Some(down) = flow_dir[cell] {
                let acc = flow_accum[cell];
                flow_accum[down] += acc;
            }
        }

        let max_accum = *flow_accum.iter().max().unwrap_or(&1);
        let cell_area_m2 = cell_size_m * cell_size_m;
        let area_ha = max_accum as f64 * cell_area_m2 / 10000.0;

        FlowAccumulation {
            flow_accum,
            catchment_area_ha: area_ha,
            rows,
            cols,
            cell_size_m,
        }
    }

    // ── 径流系数（面积加权） ──
    pub fn runoff_coefficient(
        &self,
        impervious_ratio: f64,
        grass_ratio: f64,
        forest_ratio: f64,
    ) -> f64 {
        let c = &self.config.runoff;
        (impervious_ratio * c.impervious_c + grass_ratio * c.grass_c + forest_ratio * c.forest_c)
            .clamp(0.0, 1.0)
    }

    /// 推理公式法洪峰流量：Q = C * i * A / 3.6
    /// Q: m³/s, i: mm/h, A: ha
    pub fn peak_discharge(
        &self,
        runoff_coefficient: f64,
        rainfall_intensity_mmh: f64,
        catchment_area_ha: f64,
    ) -> RunoffResult {
        let q = runoff_coefficient * rainfall_intensity_mmh * catchment_area_ha / 3.6;
        // Cap at reasonable value
        let q = q.min(1_000_000.0);
        RunoffResult {
            runoff_coefficient,
            peak_discharge_m3s: q,
            rainfall_intensity_mmh,
        }
    }

    // ── 淹没面积 ──
    pub fn estimate_inundation_area(&self, catchment_area_ha: f64, rainfall_mm: f64) -> f64 {
        let runoff_volume = catchment_area_ha * 10000.0 * rainfall_mm / 1000.0;
        let avg_ponding_depth: f64 = 0.3; // 默认积水深度(m)
        let area = runoff_volume * self.config.flood.safety_factor / (avg_ponding_depth * 1000.0);
        area.min(catchment_area_ha * 10000.0)
    }

    /// 详细淹没分析。
    pub fn inundation_analysis(
        &self,
        dem: &[f64],
        water_volume_m3: f64,
        rows: usize,
        cols: usize,
        cell_size_m: f64,
    ) -> InundationResult {
        let n = rows * cols;
        if dem.len() < n {
            return InundationResult {
                inundation_area_m2: 0.0,
                avg_depth_m: 0.0,
                max_depth_m: 0.0,
                risk_zone_ha: 0.0,
            };
        }

        // Binary search water level
        let mut lo = dem.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let mut hi = dem.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)) + 10.0;
        let cell_area = cell_size_m * cell_size_m;

        for _ in 0..40 {
            let mid = (lo + hi) / 2.0;
            let vol: f64 = dem.iter().map(|&z| (mid - z).max(0.0)).sum::<f64>() * cell_area;
            if vol > water_volume_m3 {
                hi = mid;
            } else {
                lo = mid;
            }
        }

        let water_level = (lo + hi) / 2.0;
        let mut flooded: usize = 0;
        let mut total_depth = 0.0_f64;
        let mut max_d = 0.0_f64;
        for &z in dem {
            let d = (water_level - z).max(0.0);
            if d > 0.0 {
                flooded += 1;
                total_depth += d;
                max_d = max_d.max(d);
            }
        }
        let area = flooded as f64 * cell_area;
        let avg_d = if flooded > 0 {
            total_depth / flooded as f64
        } else {
            0.0
        };
        let risk_ha = if max_d > 1.0 { area / 10000.0 } else { 0.0 };

        InundationResult {
            inundation_area_m2: area,
            avg_depth_m: avg_d,
            max_depth_m: max_d,
            risk_zone_ha: risk_ha,
        }
    }

    // 简化的径流系数（仅用不透水率估算）。
    pub fn runoff_coefficient_simple(&self, impervious_ratio: f64) -> f64 {
        0.05 + 0.9 * impervious_ratio.min(1.0)
    }

    // ── 综合评估 ──
    pub fn assess(
        &self,
        dem: &[f64],
        rows: usize,
        cols: usize,
        cell_size_m: f64,
        impervious_ratio: f64,
        rainfall_mmh: f64,
    ) -> HydroAssessment {
        let flow = self.flow_accumulation(dem, rows, cols, cell_size_m);
        let rcoef = self.runoff_coefficient_simple(impervious_ratio);
        let runoff = self.peak_discharge(rcoef, rainfall_mmh, flow.catchment_area_ha);
        let inundation = self.inundation_analysis(
            dem,
            runoff.peak_discharge_m3s * 3600.0,
            rows,
            cols,
            cell_size_m,
        );
        HydroAssessment {
            flow: Some(flow),
            runoff,
            inundation,
        }
    }
}

// ═══ 测试 ═══
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{HydroConfig, PluginMeta};

    fn default_plugin() -> HydroPlugin {
        HydroPlugin::new(HydroConfig {
            plugin: PluginMeta {
                name: "hydro".into(),
                version: "0.2".into(),
                description: "test".into(),
            },
            flood: Default::default(),
            runoff: Default::default(),
            catchment: Default::default(),
        })
    }

    #[test]
    fn test_flow_accumulation() {
        let p = default_plugin();
        // Simple 3x3 DEM: bowl shape → all water flows to center
        let dem = vec![10.0, 10.0, 10.0, 10.0, 5.0, 10.0, 10.0, 10.0, 10.0];
        let r = p.flow_accumulation(&dem, 3, 3, 10.0);
        assert_eq!(r.flow_accum.len(), 9);
        assert!(r.catchment_area_ha > 0.0);
    }

    #[test]
    fn test_runoff_coefficient() {
        let p = default_plugin();
        let c = p.runoff_coefficient(0.6, 0.3, 0.1);
        assert!(c > 0.5 && c < 1.0);
    }

    #[test]
    fn test_peak_discharge() {
        let p = default_plugin();
        let r = p.peak_discharge(0.7, 50.0, 100.0);
        assert!(r.peak_discharge_m3s > 0.0);
    }

    #[test]
    fn test_inundation_simple() {
        let p = default_plugin();
        let a = p.estimate_inundation_area(100.0, 50.0);
        assert!(a > 0.0);
    }

    #[test]
    fn test_inundation_analysis() {
        let p = default_plugin();
        let dem = vec![10.0, 10.0, 10.0, 10.0, 1.0, 10.0, 10.0, 10.0, 10.0];
        let r = p.inundation_analysis(&dem, 1000.0, 3, 3, 10.0);
        assert!(r.inundation_area_m2 > 0.0);
    }

    #[test]
    fn test_assess() {
        let p = default_plugin();
        let dem = vec![10.0, 10.0, 10.0, 10.0, 2.0, 10.0, 10.0, 10.0, 10.0];
        let a = p.assess(&dem, 3, 3, 10.0, 0.5, 50.0);
        assert!(a.flow.is_some());
    }
}
