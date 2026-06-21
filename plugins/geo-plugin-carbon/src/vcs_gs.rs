//! VCS/GS 额外性评估与黄金标准 SDG 贡献。

/// VCS 额外性评估（四步法）。
pub fn vcs_additionality_assessment(
    project_type: &str,
    baseline_scenario: &str,
    additionality_evidence: &[&str],
) -> serde_json::Value {
    let regulatory = additionality_evidence.iter().any(|e| *e == "regulatory");
    let barrier = additionality_evidence.iter().any(|e| *e == "barrier");
    let investment = additionality_evidence.iter().any(|e| *e == "investment");
    let common_practice = additionality_evidence
        .iter()
        .any(|e| *e == "common_practice");

    let overall_pass = regulatory && barrier && investment && common_practice;
    let score = if overall_pass {
        100
    } else {
        [regulatory, barrier, investment, common_practice]
            .iter()
            .filter(|&&x| x)
            .count() as u8
            * 25
    };

    serde_json::json!({
        "project_type": project_type,
        "baseline_scenario": baseline_scenario,
        "overall_pass": overall_pass,
        "regulatory": regulatory,
        "barrier": barrier,
        "investment": investment,
        "common_practice": common_practice,
        "score": score,
        "recommendation": if overall_pass { "additionality demonstrated" } else { "additional evidence required" }
    })
}

/// 黄金标准 SDG 贡献映射。
pub fn gold_standard_sdg(scenario_type: &str, sdg_contributions: &[u8]) -> serde_json::Value {
    let valid_sdgs: Vec<u8> = sdg_contributions
        .iter()
        .copied()
        .filter(|sdg| {
            *sdg == 3
                || *sdg == 5
                || *sdg == 7
                || *sdg == 8
                || *sdg == 11
                || *sdg == 13
                || *sdg == 15
        })
        .collect();

    let safeguards_pass = valid_sdgs.contains(&3) || valid_sdgs.contains(&5);
    let climate_sdg = valid_sdgs.contains(&13);
    let land_sdg = valid_sdgs.contains(&15);

    serde_json::json!({
        "scenario_type": scenario_type,
        "sdgs_contributed": valid_sdgs,
        "safeguards_pass": safeguards_pass,
        "climate_action_sdg13": climate_sdg,
        "life_on_land_sdg15": land_sdg,
        "overall_eligible": safeguards_pass && climate_sdg
    })
}

/// VCS 缓冲池扣减率。
///
/// 按项目类型和风险等级返回缓冲扣减百分比。
pub fn vcs_buffer_calculation(annual_tco2e: f64, project_type: &str, risk_class: &str) -> f64 {
    let base_buffer = match project_type {
        "ARR" | "afforestation" | "reforestation" => 20.0,
        "IFM" | "forest_management" => 15.0,
        "REDD" | "redd_plus" | "avoided_deforestation" => 25.0,
        "soil_carbon" | "agricultural" => 30.0,
        "wetland" | "peatland" => 35.0,
        _ => 20.0,
    };

    let risk_mult = match risk_class {
        "low" => 0.8_f64,
        "medium" | "moderate" => 1.0_f64,
        "high" => 1.2_f64,
        _ => 1.0_f64,
    };

    (base_buffer * risk_mult).min(50.0_f64)
}

/// VCS 验证检查：项目净减排量、面积等合理性。
pub fn vcs_validation_check(
    project_type: &str,
    area_ha: f64,
    baseline_tco2e: f64,
    project_tco2e: f64,
) -> serde_json::Value {
    let net_tco2e = baseline_tco2e - project_tco2e;
    let mut errors = Vec::new();

    if area_ha <= 0.0 {
        errors.push("area must be > 0 ha");
    }
    if baseline_tco2e <= 0.0 {
        errors.push("baseline must be > 0 tCO₂e/yr");
    }
    if project_tco2e < 0.0 {
        errors.push("project must be >= 0 tCO₂e/yr");
    }
    if net_tco2e <= 0.0 {
        errors.push("net emissions must be > 0 tCO₂e/yr for crediting");
    }

    let passes = errors.is_empty();

    serde_json::json!({
        "project_type": project_type,
        "area_ha": area_ha,
        "baseline_tco2e": baseline_tco2e,
        "project_tco2e": project_tco2e,
        "net_tco2e": net_tco2e,
        "passes_validation": passes,
        "errors": errors
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_additionality_all_pass() {
        let evidence = vec!["regulatory", "barrier", "investment", "common_practice"];
        let r = vcs_additionality_assessment("ARR", "grassland", &evidence);
        assert!(r["overall_pass"].as_bool().unwrap());
        assert_eq!(r["score"], 100);
    }

    #[test]
    fn test_additionality_partial() {
        let evidence = vec!["regulatory", "barrier"];
        let r = vcs_additionality_assessment("ARR", "grassland", &evidence);
        assert!(!r["overall_pass"].as_bool().unwrap());
        assert_eq!(r["score"], 50);
    }

    #[test]
    fn test_gs_sdg() {
        let sdgs = vec![7, 13, 15];
        let r = gold_standard_sdg("renewable_energy", &sdgs);
        assert!(r["climate_action_sdg13"].as_bool().unwrap());
    }

    #[test]
    fn test_gs_sdg_no_safeguards() {
        let sdgs = vec![13];
        let r = gold_standard_sdg("renewable_energy", &sdgs);
        assert!(!r["safeguards_pass"].as_bool().unwrap());
    }

    #[test]
    fn test_buffer_afforestation() {
        let buf = vcs_buffer_calculation(1000.0, "ARR", "low");
        assert!((buf - 16.0).abs() < 0.01, "buffer={buf}");
    }

    #[test]
    fn test_buffer_redd_high() {
        let buf = vcs_buffer_calculation(1000.0, "REDD", "high");
        assert!((buf - 30.0).abs() < 0.01, "buffer={buf}");
    }

    #[test]
    fn test_validation_pass() {
        let r = vcs_validation_check("ARR", 100.0, 5000.0, 500.0);
        assert!(r["passes_validation"].as_bool().unwrap());
    }

    #[test]
    fn test_validation_fail() {
        let r = vcs_validation_check("ARR", 0.0, 100.0, 200.0);
        assert!(!r["passes_validation"].as_bool().unwrap());
    }
}
