/// 栅格镶嵌算法：合并和羽化融合。
///
/// 纯 Rust 实现，无外部依赖。
use crate::grid::RasterBand;

/// 简单合并：多个同尺寸波段中，逐像素取第一个有效值。
///
/// 对所有波段同一位置的所有像素，取第一个既非 NaN 也非 nodata 的值。
/// 若所有波段在该位置均为 nodata，输出 nodata。
///
/// # 参数
/// - `bands`: 同尺寸 (`rows`, `cols`) 的波段切片。
///   若为空或尺寸不一致，返回 `None`。
pub fn mosaic_merge(bands: &[RasterBand]) -> Option<RasterBand> {
    if bands.is_empty() {
        return None;
    }
    let rows = bands[0].rows;
    let cols = bands[0].cols;
    let nodata = bands[0].nodata;

    // 检查所有波段尺寸一致
    for b in &bands[1..] {
        if b.rows != rows || b.cols != cols {
            return None;
        }
    }

    let mut out = bands[0].clone();
    out.name = "mosaic_merge".to_string();

    for i in 0..out.data.len() {
        let mut found = false;
        for b in bands {
            let v = b.data[i];
            if v.is_nan() || (!nodata.is_nan() && (v - nodata).abs() < 1e-10) {
                continue;
            }
            out.data[i] = v;
            found = true;
            break;
        }
        if !found {
            out.data[i] = nodata;
        }
    }

    Some(out)
}

/// 羽化融合：两个同尺寸波段沿列方向线性过渡融合。
///
/// 在 `blend_width` 列范围内从波段 A（左）渐变到波段 B（右），
/// 权重公式：`weight = min(col / blend_width, 1.0)`。
///
/// 若某像素仅一个波段有效，直接取该波段值。
/// 若均无效，输出 nodata。
///
/// # 参数
/// - `band_a`: 左波段（从该侧开始过渡）
/// - `band_b`: 右波段（过渡到此侧结束）
/// - `blend_width`: 过渡宽度（列数）。
///   若为 0，等效于逐像素用 band_b 覆盖 band_a。
pub fn mosaic_feather(
    band_a: &RasterBand,
    band_b: &RasterBand,
    blend_width: usize,
) -> Option<RasterBand> {
    if band_a.rows != band_b.rows || band_a.cols != band_b.cols {
        return None;
    }

    let _rows = band_a.rows;
    let cols = band_a.cols;
    let nodata = band_a.nodata;
    let len = band_a.data.len();
    let mut out = band_a.clone();
    out.name = "mosaic_feather".to_string();

    for i in 0..len {
        let v_a = band_a.data[i];
        let v_b = band_b.data[i];
        let a_valid = !v_a.is_nan() && (nodata.is_nan() || (v_a - nodata).abs() >= 1e-10);
        let b_valid = !v_b.is_nan() && (nodata.is_nan() || (v_b - nodata).abs() >= 1e-10);

        let result = match (a_valid, b_valid) {
            (false, false) => nodata,
            (true, false) => v_a,
            (false, true) => v_b,
            (true, true) => {
                if blend_width == 0 {
                    v_b
                } else {
                    let col = i % cols;
                    let weight = (col as f64 / blend_width as f64).min(1.0);
                    v_a * (1.0 - weight) + v_b * weight
                }
            }
        };
        out.data[i] = result;
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_band(name: &str, rows: usize, cols: usize, data: Vec<f64>, nodata: f64) -> RasterBand {
        RasterBand::new(name, rows, cols, data, nodata)
    }

    // ── mosaic_merge ──

    #[test]
    fn test_merge_2bands_first_valid_wins() {
        let a = make_band("a", 2, 2, vec![1.0, 2.0, f64::NAN, 4.0], -999.0);
        let b = make_band("b", 2, 2, vec![10.0, f64::NAN, 30.0, 40.0], -999.0);
        let result = mosaic_merge(&[a, b]).unwrap();
        assert_eq!(result.data[0], 1.0); // a 有效
        assert_eq!(result.data[1], 2.0); // a 有效
        assert_eq!(result.data[2], 30.0); // a NA, b 有效
        assert_eq!(result.data[3], 4.0); // a 有效
    }

    #[test]
    fn test_merge_all_nodata() {
        let a = make_band("a", 1, 2, vec![f64::NAN, -999.0], -999.0);
        let b = make_band("b", 1, 2, vec![f64::NAN, -999.0], -999.0);
        let result = mosaic_merge(&[a, b]).unwrap();
        assert!(result.data[0].is_nan() || (result.data[0] + 999.0).abs() < 1e-10);
        assert!((result.data[1] + 999.0).abs() < 1e-10 || result.data[1].is_nan());
    }

    #[test]
    fn test_merge_size_mismatch() {
        let a = make_band("a", 2, 2, vec![1.0; 4], -999.0);
        let b = make_band("b", 3, 3, vec![2.0; 9], -999.0);
        assert!(mosaic_merge(&[a, b]).is_none());
    }

    #[test]
    fn test_merge_empty() {
        assert!(mosaic_merge(&[]).is_none());
    }

    #[test]
    fn test_merge_nodata_gap_filled() {
        let a = make_band("a", 1, 3, vec![1.0, f64::NAN, 3.0], -999.0);
        let b = make_band("b", 1, 3, vec![10.0, 20.0, f64::NAN], -999.0);
        let result = mosaic_merge(&[a, b]).unwrap();
        assert_eq!(result.data[0], 1.0);
        assert_eq!(result.data[1], 20.0); // b 填补 a 的孔
        assert_eq!(result.data[2], 3.0);
    }

    // ── mosaic_feather ──

    #[test]
    fn test_feather_single_valid_only() {
        let a = make_band("a", 1, 3, vec![1.0, 2.0, 3.0], -999.0);
        let b = make_band("b", 1, 3, vec![f64::NAN, f64::NAN, f64::NAN], -999.0);
        let result = mosaic_feather(&a, &b, 3).unwrap();
        assert_eq!(result.data, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_feather_both_valid() {
        let a = make_band("a", 1, 4, vec![0.0, 0.0, 0.0, 0.0], -999.0);
        let b = make_band("b", 1, 4, vec![10.0, 10.0, 10.0, 10.0], -999.0);
        // blend_width=4: col0 weight=0, col3 weight=0.75
        let result = mosaic_feather(&a, &b, 4).unwrap();
        assert!((result.data[0] - 0.0).abs() < 1e-6); // weight=0: all a
        assert!((result.data[1] - 2.5).abs() < 1e-6); // weight=0.25: a*0.75 + b*0.25
        assert!((result.data[2] - 5.0).abs() < 1e-6); // weight=0.5: a*0.5 + b*0.5
        assert!((result.data[3] - 7.5).abs() < 1e-6); // weight=0.75: a*0.25 + b*0.75
    }

    #[test]
    fn test_feather_size_mismatch() {
        let a = make_band("a", 2, 2, vec![1.0; 4], -999.0);
        let b = make_band("b", 3, 3, vec![2.0; 9], -999.0);
        assert!(mosaic_feather(&a, &b, 3).is_none());
    }

    #[test]
    fn test_feather_zero_blend_width() {
        let a = make_band("a", 1, 3, vec![1.0, 2.0, 3.0], -999.0);
        let b = make_band("b", 1, 3, vec![10.0, 20.0, 30.0], -999.0);
        let result = mosaic_feather(&a, &b, 0).unwrap();
        assert_eq!(result.data, vec![10.0, 20.0, 30.0]); // 纯覆盖
    }
}
