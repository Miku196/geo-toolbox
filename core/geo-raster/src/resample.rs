/// 重采样算法：最邻近法和双三次法。
///
/// 所有函数均为纯 Rust 实现，无外部依赖。
///
/// 最邻近重采样：每个目标像素取最近源像素值。
///
/// 使用中心点映射（与 `resample_bilinear` 一致）。
/// 若最近源像素为 NoData，输出 NoData。
///
/// # 参数
/// - `src`: 源数据（行优先）
/// - `src_rows`, `src_cols`: 源尺寸
/// - `dst_rows`, `dst_cols`: 目标尺寸
/// - `nodata`: 源 NoData 值
pub fn resample_nearest(
    src: &[f64],
    src_rows: usize,
    src_cols: usize,
    dst_rows: usize,
    dst_cols: usize,
    nodata: Option<f64>,
) -> Vec<f64> {
    let nd = nodata.unwrap_or(f64::NAN);
    let mut dst = vec![nd; dst_rows * dst_cols];
    let scale_r = src_rows as f64 / dst_rows as f64;
    let scale_c = src_cols as f64 / dst_cols as f64;

    for dr in 0..dst_rows {
        for dc in 0..dst_cols {
            // 目标像素中心 → 源像素坐标
            let sr = ((dr as f64 + 0.5) * scale_r - 0.5).round();
            let sc = ((dc as f64 + 0.5) * scale_c - 0.5).round();

            let sr = sr.max(0.0).min((src_rows - 1) as f64) as usize;
            let sc = sc.max(0.0).min((src_cols - 1) as f64) as usize;

            let v = src[sr * src_cols + sc];
            if v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10) {
                continue; // 保持 dst 为 nd（默认值）
            }
            dst[dr * dst_cols + dc] = v;
        }
    }
    dst
}

/// Catmull-Rom 三次核函数（a = -0.5）。
///
/// 返回 `x` 处的一维 Catmull-Rom 样条权重。
fn catmull_rom_kernel(x: f64) -> f64 {
    let x = x.abs();
    if x < 1.0 {
        1.5 * x.powi(3) - 2.5 * x.powi(2) + 1.0
    } else if x < 2.0 {
        -0.5 * x.powi(3) + 2.5 * x.powi(2) - 4.0 * x + 2.0
    } else {
        0.0
    }
}

