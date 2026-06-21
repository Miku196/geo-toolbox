//! Harvest simulation for forest management.
//!
//! Supports selective, clearcut, and shelterwood harvest methods,
//! carbon impact analysis, and sustainable yield calculation.

/// Selective harvest from basal area.
pub fn selective_harvest(
    basal_area_before_m2_ha: f64,
    harvest_intensity_pct: f64,
    min_dbh_cm: f64,
) -> serde_json::Value {
    let intensity = harvest_intensity_pct.max(0.0).min(100.0) / 100.0;
    let removed_ba = basal_area_before_m2_ha * intensity;
    let retained_ba = basal_area_before_m2_ha - removed_ba;
    let retention_pct = if basal_area_before_m2_ha > 0.0 {
        retained_ba / basal_area_before_m2_ha * 100.0
    } else {
        0.0
    };

    serde_json::json!({
        "harvest_method": "selective",
        "basal_area_before_m2_ha": (basal_area_before_m2_ha * 100.0).round() / 100.0,
        "harvest_intensity_pct": harvest_intensity_pct,
        "removed_ba_m2_ha": (removed_ba * 100.0).round() / 100.0,
        "retained_ba_m2_ha": (retained_ba * 100.0).round() / 100.0,
        "retention_pct": (retention_pct * 100.0).round() / 100.0,
        "min_dbh_cm": min_dbh_cm,
    })
}

/// Clearcut harvest.
pub fn clearcut_harvest(
    area_ha: f64,
    volume_m3_ha: f64,
    carbon_stock_tco2e_ha: f64,
) -> serde_json::Value {
    let total_volume = area_ha * volume_m3_ha;
    let total_carbon = area_ha * carbon_stock_tco2e_ha;

    serde_json::json!({
        "harvest_method": "clearcut",
        "area_ha": area_ha,
        "total_volume_m3": (total_volume * 100.0).round() / 100.0,
        "volume_per_ha_m3": volume_m3_ha,
        "total_carbon_emitted_tco2e": (total_carbon * 100.0).round() / 100.0,
        "carbon_per_ha_tco2e": carbon_stock_tco2e_ha,
        "harvest_yield_m3": (total_volume * 100.0).round() / 100.0,
    })
}

/// Shelterwood harvest — phased removal of overstory.
/// Preparatory: 30%, Establishment: 40%, Removal: 100%
pub fn shelterwood_harvest(
    area_ha: f64,
    volume_m3_ha: f64,
    seed_trees_per_ha: u32,
    retention_pct: f64,
    phase: &str,
) -> serde_json::Value {
    let removal_ratio = match phase.to_lowercase().as_str() {
        "preparatory" | "prep" => 0.30,
        "establishment" | "est" => 0.40,
        "removal" | "final" => 1.00,
        _ => 0.0,
    };

    let total_volume = area_ha * volume_m3_ha;
    let volume_removed = total_volume * removal_ratio;
    let residual_volume = total_volume * (1.0 - removal_ratio);
    let residual_ba_pct = retention_pct.max(0.0).min(100.0) * (1.0 - removal_ratio);

    serde_json::json!({
        "harvest_method": "shelterwood",
        "phase": phase,
        "area_ha": area_ha,
        "seed_trees_per_ha": seed_trees_per_ha,
        "volume_removed_m3": (volume_removed * 100.0).round() / 100.0,
        "residual_volume_m3": (residual_volume * 100.0).round() / 100.0,
        "residual_ba_pct": (residual_ba_pct * 100.0).round() / 100.0,
        "removal_ratio": removal_ratio,
    })
}

/// Carbon impact of harvest.
pub fn harvest_carbon_impact(
    area_ha: f64,
    pre_harvest_carbon_tco2e_ha: f64,
    harvest_method: &str,
    harvest_intensity_pct: f64,
    regeneration_carbon_tco2e_ha_yr: f64,
    time_horizon_yrs: u32,
) -> serde_json::Value {
    let intensity = harvest_intensity_pct.max(0.0).min(100.0) / 100.0;
    let carbon_lost = pre_harvest_carbon_tco2e_ha * area_ha * intensity;
    let cumulative_regrowth = regeneration_carbon_tco2e_ha_yr * area_ha * time_horizon_yrs as f64;
    let net_carbon = cumulative_regrowth - carbon_lost;

    let payback_years = if regeneration_carbon_tco2e_ha_yr * area_ha > 0.0 {
        carbon_lost / (regeneration_carbon_tco2e_ha_yr * area_ha)
    } else {
        f64::MAX
    };

    serde_json::json!({
        "harvest_method": harvest_method,
        "area_ha": area_ha,
        "pre_harvest_carbon_tco2e_ha": pre_harvest_carbon_tco2e_ha,
        "harvest_intensity_pct": harvest_intensity_pct,
        "carbon_lost_tco2e": (carbon_lost * 100.0).round() / 100.0,
        "cumulative_regrowth_tco2e": (cumulative_regrowth * 100.0).round() / 100.0,
        "net_carbon_tco2e": (net_carbon * 100.0).round() / 100.0,
        "time_horizon_yrs": time_horizon_yrs,
        "payback_years": if payback_years.is_finite() { (payback_years * 10.0).round() / 10.0 } else { -1.0 },
        "carbon_status": if net_carbon >= 0.0 { "sink" } else { "source" },
    })
}

