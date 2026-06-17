use crate::config::AgriConfig;
use geo_core::errors::GeoResult;
use serde::{Deserialize, Serialize};

/// Agriculture plugin — crop yield, LAI, NPP, soil rating, irrigation.
pub struct AgriPlugin {
    pub config: AgriConfig,
}

impl AgriPlugin {
    pub fn new(config: AgriConfig) -> Self {
        Self { config }
    }

    /// ── NDVI → LAI conversion ──
    ///
    /// Exponential formula: LAI = -ln((1 - ndvi) / 1) / k
    ///
    /// Derived from the Beer-Lambert law. `k` is the extinction coefficient
    /// (≈0.5 for cereals, ≈0.6–0.65 for maize/soybean, crop-specific).
    pub fn ndvi_to_lai(&self, ndvi: f64, k: f64) -> f64 {
        if ndvi <= 0.0 || ndvi >= 1.0 || k <= 0.0 {
            return 0.0;
        }
        let lai = -(1.0 - ndvi).ln() / k;
        if lai.is_finite() && lai > 0.0 {
            lai
        } else {
            0.0
        }
    }

    /// ── LAI → fraction of intercepted photosynthetically active radiation ──
    ///
    /// fIPAR = 0.95 × (1 - exp(-0.6 × LAI))
    ///
    /// Bounded to [0, 0.95].
    pub fn lai_to_fipar(lai: f64) -> f64 {
        if lai <= 0.0 {
            return 0.0;
        }
        (0.95 * (1.0 - (-0.6 * lai).exp())).clamp(0.0, 0.95)
    }

    /// ── CASA model NPP estimation ──
    ///
    /// NPP = APAR × ε
    ///   APAR = PAR × fIPAR
    ///   PAR = incident PAR (MJ/m²/day)
    ///   ε = light use efficiency (gC/MJ), crop-specific
    pub fn estimate_npp(&self, par_mj_m2_day: f64, ndvi: f64, crop_type: &str) -> f64 {
        let k = self.config.crops.get(crop_type).map(|c| c.k).unwrap_or(0.5);
        let lue = self
            .config
            .crops
            .get(crop_type)
            .map(|c| c.light_use_efficiency)
            .unwrap_or(1.8);
        let lai = self.ndvi_to_lai(ndvi, k);
        let fipar = Self::lai_to_fipar(lai);
        par_mj_m2_day * fipar * lue
    }

    /// ── Multi-crop yield estimation ──
    ///
    /// Uses full LAI-based CASA model for best accuracy, with fallback
    /// to simplified NDVI × coefficient method.
    ///
    /// Simplified: Yield = area_ha × ndvi_mean × ndvi_weight × baseline_yield_kg_ha
    /// Full:       Yield = area_ha × (NPP × harvest_index) / biomass_fraction (≈0.45)
    pub fn estimate_yield(&self, area_ha: f64, ndvi_mean: f64, crop_type: &str) -> YieldResult {
        let params = self.config.crops.get(crop_type);

        // Full CASA-based estimation
        let par_default = 20.0; // typical growing-season daily PAR (MJ/m²/day)
        let lai = self.ndvi_to_lai(ndvi_mean, params.map(|c| c.k).unwrap_or(0.5));
        let fipar = Self::lai_to_fipar(lai);
        let lue = params.map(|c| c.light_use_efficiency).unwrap_or(1.8);
        let npp_daily = par_default * fipar * lue; // gC/m²/day
        let growing_season_days = 120.0; // typical
        let npp_season = npp_daily * growing_season_days; // gC/m²
        let hi = params.map(|c| c.harvest_index).unwrap_or(0.45);
        // Convert gC/m² to kg/ha grain: 1 gC/m² = 10 kg/ha (≈2.2× dry matter, 0.45 C fraction)
        let yield_casa_kg_ha = npp_season * 10.0 * hi;
        let yield_kg = yield_casa_kg_ha * area_ha;

        // Simplified NDVI-based estimation
        let ndvi_weight = params.map(|c| c.ndvi_weight).unwrap_or(1.2);
        let baseline = params.map(|c| c.baseline_yield_kg_ha).unwrap_or(6000.0);
        let yield_simple_kg = area_ha * ndvi_mean * ndvi_weight * baseline;

        YieldResult {
            lai,
            fipar,
            npp_gcm2_season: npp_season,
            yield_casa_kg: yield_kg,
            yield_simple_kg,
            yield_kg: (yield_kg + yield_simple_kg) / 2.0, // average both methods
            crop_type: crop_type.to_string(),
            area_ha,
        }
    }