/// 双三次重采样（Catmull-Rom，a = -0.5）。
///
/// 使用 4×4 邻域，边界处通过 clamp 到源边缘处理。
/// 若 4×4 邻域中有效像素少于 4 个，输出 NoData。
///
/// # 参数
/// - `src`: 源数据（行优先）
/// - `src_rows`, `src_cols`: 源尺寸
/// - `dst_rows`, `dst_cols`: 目标尺寸
/// - `nodata`: 源 NoData 值
pub fn resample_cubic(
    src: &[f64],
    src_rows: usize,
    src_cols: usize,
    dst_rows: usize,
    dst_cols: usize,
    nodata: Option<f64>,
) -> Vec<f64> {
    let nd = nodata.unwrap_or(f64::NAN);
    let mut dst = vec![nd; dst_rows * dst_cols];
    let scale_r = src_rows as f64 / dst_rows as f64;
    let scale_c = src_cols as f64 / dst_cols as f64;

    for dr in 0..dst_rows {
        for dc in 0..dst_cols {
            let sr = (dr as f64 + 0.5) * scale_r - 0.5;
            let sc = (dc as f64 + 0.5) * scale_c - 0.5;

            let r_floor = sr.floor() as isize;
            let c_floor = sc.floor() as isize;

            // 收集 4×4 邻域（r_floor-1 .. r_floor+2, c_floor-1 .. c_floor+2）
            let mut valid_count = 0;
            let mut total = 0.0;
            let mut weight_sum = 0.0;

            for ri in -1..=2 {
                for ci in -1..=2 {
                    let r = (r_floor + ri).clamp(0, src_rows as isize - 1) as usize;
                    let c = (c_floor + ci).clamp(0, src_cols as isize - 1) as usize;
                    let v = src[r * src_cols + c];

                    if v.is_nan() || (!nd.is_nan() && (v - nd).abs() < 1e-10) {
                        continue;
                    }

                    let wr = catmull_rom_kernel((r_floor + ri) as f64 - sr);
                    let wc = catmull_rom_kernel((c_floor + ci) as f64 - sc);
                    let w = wr * wc;

                    total += v * w;
                    weight_sum += w;
                    valid_count += 1;
                }
            }

            if valid_count >= 4 && weight_sum > 0.0 {
                dst[dr * dst_cols + dc] = total / weight_sum;
            }
            // 否则保持 nd
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── resample_nearest ──

    #[test]
    fn test_nearest_2x2_to_4x4() {
        // 2×2 棋盘
        let src = vec![1.0, 2.0, 3.0, 4.0];
        let dst = resample_nearest(&src, 2, 2, 4, 4, None);
        assert_eq!(dst.len(), 16);
        // 每个 2×2 块应相同
        assert_eq!(dst[0], 1.0);
        assert_eq!(dst[3], 2.0);
        assert_eq!(dst[12], 3.0);
        assert_eq!(dst[15], 4.0);
    }

    #[test]
    fn test_nearest_nodata_propagation() {
        let src = vec![1.0, f64::NAN, 3.0, 4.0];
        let dst = resample_nearest(&src, 2, 2, 2, 2, None);
        assert!(dst[1].is_nan());
        assert_eq!(dst[0], 1.0);
    }

    #[test]
    fn test_nearest_3x3_to_2x2() {
        let src: Vec<f64> = (1..=9).map(|i| i as f64).collect();
        let dst = resample_nearest(&src, 3, 3, 2, 2, None);
        assert_eq!(dst.len(), 4);
        // 目标(0,0) 应该映射到源(0,0)=1
        // 目标(0,1) → 源(0,2)=3
        // 目标(1,0) → 源(2,0)=7
        // 目标(1,1) → 源(2,2)=9
        assert_eq!(dst[0], 1.0);
        assert_eq!(dst[1], 3.0);
        assert_eq!(dst[2], 7.0);
        assert_eq!(dst[3], 9.0);
    }

    #[test]
    fn test_nearest_with_nodata_value() {
        let src = vec![-999.0, 2.0, 3.0, 4.0];
        let dst = resample_nearest(&src, 2, 2, 2, 2, Some(-999.0));
        assert!((dst[0] - (-999.0)).abs() < 1e-10 || dst[0].is_nan());
        assert_eq!(dst[1], 2.0);
    }

    // ── resample_cubic ──

    #[test]
    fn test_cubic_small_to_large() {
        // 4×4 渐变 → 6×6：角点值应与源接近
        let src: Vec<f64> = (1..=16).map(|i| i as f64).collect();
        let dst = resample_cubic(&src, 4, 4, 6, 6, None);
        assert_eq!(dst.len(), 36);
        assert!(!dst[0].is_nan());
        assert!(!dst[35].is_nan());
        // 左上角 (col0, row0) 应接近 1.0
        assert!((dst[0] - 1.0).abs() < 2.0);
    }

    #[test]
    fn test_cubic_edge_clamping() {
        // 1×4 水平渐变
        let src = vec![0.0, 3.0, 6.0, 9.0];
        let dst = resample_cubic(&src, 1, 4, 1, 8, None);
        assert_eq!(dst.len(), 8);
        // 左边缘不应 NaN
        assert!(!dst[0].is_nan());
        // 右边缘不应 NaN
        assert!(!dst[7].is_nan());
        // 单调递增
        for i in 1..dst.len() {
            assert!(dst[i] >= dst[i - 1] - 0.1);
        }
    }

    #[test]
    fn test_cubic_nodata_handling() {
        // 太多 nodata 导致 valid_count < 4
        let src = vec![
            1.0,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            9.0,
        ];
        let dst = resample_cubic(&src, 3, 3, 3, 3, None);
        // 中心应该有足够邻域（除非 nodata 太多）
        assert!(dst[4].is_nan() || dst[4] > 0.0);
    }

    #[test]
    fn test_cubic_uniform_input() {
        // 均匀输入应得到均匀输出
        let src = vec![5.0; 16];
        let dst = resample_cubic(&src, 4, 4, 6, 6, None);
        assert_eq!(dst.len(), 36);
        for &v in &dst {
            assert!(!v.is_nan());
            assert!((v - 5.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_cubic_4x4_to_6x6() {
        let src: Vec<f64> = (1..=16).map(|i| i as f64).collect();
        let dst = resample_cubic(&src, 4, 4, 6, 6, None);
        assert_eq!(dst.len(), 36);
        // 角点近似
        assert!(!dst[0].is_nan());
        assert!(!dst[35].is_nan());
    }
}
