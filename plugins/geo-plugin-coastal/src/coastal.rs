//! 海岸带变化监测 — 侵蚀速率 + 海平面上升淹没。

use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use geo_raster::RasterBand;
use serde::Serialize;

pub struct CoastalPlugin;

#[derive(Debug, Clone, Serialize)]
pub struct ShorelineReport {
    pub aoi_name: String,
    pub bbox: BBox,
    pub baseline_year: u16,
    pub assessment_year: u16,
    /// 岸线变化率 (m/yr)，负值=侵蚀
    pub erosion_rate_m_per_yr: f64,
    /// 侵蚀岸段占比
    pub erosion_ratio: f64,
    /// 海平面上升淹没面积 (ha)
    pub inundated_area_ha: f64,
    pub risk_level: String,
    pub summary: String,
}

impl Default for CoastalPlugin {
    fn default() -> Self {
        Self
    }
}

impl CoastalPlugin {
    pub fn new() -> Self {
        Self
    }

    /// 岸线变化评估（基于两期 NDVI 差值判断陆地→水体转换）。
    #[allow(clippy::too_many_arguments)]
    pub fn assess_shoreline(
        &self,
        aoi_name: &str,
        aoi_geojson: &str,
        dem: &RasterBand,
        ndvi_old: &RasterBand,
        ndvi_new: &RasterBand,
        baseline_year: u16,
        assessment_year: u16,
        sea_level_rise_m: f64,
    ) -> GeoResult<ShorelineReport> {
        let bbox = geo_io::extract_bbox(aoi_geojson)?;
        let years = (assessment_year - baseline_year).max(1) as f64;

        let mut eroded = 0usize;
        let mut total = 0usize;
        let mut inundated = 0usize;

        let n = ndvi_old
            .data
            .len()
            .min(ndvi_new.data.len())
            .min(dem.data.len());
        for i in 0..n {
            let o = ndvi_old.data[i];
            let nv = ndvi_new.data[i];
            if o == ndvi_old.nodata || nv == ndvi_new.nodata {
                continue;
            }
            total += 1;

            // 陆地→水体: NDVI 显著下降
            if o > 0.2 && nv < 0.05 {
                eroded += 1;
            }

            // 海平面上升淹没: DEM < SLR
            let elev = dem.data[i];
            if elev != dem.nodata && elev < sea_level_rise_m {
                inundated += 1;
            }
        }

        let erosion_ratio = if total > 0 {
            eroded as f64 / total as f64
        } else {
            0.0
        };
        let erosion_rate = if erosion_ratio > 0.0 {
            (erosion_ratio * 100.0) / years
        } else {
            0.0
        }; // %/yr → 近似 m/yr
        let inundated_ha = inundated as f64 * 0.01; // 10m像素→ha

        let (risk, summary) = if erosion_ratio > 0.1 || inundated_ha > 50.0 {
            (
                "🔴 高风险",
                format!(
                    "{aoi_name} 侵蚀显著 ({:.0}%)，淹没 {:.0} ha",
                    erosion_ratio * 100.0,
                    inundated_ha
                ),
            )
        } else if erosion_ratio > 0.03 {
            (
                "🟡 中风险",
                format!("{aoi_name} 中度侵蚀 ({:.0}%)", erosion_ratio * 100.0),
            )
        } else {
            ("🟢 低风险", format!("{aoi_name} 岸线稳定"))
        };

        Ok(ShorelineReport {
            aoi_name: aoi_name.to_string(),
            bbox,
            baseline_year,
            assessment_year,
            erosion_rate_m_per_yr: erosion_rate,
            erosion_ratio,
            inundated_area_ha: inundated_ha,
            risk_level: risk.to_string(),
            summary: summary.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn band(d: Vec<f64>) -> RasterBand {
        RasterBand::new("t", d.len(), 1, d, -999.0)
    }

    #[test]
    fn test_erosion() {
        let p = CoastalPlugin::new();
        let aoi = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[121.0,31.0],[121.1,31.0],[121.1,31.1],[121.0,31.1],[121.0,31.0]]]}}]}"#;
        let r = p
            .assess_shoreline(
                "上海海岸",
                aoi,
                &band(vec![5.0, 2.0, 3.0, 1.0]),
                &band(vec![0.35, 0.30, 0.25, 0.10]),
                &band(vec![0.02, 0.28, 0.03, 0.01]),
                2015,
                2025,
                1.0,
            )
            .unwrap();
        assert!(r.erosion_ratio > 0.0);
    }
}