    /// ── Comprehensive soil rating ──
    ///
    /// Ranks soil quality on a 0–100 scale considering:
    ///   - Organic matter content
    ///   - pH
    ///   - Available N, P, K
    ///   - Soil texture
    ///   - Drainage
    pub fn soil_rating_detailed(
        &self,
        organic_matter_pct: f64,
        ph: f64,
        n_mg_kg: f64,
        p_mg_kg: f64,
        k_mg_kg: f64,
        texture: &str,
        drainage_ok: bool,
    ) -> SoilRatingResult {
        let sp = &self.config.soil;

        // OM score (0–20)
        let om_score = if organic_matter_pct >= sp.om_good {
            20.0
        } else if organic_matter_pct >= sp.om_moderate {
            15.0 + 5.0 * (organic_matter_pct - sp.om_moderate) / (sp.om_good - sp.om_moderate)
        } else {
            10.0 * organic_matter_pct / sp.om_moderate
        };

        // pH score (0–20)
        let mid = (sp.ph_lo + sp.ph_hi) / 2.0;
        let half_range = (sp.ph_hi - sp.ph_lo) / 2.0;
        let ph_score = if (ph - mid).abs() <= half_range {
            20.0
        } else if ph >= 4.5 && ph <= 9.0 {
            let dist = (ph - mid).abs() - half_range;
            (20.0 - dist * 8.0).max(4.0)
        } else {
            0.0
        };

        // N score (0–20)
        let n_score = if n_mg_kg >= sp.n_good {
            20.0
        } else {
            20.0 * n_mg_kg / sp.n_good
        };

        // P score (0–20)
        let p_score = if p_mg_kg >= sp.p_good {
            20.0
        } else {
            20.0 * p_mg_kg / sp.p_good
        };

        // K score (0–20)
        let k_score = if k_mg_kg >= sp.k_good {
            20.0
        } else {
            20.0 * k_mg_kg / sp.k_good
        };

        // Texture multiplier (0–10 bonus, capped)
        let texture_weight = sp.texture_weights.get(texture).copied().unwrap_or(0.7);
        let texture_score = texture_weight * 10.0;

        // Drainage bonus (5 max)
        let drainage_score = if drainage_ok { 5.0 } else { 0.0 };

        let total =
            om_score + ph_score + n_score + p_score + k_score + texture_score + drainage_score;
        let total = total.clamp(0.0, 100.0);

        let grade = if total >= 85.0 {
            "优"
        } else if total >= 70.0 {
            "良"
        } else if total >= 50.0 {
            "中"
        } else {
            "差"
        };

        SoilRatingResult {
            score: total,
            grade: grade.to_string(),
            om_score,
            ph_score,
            n_score,
            p_score,
            k_score,
            texture_score,
            drainage_score,
        }
    }

    /// Simplified soil rating (backward-compatible).
    pub fn soil_rating(&self, organic_matter_pct: f64, ph: f64) -> &'static str {
        let organic_ok = organic_matter_pct >= 2.0;
        let ph_ok = (5.5..=8.0).contains(&ph);
        match (organic_ok, ph_ok) {
            (true, true) => "优",
            (true, false) | (false, true) => "中",
            _ => "差",
        }
    }

    /// ── Reference evapotranspiration (ET₀) via simplified Penman-Monteith ──
    ///
    /// FAO-56 simplified: ET₀ = 0.0023 × Ra × (Tavg + 17.8) × sqrt(Tmax - Tmin)
    ///
    /// where Ra = extraterrestrial radiation (MJ/m²/day) approximated by latitude/month.
    pub fn estimate_et0_simple(
        &self,
        tmax_c: f64,
        tmin_c: f64,
        latitude_rad: f64,
        month: u32,
    ) -> f64 {
        // Solar declination (rad)
        let j = month as f64 * 30.4 - 15.0; // approx day of year
        let decl = 0.4093 * (2.0 * std::f64::consts::PI * j / 365.0 - 1.39).sin();

        // Sunset hour angle (rad)
        let ws = (-latitude_rad.tan() * decl.tan()).acos();

        // Extraterrestrial radiation (MJ/m²/day)
        let solar_constant = 0.0820;
        let dr = 1.0 + 0.033 * (2.0 * std::f64::consts::PI * j / 365.0).cos();
        let ra = (solar_constant
            * dr
            * (ws * latitude_rad.sin() * decl.sin() + latitude_rad.cos() * decl.cos() * ws.sin()))
            * 11.574; // adjust units

        let tavg = (tmax_c + tmin_c) / 2.0;
        let t_range = (tmax_c - tmin_c).abs().sqrt();
        let et0 = 0.0023 * ra * (tavg + 17.8) * t_range;
        et0.max(0.0)
    }

    /// ── Net irrigation requirement ──
    ///
    /// I_net = ETc - Peff = (ET₀ × Kc) - Peff
    ///
    /// where Peff = effective rainfall (0.8 × total rainfall for most soils).
    pub fn net_irrigation(&self, et0_mm_day: f64, crop_type: &str, rainfall_mm: f64) -> f64 {
        let kc = self
            .config
            .crops
            .get(crop_type)
            .map(|c| c.kc)
            .unwrap_or(0.85);
        let etc = et0_mm_day * kc;
        let peff = rainfall_mm * 0.8; // effective rainfall approximation
        (etc - peff).max(0.0)
    }

    /// Gross irrigation requirement (accounting for application efficiency).
    pub fn gross_irrigation(&self, net_irrigation_mm: f64) -> f64 {
        let eff = self.config.irrigation.application_efficiency;
        if eff > 0.0 {
            net_irrigation_mm / eff
        } else {
            net_irrigation_mm
        }
    }
}

