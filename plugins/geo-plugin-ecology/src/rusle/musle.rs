/// 计算 MUSLE（Modified Universal Soil Loss Equation）— 单场暴雨产沙量。
///
/// MUSLE 用径流因子替代 RUSLE 的降雨侵蚀力 R 因子，
/// 适用于单场暴雨的泥沙产量估算。
/// `Y = 11.8 × (Q × q_p)^0.56 × K × LS × C × P`
///
/// 其中 `Q` 为径流深 (mm，来自 SCS-CN)，`q_p` 为洪峰流量 (mm/h)，
/// K/LS/C/P 与 RUSLE 相同。返回 t/ha（吨/公顷）。
///
/// **来源**: Williams, J.R. (1975).
/// "Sediment-yield prediction with Universal Equation using runoff energy factor".
/// USDA-ARS, ARS-S-40, pp. 244-252.
///
/// **注意**: MUSLE 是事件模型，适用于单次暴雨。
/// 常数 11.8 将单位转换为 (t·ha⁻¹)。
pub fn compute_musle_sediment(
    runoff_depth_mm: &[f64],
    peak_runoff_rate_mm_h: &[f64],
    k_factor: &[f64],
    ls_factor: &[f64],
    c_factor: &[f64],
    p_factor: &[f64],
    cells: usize,
) -> Vec<f64> {
    let mut sediment = vec![0.0; cells];
    for (i, s) in sediment.iter_mut().enumerate() {
        let q = runoff_depth_mm.get(i).copied().unwrap_or(0.0);
        let qp = peak_runoff_rate_mm_h.get(i).copied().unwrap_or(0.0);
        let k = k_factor.get(i).copied().unwrap_or(0.0);
        let ls = ls_factor.get(i).copied().unwrap_or(0.0);
        let c = c_factor.get(i).copied().unwrap_or(0.0);
        let p = p_factor.get(i).copied().unwrap_or(0.0);

        if q <= 0.0 || qp <= 0.0 {
            continue;
        }
        let energy = (q * qp).powf(0.56);
        *s = 11.8 * energy * k * ls * c * p;
    }
    sediment
}
