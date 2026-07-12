//! Ecosystem services: water yield, sediment retention, habitat quality, carbon storage.
//! InVEST-style simplified models.

use serde::Serialize;

// ── Water Yield ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct WaterYieldResult {
    pub precipitation_mm: f64,
    pub evapotranspiration_mm: f64,
    pub water_yield_mm: f64,
    pub water_yield_m3_ha: f64,
    pub runoff_coefficient: f64,
}

/// Simplified water yield model (Budyko curve).
/// Y = P * (1 - AET/P), where AET/P ≈ f(aridity index)
/// AET/P = 1 + PET/P - (1 + (PET/P)^ω)^(1/ω)  (Fu's equation)
pub fn water_yield(precip_mm: f64, pet_mm: f64, omega: f64) -> WaterYieldResult {
    let aridity = if precip_mm > 0.0 {
        pet_mm / precip_mm
    } else {
        10.0
    };
    let aet_ratio = 1.0 + aridity - (1.0 + aridity.powf(omega)).powf(1.0 / omega);
    let yield_mm = precip_mm * (1.0 - aet_ratio);
    WaterYieldResult {
        precipitation_mm: precip_mm,
        evapotranspiration_mm: precip_mm * aet_ratio,
        water_yield_mm: (yield_mm * 10.0).round() / 10.0,
        water_yield_m3_ha: yield_mm * 10.0,
        runoff_coefficient: if precip_mm > 0.0 {
            (yield_mm / precip_mm * 1000.0).round() / 1000.0
        } else {
            0.0
        },
    }
}

// ── Sediment Retention ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SedimentRetentionResult {
    pub soil_loss_t_ha_yr: f64,
    pub sediment_retained_t_ha_yr: f64,
    pub sediment_export_t_ha_yr: f64,
    pub retention_efficiency_pct: f64,
}

/// Simplified sediment retention (SDR — Sediment Delivery Ratio).
/// Sediment export = USLE * SDR;  Retention = USLE * (1 - SDR)
/// SDR depends on upstream area and land cover.
pub fn sediment_retention(
    soil_loss_t_ha_yr: f64,
    upstream_area_ha: f64,
    land_cover_roughness: f64,
) -> SedimentRetentionResult {
    let sdr = if upstream_area_ha > 0.0 {
        (upstream_area_ha.powf(-0.2) * land_cover_roughness).min(1.0)
    } else {
        0.5
    };
    let export = soil_loss_t_ha_yr * sdr;
    SedimentRetentionResult {
        soil_loss_t_ha_yr,
        sediment_retained_t_ha_yr: (soil_loss_t_ha_yr - export).max(0.0),
        sediment_export_t_ha_yr: export,
        retention_efficiency_pct: ((1.0 - sdr) * 1000.0).round() / 10.0,
    }
}

// ── Habitat Quality ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct HabitatQualityResult {
    pub habitat_score: f64, // 0-1
    pub degradation_score: f64,
    pub quality_class: String,
    pub threat_distance_score: f64,
}

/// Simplified habitat quality model (InVEST-style).
/// Quality = H × (1 - D), where H = habitat suitability, D = degradation from threats.
/// D = Σ(impact_weight × linear_decay(distance))
pub fn habitat_quality(
    habitat_suitability: f64,
    threat_distances: &[(f64, f64)], // (distance, max_impact_distance) pairs
    threat_weights: &[f64],
) -> HabitatQualityResult {
    let mut degradation = 0.0;
    for (i, &(dist, max_dist)) in threat_distances.iter().enumerate() {
        if dist < max_dist {
            let w = threat_weights.get(i).copied().unwrap_or(1.0);
            degradation += w * (1.0 - dist / max_dist);
        }
    }
    degradation = degradation.min(1.0);
    let quality = habitat_suitability * (1.0 - degradation);
    let class = if quality > 0.8 {
        "High quality habitat"
    } else if quality > 0.5 {
        "Moderate"
    } else if quality > 0.3 {
        "Low quality"
    } else {
        "Degraded"
    };
    HabitatQualityResult {
        habitat_score: (quality * 100.0).round() / 100.0,
        degradation_score: (degradation * 100.0).round() / 100.0,
        quality_class: class.to_string(),
        threat_distance_score: (1.0 - degradation) * 100.0,
    }
}