// ── Result structs ──

/// Multi-method yield estimation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldResult {
    pub lai: f64,
    pub fipar: f64,
    pub npp_gcm2_season: f64,
    pub yield_casa_kg: f64,
    pub yield_simple_kg: f64,
    pub yield_kg: f64,
    pub crop_type: String,
    pub area_ha: f64,
}

/// Detailed soil rating result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilRatingResult {
    pub score: f64,
    pub grade: String,
    pub om_score: f64,
    pub ph_score: f64,
    pub n_score: f64,
    pub p_score: f64,
    pub k_score: f64,
    pub texture_score: f64,
    pub drainage_score: f64,
}

// ── Plugin trait impls ──

impl geo_core::plugin::Plugin for AgriPlugin {
    type Config = crate::AgriConfig;

    fn new(config: crate::AgriConfig) -> Self {
        Self::new(config)
    }

    fn name(&self) -> &str {
        "agri"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Agriculture plugin — crop yield estimation, LAI/NPP modeling, soil quality rating, irrigation requirement"
    }
    fn category(&self) -> geo_core::plugin::PluginCategory {
        geo_core::plugin::PluginCategory::Process
    }
    fn is_healthy(&self) -> bool {
        true
    }
}

impl geo_core::plugin::ProcessPlugin for AgriPlugin {
    fn process_type(&self) -> &str {
        "agri"
    }

