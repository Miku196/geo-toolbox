use crate::config::{
    BARE_FACTOR, BUILT_UP_FACTOR, CROPLAND_FACTOR, FOREST_FACTOR, GRASSLAND_FACTOR,
};
use geo_core::errors::GeoResult;
use geo_raster::grid::RasterBand;

// ── NDVI → 土地覆盖分类 ─────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LandcoverClass {
    name: String,
    area_ha: f64,
    factor: f64,
}

fn classify_pixel(ndvi: f64, ndvi_diff: f64) -> &'static str {
    if ndvi < -0.5 {
        "water"
    } else if ndvi < 0.05 {
        "bare:open_pit"
    } else if ndvi < 0.2 {
        if ndvi_diff > 0.08 {
            "bare:tailings_recovering"
        } else {
            "bare:tailings"
        }
    } else if ndvi < 0.35 {
        if ndvi_diff > 0.1 {
            "grassland:restored_shrub_grass"
        } else {
            "grassland:natural"
        }
    } else if ndvi < 0.55 {
        "forest:restored_mixed_forest"
    } else {
        "forest:evergreen_broadleaf"
    }
}

pub(crate) fn classify_to_landcover_map(
    ndvi: &RasterBand,
    ndvi_diff: &RasterBand,
) -> Vec<&'static str> {
    let n = ndvi.data.len();
    let mut labels = Vec::with_capacity(n);
    for i in 0..n {
        let v = ndvi.data[i];
        let d = ndvi_diff.data.get(i).copied().unwrap_or(0.0);
        if v == ndvi.nodata {
            labels.push("nodata");
        } else {
            labels.push(classify_pixel(v, d));
        }
    }
    labels
}

fn landcover_to_factor(class: &str) -> f64 {
    match class {
        "forest:evergreen_broadleaf" | "forest:restored_mixed_forest" => FOREST_FACTOR,
        "grassland:restored_shrub_grass" | "grassland:natural" => GRASSLAND_FACTOR,
        "bare:tailings_recovering" => GRASSLAND_FACTOR * 0.5, // 恢复中的尾矿库, 部分碳汇
        "built_up:processing_plant" | "built_up" => BUILT_UP_FACTOR,
        "cropland:paddy_field" | "cropland" => CROPLAND_FACTOR,
        "bare:open_pit" | "bare:tailings" | "bare:waste_dump" | "bare" => BARE_FACTOR,
        "water" | "nodata" => 0.0,
        _ => 0.0,
    }
}

pub(crate) fn calculate_carbon_balance(labels: &[&str]) -> GeoResult<f64> {
    let pixel_area_ha = 0.01; // 10m × 10m
    let total: f64 = labels
        .iter()
        .map(|c| landcover_to_factor(c) * pixel_area_ha)
        .sum();
    Ok(total)
}

// ── 简化统计 ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct NdviStatsSimple {
    pub(crate) mean: f64,
    pub(crate) healthy_ratio: f64,
    pub(crate) degraded_ratio: f64,
    pub(crate) valid_pixels: usize,
}

pub(crate) fn compute_ndvi_stats(ndvi: &RasterBand) -> NdviStatsSimple {
    let valid: Vec<f64> = ndvi
        .data
        .iter()
        .filter(|v| !v.is_nan() && **v != ndvi.nodata)
        .copied()
        .collect();

    let n = valid.len();
    if n == 0 {
        return NdviStatsSimple {
            mean: 0.0,
            healthy_ratio: 0.0,
            degraded_ratio: 0.0,
            valid_pixels: 0,
        };
    }

    let mean = valid.iter().sum::<f64>() / n as f64;
    let healthy = valid.iter().filter(|v| **v >= 0.5).count() as f64 / n as f64;
    let degraded = valid.iter().filter(|v| **v <= 0.2).count() as f64 / n as f64;

    NdviStatsSimple {
        mean,
        healthy_ratio: healthy,
        degraded_ratio: degraded,
        valid_pixels: n,
    }
}
