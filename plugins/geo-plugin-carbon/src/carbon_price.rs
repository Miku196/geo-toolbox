//! 碳价情景分析 — 不同碳市场价格下的项目收益。

/// 碳价情景：给定碳减排量（tCO₂e）和价格，计算收益。
pub fn carbon_price_scenario(
    tonnes_co2e: f64,
    price_per_tonne_usd: f64,
    scenario: &str,
) -> serde_json::Value {
    let price = match scenario {
        "eu_ets" => 85.0,
        "china_national" => 10.0,
        "california" => 30.0,
        "voluntary" => 5.0,
        "custom" => price_per_tonne_usd,
        _ => price_per_tonne_usd,
    };
    serde_json::json!({
        "scenario": scenario,
        "price_per_tonne_usd": price,
        "total_revenue_usd": tonnes_co2e * price
    })
}

/// 碳价预测：NPV 折现（折现率 5%）。
pub fn carbon_price_forecast(
    tonnes_co2e: f64,
    years: u32,
    annual_increase_pct: f64,
    start_price_usd: f64,
) -> serde_json::Value {
    let discount_rate = 0.05;
    let mut npv = 0.0;
    let mut annual_revenues = Vec::with_capacity(years as usize);
    let mut year_end_price = start_price_usd;

    for y in 0..years {
        year_end_price = start_price_usd * (1.0 + annual_increase_pct / 100.0).powi(y as i32);
        let revenue = tonnes_co2e * year_end_price;
        npv += revenue / (1.0_f64 + discount_rate).powi((y + 1) as i32);
        annual_revenues.push(revenue);
    }

    serde_json::json!({
        "npv_usd": npv,
        "annual_revenues": annual_revenues,
        "final_year_price": year_end_price
    })
}

/// 碳抵消收益计算（含缓冲池扣减）。
pub fn carbon_offset_revenue(
    project_type: &str,
    area_ha: f64,
    annual_sink_tco2e_per_ha: f64,
    credit_period_yrs: u32,
    price_per_tonne: f64,
    buffer_pct: f64,
) -> serde_json::Value {
    let annual_tco2e = area_ha * annual_sink_tco2e_per_ha;
    let total_gross = annual_tco2e * credit_period_yrs as f64;
    let buffer_tco2e = total_gross * buffer_pct / 100.0;
    let sellable_tco2e = total_gross - buffer_tco2e;
    let annual_revenue = annual_tco2e * price_per_tonne;
    let total_revenue = sellable_tco2e * price_per_tonne;

    serde_json::json!({
        "project_type": project_type,
        "area_ha": area_ha,
        "annual_tco2e": annual_tco2e,
        "buffer_pct": buffer_pct,
        "buffer_tco2e": buffer_tco2e,
        "sellable_tco2e": sellable_tco2e,
        "annual_revenue_usd": annual_revenue,
        "total_revenue_usd": total_revenue,
        "credit_period_yrs": credit_period_yrs
    })
}

/// 社会碳成本 SCC（IWG 2021）。
pub fn social_cost_of_carbon(tco2e: f64, discount_rate: f64) -> f64 {
    // IWG 2021: SCC = 51 $/tCO₂ at 3% discount
    // SCC(d) = SCC(3%) * (0.03 / d)^0.5
    if discount_rate <= 0.0 {
        return tco2e * 51.0;
    }
    tco2e * 51.0 * (0.03 / discount_rate).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eu_ets_scenario() {
        let r = carbon_price_scenario(1000.0, 0.0, "eu_ets");
        assert_eq!(r["price_per_tonne_usd"], 85.0);
        assert_eq!(r["total_revenue_usd"], 85000.0);
    }

    #[test]
    fn test_custom_scenario() {
        let r = carbon_price_scenario(500.0, 12.0, "custom");
        assert_eq!(r["price_per_tonne_usd"], 12.0);
        assert_eq!(r["total_revenue_usd"], 6000.0);
    }

    #[test]
    fn test_forecast() {
        let r = carbon_price_forecast(100.0, 5, 5.0, 10.0);
        assert!(r["npv_usd"].as_f64().unwrap_or(0.0) > 0.0);
        let revenues = r["annual_revenues"].as_array().unwrap();
        assert_eq!(revenues.len(), 5);
    }

    #[test]
    fn test_offset_revenue() {
        let r = carbon_offset_revenue("ARR", 100.0, 10.0, 30, 15.0, 20.0);
        assert_eq!(r["annual_tco2e"], 1000.0);
        let total = r["total_revenue_usd"].as_f64().unwrap();
        // 30000 * 0.8 * 15 = 360000
        assert!((total - 360000.0).abs() < 1.0, "total={total}");
    }

    #[test]
    fn test_social_cost() {
        let scc = social_cost_of_carbon(1000.0, 0.03);
        assert!((scc - 51000.0).abs() < 1.0, "scc={scc}");
    }
}