    async fn execute(&self, params: serde_json::Value) -> GeoResult<serde_json::Value> {
        let ndvi = params["ndvi_mean"].as_f64().unwrap_or(0.7);
        let area_ha = params["area_ha"].as_f64().unwrap_or(10.0);
        let crop_type = params["crop_type"].as_str().unwrap_or("wheat");

        let yield_result = self.estimate_yield(area_ha, ndvi, crop_type);
        let soil = self.soil_rating(
            params["organic_matter_pct"].as_f64().unwrap_or(3.0),
            params["ph"].as_f64().unwrap_or(6.5),
        );

        Ok(serde_json::json!({
            "yield_kg": yield_result.yield_kg,
            "lai": yield_result.lai,
            "npp_gcm2_season": yield_result.npp_gcm2_season,
            "soil_rating": soil,
            "crop_type": crop_type,
        }))
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgriConfig;
    use geo_core::plugin::ProcessPlugin;

    fn test_config() -> AgriConfig {
        AgriConfig::default()
    }

    #[test]
    fn test_ndvi_to_lai() {
        let p = AgriPlugin::new(test_config());
        // NDVI = 0.7, k = 0.5 → LAI = -ln(0.3)/0.5 ≈ 2.41
        let lai = p.ndvi_to_lai(0.7, 0.5);
        assert!((lai - 2.41).abs() < 0.01, "LAI expected ~2.41, got {lai}");

        // NDVI = 0 → LAI = 0
        assert_eq!(p.ndvi_to_lai(0.0, 0.5), 0.0);
        // NDVI = negative → 0
        assert_eq!(p.ndvi_to_lai(-0.1, 0.5), 0.0);
    }

    #[test]
    fn test_lai_to_fipar() {
        let fipar = AgriPlugin::lai_to_fipar(2.41);
        assert!((fipar - 0.95_f64 * (1.0_f64 - (-0.6_f64 * 2.41_f64).exp())).abs() < 0.001);
        assert!(fipar > 0.0 && fipar <= 0.95);

        // LAI = 0 → fIPAR = 0
        assert!((AgriPlugin::lai_to_fipar(0.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_estimate_npp() {
        let p = AgriPlugin::new(test_config());
        // wheat: PAR=20, NDVI=0.7, k=0.55, LUE=1.8
        let npp = p.estimate_npp(20.0, 0.7, "wheat");
        assert!(npp > 0.0, "NPP should be positive");
        assert!(npp < 50.0, "NPP should be reasonable (< 50)");
    }

    #[test]
    fn test_yield_estimation() {
        let p = AgriPlugin::new(test_config());
        let result = p.estimate_yield(10.0, 0.7, "corn");
        assert!(result.yield_kg > 0.0, "Yield should be positive");
        assert_eq!(result.crop_type, "corn");
        assert!(result.lai > 0.0);
        assert!(result.fipar > 0.0);
        assert!(result.yield_casa_kg > 0.0);
        assert!(result.yield_simple_kg > 0.0);
    }

    #[test]
    fn test_yield_different_crops() {
        let p = AgriPlugin::new(test_config());
        let wheat = p.estimate_yield(10.0, 0.7, "wheat");
        let rice = p.estimate_yield(10.0, 0.7, "rice");
        let soybean = p.estimate_yield(10.0, 0.7, "soybean");

        // Different crops should give different yields
        assert!(
            (wheat.yield_kg - rice.yield_kg).abs() > 0.01,
            "Wheat/rice should differ"
        );
        assert!(
            (wheat.yield_kg - soybean.yield_kg).abs() > 0.01,
            "Wheat/soybean should differ"
        );
    }

    #[test]
    fn test_unknown_crop_fallback() {
        let p = AgriPlugin::new(test_config());
        let result = p.estimate_yield(10.0, 0.7, "unknown_crop");
        assert!(result.yield_kg > 0.0);
        assert_eq!(result.crop_type, "unknown_crop");
    }

    #[test]
    fn test_soil_rating_detailed() {
        let p = AgriPlugin::new(test_config());
        // Excellent soil
        let r = p.soil_rating_detailed(4.0, 6.8, 150.0, 30.0, 200.0, "loam", true);
        assert!(
            r.score >= 85.0,
            "Excellent soil should score ≥85, got {}",
            r.score
        );
        assert_eq!(r.grade, "优");

        // Poor soil
        let r = p.soil_rating_detailed(0.5, 4.0, 30.0, 5.0, 50.0, "sand", false);
        assert!(
            r.score < 50.0,
            "Poor soil should score <50, got {}",
            r.score
        );
        assert_eq!(r.grade, "差");
    }

    #[test]
    fn test_soil_rating_simple() {
        let p = AgriPlugin::new(test_config());
        assert_eq!(p.soil_rating(3.0, 6.5), "优");
        assert_eq!(p.soil_rating(0.5, 4.0), "差");
        assert_eq!(p.soil_rating(3.0, 4.0), "中");
        assert_eq!(p.soil_rating(0.5, 6.5), "中");
    }

    #[test]
    fn test_et0_calculation() {
        let p = AgriPlugin::new(test_config());
        // Summer: Tmax=30, Tmin=20, lat=30°N, month=7
        let lat = 30.0_f64.to_radians();
        let et0 = p.estimate_et0_simple(30.0, 20.0, lat, 7);
        assert!(et0 > 0.0, "ET₀ should be positive");
        assert!(et0 < 10.0, "Summer ET₀ should be < 10 mm/day, got {et0}");
    }

    #[test]
    fn test_et0_winter_vs_summer() {
        let p = AgriPlugin::new(test_config());
        let lat = 30.0_f64.to_radians();
        let summer = p.estimate_et0_simple(32.0, 22.0, lat, 7);
        let winter = p.estimate_et0_simple(10.0, 2.0, lat, 1);
        assert!(summer > winter, "Summer ET₀ should be > winter ET₀");
    }

    #[test]
    fn test_net_irrigation() {
        let p = AgriPlugin::new(test_config());
        // Dry period: ET₀=5mm, rice (Kc=1.05), no rain
        let net = p.net_irrigation(5.0, "rice", 0.0);
        // ETc = 5*1.05 = 5.25, Peff = 0, net = 5.25
        assert!((net - 5.25).abs() < 0.01, "Expected 5.25, got {net}");

        // Wet period: enough rain
        let net = p.net_irrigation(5.0, "wheat", 50.0);
        assert!(net <= 0.0, "No irrigation needed in wet period");
    }

    #[test]
    fn test_gross_irrigation() {
        let p = AgriPlugin::new(test_config());
        let gross = p.gross_irrigation(10.0);
        // eff=0.75 → 10/0.75 ≈ 13.33
        assert!((gross - 13.33).abs() < 0.01);
    }

    #[test]
    fn test_execute() {
        let p = AgriPlugin::new(test_config());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(p.execute(serde_json::json!({
                "ndvi_mean": 0.7,
                "area_ha": 10.0,
                "crop_type": "wheat",
                "organic_matter_pct": 3.0,
                "ph": 6.5,
            })))
            .unwrap();

        assert!(result["yield_kg"].as_f64().unwrap() > 0.0);
        assert!(result["lai"].as_f64().unwrap() > 0.0);
        assert_eq!(result["soil_rating"].as_str().unwrap(), "优");
        assert_eq!(result["crop_type"].as_str().unwrap(), "wheat");
    }
}