/// Sustainable yield (Annual Allowable Cut).
/// Area regulation method: AAC = area * volume / rotation
/// Growth-based: AAC = area * growth_rate
pub fn sustainable_yield(
    area_ha: f64,
    volume_m3_ha: f64,
    rotation_yrs: u32,
    growth_rate_m3_ha_yr: f64,
) -> f64 {
    if rotation_yrs == 0 {
        return area_ha * growth_rate_m3_ha_yr;
    }
    let area_regulation = area_ha * volume_m3_ha / rotation_yrs as f64;
    let growth_based = area_ha * growth_rate_m3_ha_yr;
    // Use the smaller of the two (conservative)
    area_regulation.min(growth_based)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selective_harvest() {
        let r = selective_harvest(25.0, 30.0, 10.0);
        assert!((r["removed_ba_m2_ha"].as_f64().unwrap() - 7.5).abs() < 0.01);
        assert!((r["retained_ba_m2_ha"].as_f64().unwrap() - 17.5).abs() < 0.01);
        assert!((r["retention_pct"].as_f64().unwrap() - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_selective_harvest_full() {
        let r = selective_harvest(25.0, 100.0, 5.0);
        assert!((r["removed_ba_m2_ha"].as_f64().unwrap() - 25.0).abs() < 0.01);
        assert_eq!(r["retention_pct"].as_f64().unwrap(), 0.0);
    }

    #[test]
    fn test_clearcut_harvest() {
        let r = clearcut_harvest(10.0, 200.0, 150.0);
        assert_eq!(r["total_volume_m3"].as_f64().unwrap(), 2000.0);
        assert_eq!(r["total_carbon_emitted_tco2e"].as_f64().unwrap(), 1500.0);
        assert_eq!(r["harvest_method"].as_str().unwrap(), "clearcut");
    }

    #[test]
    fn test_shelterwood_preparatory() {
        let r = shelterwood_harvest(10.0, 200.0, 20, 70.0, "preparatory");
        assert_eq!(r["phase"].as_str().unwrap(), "preparatory");
        assert!((r["volume_removed_m3"].as_f64().unwrap() - 600.0).abs() < 0.01);
        assert!((r["residual_volume_m3"].as_f64().unwrap() - 1400.0).abs() < 0.01);
    }

    #[test]
    fn test_shelterwood_removal() {
        let r = shelterwood_harvest(10.0, 200.0, 20, 70.0, "removal");
        assert_eq!(r["removal_ratio"].as_f64().unwrap(), 1.0);
        assert_eq!(r["residual_volume_m3"].as_f64().unwrap(), 0.0);
    }

    #[test]
    fn test_carbon_impact_net_sink() {
        let r = harvest_carbon_impact(100.0, 150.0, "selective", 30.0, 2.0, 50);
        // carbon_lost = 150*100*0.3 = 4500
        // regrowth = 2*100*50 = 10000
        // net = 10000 - 4500 = 5500 (sink)
        assert_eq!(r["carbon_status"].as_str().unwrap(), "sink");
        assert!(r["payback_years"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_carbon_impact_net_source() {
        let r = harvest_carbon_impact(100.0, 150.0, "clearcut", 100.0, 1.0, 30);
        // carbon_lost = 150*100*1.0 = 15000
        // regrowth = 1*100*30 = 3000
        // net = 3000 - 15000 = -12000 (source)
        assert_eq!(r["carbon_status"].as_str().unwrap(), "source");
    }

    #[test]
    fn test_sustainable_yield() {
        let aac = sustainable_yield(100.0, 200.0, 30, 5.0);
        // area_reg = 100*200/30 = 666.7, growth = 100*5 = 500
        // min = 500
        assert!((aac - 500.0).abs() < 0.1, "got {aac}");
    }

    #[test]
    fn test_sustainable_yield_area_regulation() {
        let aac = sustainable_yield(100.0, 200.0, 30, 10.0);
        // area_reg = 666.7, growth = 1000
        // min = 666.7
        assert!((aac - 666.67).abs() < 0.1, "got {aac}");
    }
}