// ── Carbon Storage ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CarbonStorageResult {
    pub aboveground_biomass_tc_ha: f64,
    pub belowground_biomass_tc_ha: f64,
    pub soil_organic_carbon_tc_ha: f64,
    pub dead_organic_matter_tc_ha: f64,
    pub total_carbon_tc_ha: f64,
    pub total_co2e_t_ha: f64,
}

/// Aggregate carbon storage across four IPCC pools.
pub fn carbon_storage(
    agb_tc_ha: f64,
    bgb_tc_ha: f64,
    soc_tc_ha: f64,
    dom_tc_ha: f64,
) -> CarbonStorageResult {
    let total = agb_tc_ha + bgb_tc_ha + soc_tc_ha + dom_tc_ha;
    CarbonStorageResult {
        aboveground_biomass_tc_ha: agb_tc_ha,
        belowground_biomass_tc_ha: bgb_tc_ha,
        soil_organic_carbon_tc_ha: soc_tc_ha,
        dead_organic_matter_tc_ha: dom_tc_ha,
        total_carbon_tc_ha: total,
        total_co2e_t_ha: total * 44.0 / 12.0,
    }
}

// ── Nutrient Retention ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct NutrientRetentionResult {
    pub n_load_kg_ha_yr: f64,
    pub n_retained_kg_ha_yr: f64,
    pub n_export_kg_ha_yr: f64,
    pub retention_efficiency_pct: f64,
}

/// Simplified nutrient retention model.
/// Export = Load × (1 - retention_efficiency)
/// Retention efficiency depends on vegetation filter width and slope.
pub fn nutrient_retention(
    n_load_kg_ha_yr: f64,
    buffer_width_m: f64,
    slope_pct: f64,
) -> NutrientRetentionResult {
    let efficiency =
        (1.0 - (-0.02 * buffer_width_m).exp()) * (1.0 - slope_pct / 100.0).max(0.1).min(0.95);
    let retained = n_load_kg_ha_yr * efficiency;
    NutrientRetentionResult {
        n_load_kg_ha_yr,
        n_retained_kg_ha_yr: (retained * 100.0).round() / 100.0,
        n_export_kg_ha_yr: (n_load_kg_ha_yr - retained).max(0.0),
        retention_efficiency_pct: (efficiency * 1000.0).round() / 10.0,
    }
}

// ── Plugin ───────────────────────────────────────────────────────

pub struct EcosystemPlugin;

impl EcosystemPlugin {
    pub fn water_yield(&self, p: f64, pet: f64, w: f64) -> WaterYieldResult {
        water_yield(p, pet, w)
    }
    pub fn sediment(&self, sl: f64, ua: f64, lcr: f64) -> SedimentRetentionResult {
        sediment_retention(sl, ua, lcr)
    }
    pub fn habitat(&self, hs: f64, td: &[(f64, f64)], tw: &[f64]) -> HabitatQualityResult {
        habitat_quality(hs, td, tw)
    }
    pub fn carbon(&self, a: f64, b: f64, s: f64, d: f64) -> CarbonStorageResult {
        carbon_storage(a, b, s, d)
    }
    pub fn nutrient(&self, n: f64, bw: f64, sp: f64) -> NutrientRetentionResult {
        nutrient_retention(n, bw, sp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_water_yield() {
        let r = water_yield(800.0, 1200.0, 2.6);
        assert!(r.water_yield_mm > 0.0);
    }
    #[test]
    fn test_sediment() {
        let r = sediment_retention(10.0, 100.0, 0.6);
        assert!(r.retention_efficiency_pct > 0.0);
    }
    #[test]
    fn test_habitat() {
        let r = habitat_quality(0.8, &[(500.0, 1000.0), (2000.0, 3000.0)], &[1.0, 0.5]);
        assert!(r.habitat_score > 0.2);
    }
    #[test]
    fn test_carbon() {
        let r = carbon_storage(80.0, 20.0, 60.0, 5.0);
        assert!(r.total_co2e_t_ha > 100.0);
    }
    #[test]
    fn test_nutrient() {
        let r = nutrient_retention(50.0, 30.0, 5.0);
        assert!(r.retention_efficiency_pct > 30.0);
    }
}
