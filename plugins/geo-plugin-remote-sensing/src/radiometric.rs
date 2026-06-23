//! 辐射校正模块 — TOA 反射率、暗目标减法大气校正、云检测。
use serde::{Deserialize, Serialize};

/// 辐射校正结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiometricResult {
    /// 波段数
    pub bands: usize,
    /// TOA 辐射亮度 (W/m²/sr/μm), per pixel per band
    pub toa_radiance: Vec<Vec<f64>>,
    /// TOA 反射率 (0-1), per pixel per band
    pub toa_reflectance: Vec<Vec<f64>>,
    /// 大气校正后地表反射率 (0-1), per pixel per band
    pub surface_reflectance: Vec<Vec<f64>>,
    /// 云掩膜 (true = 云)
    pub cloud_mask: Vec<bool>,
    /// 暗目标像素索引
    pub dark_object_indices: Vec<usize>,
    /// 各波段 DOS 偏移值
    pub dos_offsets: Vec<f64>,
}

/// 计算 TOA 辐射亮度: L = gain × DN + bias
pub fn toa_radiance(dn: &[Vec<f64>], gain: &[f64], bias: &[f64]) -> Vec<Vec<f64>> {
    dn.iter()
        .enumerate()
        .map(|(b, band)| {
            let g = gain.get(b).copied().unwrap_or(0.1);
            let o = bias.get(b).copied().unwrap_or(0.0);
            band.iter().map(|&v| v * g + o).collect()
        })
        .collect()
}

/// 计算 TOA 反射率: ρ = π × L × d² / (ESUN × cos(θ))
pub fn toa_reflectance(
    radiance: &[Vec<f64>],
    sun_elevation_deg: f64,
    sun_earth_distance_au: f64,
) -> Vec<Vec<f64>> {
    let cos_theta = sun_elevation_deg.to_radians().sin();
    let d2 = sun_earth_distance_au * sun_earth_distance_au;
    let esun = [1890.0, 1960.0, 1820.0, 1510.0, 1130.0, 950.0, 240.0];
    radiance
        .iter()
        .enumerate()
        .map(|(b, band)| {
            let e = esun.get(b).copied().unwrap_or(1500.0);
            band.iter()
                .map(|&l| {
                    let rho = std::f64::consts::PI * l * d2 / (e * cos_theta);
                    rho.clamp(0.0, 1.0)
                })
                .collect()
        })
        .collect()
}

/// 暗目标减法 (DOS) 大气校正。
pub fn dos_correction(
    toa_ref: &[Vec<f64>],
    dark_pct: f64,
    gain: &[f64],
    bias: &[f64],
) -> (Vec<Vec<f64>>, Vec<f64>, Vec<usize>) {
    let n_bands = toa_ref.len();
    if n_bands == 0 {
        return (vec![], vec![], vec![]);
    }
    let n_pixels = toa_ref[0].len();
    let n_dark = ((n_pixels as f64) * dark_pct).max(1.0) as usize;

    let mut dos_offsets = Vec::with_capacity(n_bands);
    let mut all_dark_indices = Vec::new();

    for b in 0..n_bands {
        let mut indices: Vec<(usize, f64)> = toa_ref[b].iter().copied().enumerate().collect();
        indices.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let dark_indices: Vec<usize> = indices.iter().take(n_dark).map(|(i, _)| *i).collect();
        let dark_mean: f64 =
            dark_indices.iter().map(|&i| toa_ref[b][i]).sum::<f64>() / dark_indices.len() as f64;
        let dn_dark = gain.get(b).copied().unwrap_or(0.1);
        let haze = if dn_dark > 0.0 {
            (dark_mean - 0.01 / dn_dark).max(0.0)
        } else {
            0.0
        };
        dos_offsets.push(haze);
        if b == 0 {
            all_dark_indices = dark_indices;
        }
    }

    let surface_ref: Vec<Vec<f64>> = toa_ref
        .iter()
        .enumerate()
        .map(|(b, band)| {
            let offset = dos_offsets.get(b).copied().unwrap_or(0.0);
            band.iter().map(|&v| (v - offset).max(0.0)).collect()
        })
        .collect();

    (surface_ref, dos_offsets, all_dark_indices)
}

#[allow(unused_variables)]
pub fn quick_atmospheric_correction(
    toa_ref: &[Vec<f64>],
    sun_elevation_deg: f64,
    dark_pct: f64,
    gain: &[f64],
    bias: &[f64],
) -> Vec<Vec<f64>> {
    let (surface, _, _) = dos_correction(toa_ref, dark_pct, gain, bias);
    surface
}

/// 云检测 — 基于 NDVI + 亮度阈值。
pub fn cloud_mask(
    red_band: &[f64],
    nir_band: &[f64],
    ndvi_threshold: f64,
    brightness_threshold: f64,
) -> Vec<bool> {
    let n = red_band.len().min(nir_band.len());
    (0..n)
        .map(|i| {
            let r = red_band[i];
            let nir = nir_band[i];
            let ndvi = if r + nir > 0.0 {
                (nir - r) / (nir + r)
            } else {
                0.0
            };
            ndvi < ndvi_threshold && nir > brightness_threshold
        })
        .collect()
}

/// 完整辐射校正管线：DN -> TOA 辐射亮度 -> TOA 反射率 -> 大气校正 -> 云掩膜。
pub fn full_radiometric_pipeline(
    dn_bands: &[Vec<f64>],
    gain: &[f64],
    bias: &[f64],
    sun_elevation_deg: f64,
    sun_earth_distance_au: f64,
    dark_pct: f64,
    cloud_ndvi_threshold: f64,
    red_band_idx: usize,
    nir_band_idx: usize,
) -> RadiometricResult {
    let bands = dn_bands.len();
    let toa_rad = toa_radiance(dn_bands, gain, bias);
    let toa_ref = toa_reflectance(&toa_rad, sun_elevation_deg, sun_earth_distance_au);
    let (surface_ref, dos_offsets, dark_indices) = dos_correction(&toa_ref, dark_pct, gain, bias);

    let cloud = if red_band_idx < bands && nir_band_idx < bands {
        cloud_mask(
            &surface_ref[red_band_idx],
            &surface_ref[nir_band_idx],
            cloud_ndvi_threshold,
            0.3,
        )
    } else {
        vec![false; dn_bands.first().map(|b| b.len()).unwrap_or(0)]
    };

    RadiometricResult {
        bands,
        toa_radiance: toa_rad,
        toa_reflectance: toa_ref,
        surface_reflectance: surface_ref,
        cloud_mask: cloud,
        dark_object_indices: dark_indices,
        dos_offsets,
    }
}

/// 从校正后反射率计算 NDVI。
pub fn ndvi_from_reflectance(red: &[f64], nir: &[f64], mask: Option<&[bool]>) -> Vec<Option<f64>> {
    let n = red.len().min(nir.len());
    (0..n)
        .map(|i| {
            let r = red[i];
            let nir = nir[i];
            if let Some(m) = mask {
                if i < m.len() && m[i] {
                    return None;
                }
            }
            if r + nir > 0.0 {
                Some(((nir - r) / (nir + r) * 10000.0).round() / 10000.0)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toa_radiance() {
        let dn = vec![vec![100.0, 200.0], vec![150.0, 250.0]];
        let gain = vec![0.1, 0.08];
        let bias = vec![-0.5, -0.3];
        let rad = toa_radiance(&dn, &gain, &bias);
        assert!((rad[0][0] - 9.5).abs() < 1e-6);
        assert!((rad[1][1] - 19.7).abs() < 1e-6);
    }

    #[test]
    fn test_toa_reflectance() {
        let rad = vec![vec![10.0; 3]];
        let ref_ = toa_reflectance(&rad, 50.0, 1.0);
        assert!(ref_[0][0] > 0.01);
        assert!(ref_[0][0] <= 1.0);
    }

    #[test]
    fn test_dos_correction() {
        let toa_ref = vec![vec![0.1, 0.9, 0.15, 0.85, 0.08, 0.95]];
        let gain = vec![0.1];
        let bias = vec![-0.5];
        let (surface, offsets, dark_idx) = dos_correction(&toa_ref, 0.3, &gain, &bias);
        assert!(offsets[0] >= 0.0);
        assert_eq!(dark_idx.len(), 1);
        assert!(surface[0].iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn test_cloud_mask() {
        let red = vec![0.05, 0.02, 0.3, 0.1];
        let nir = vec![0.04, 0.01, 0.15, 0.5];
        let cm = cloud_mask(&red, &nir, 0.2, 0.08);
        assert!(!cm[0]); // NDVI < 0.2 but nir=0.04 < brightness 0.08 -> not cloud
        assert!(!cm[1]); // both too dim
        assert!(cm[2]); // NDVI negative + bright
    }

    #[test]
    fn test_ndvi_from_reflectance() {
        let red = vec![0.1, 0.3, 0.02];
        let nir = vec![0.5, 0.1, 0.08];
        let ndvi = ndvi_from_reflectance(&red, &nir, None);
        assert!(ndvi[0].unwrap() > 0.6);
        assert!(ndvi[1].unwrap() < 0.0);
        assert!(ndvi[2].unwrap() > 0.5);
    }

    #[test]
    fn test_ndvi_with_cloud_mask() {
        let red = vec![0.1, 0.2, 0.3];
        let nir = vec![0.5, 0.1, 0.4];
        let mask = vec![false, true, false];
        let ndvi = ndvi_from_reflectance(&red, &nir, Some(&mask));
        assert!(ndvi[0].is_some());
        assert!(ndvi[1].is_none());
        assert!(ndvi[2].is_some());
    }

    #[test]
    fn test_full_pipeline() {
        let dn = vec![
            vec![80.0; 10],  // band 1
            vec![70.0; 10],  // band 2
            vec![60.0; 10],  // band 3
            vec![50.0; 10],  // band 4 (red)
            vec![200.0; 10], // band 5 (NIR)
        ];
        let result = full_radiometric_pipeline(
            &dn,
            &[0.1, 0.08, 0.06, 0.05, 0.04],
            &[-0.5, -0.3, -0.2, -0.1, 0.0],
            50.0,
            1.0,
            0.1,
            0.2,
            3,
            4,
        );
        assert_eq!(result.bands, 5);
        assert!(!result.cloud_mask.is_empty());
        assert!(!result.dos_offsets.is_empty());
    }
}
