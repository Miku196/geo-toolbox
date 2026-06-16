//! CCER (中国核证自愿减排量) 报告模板。
//!
//! China Certified Emission Reduction voluntary carbon market report generator.

use serde::{Deserialize, Serialize};

/// CCER project methodology identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CcerMethodology {
    /// AR-CM-001-V01 造林碳汇项目方法学
    AfforestationMr,
    /// AR-CM-003-V01 森林经营碳汇项目方法学
    ForestMgmtMr,
    /// CM-092-V01 可再生能源并网发电方法学
    RenewableMr,
    /// CM-098-V01 工业能效提升方法学
    IndustrialEffMr,
    /// CM-004-V01 废弃物处理回收方法学
    WasteRecoveryMr,
    /// 自定义方法学
    Custom(String),
}

impl CcerMethodology {
    pub fn code(&self) -> &str {
        match self {
            CcerMethodology::AfforestationMr => "AR-CM-001-V01",
            CcerMethodology::ForestMgmtMr => "AR-CM-003-V01",
            CcerMethodology::RenewableMr => "CM-092-V01",
            CcerMethodology::IndustrialEffMr => "CM-098-V01",
            CcerMethodology::WasteRecoveryMr => "CM-004-V01",
            CcerMethodology::Custom(s) => s.as_str(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CcerMethodology::AfforestationMr => "造林碳汇",
            CcerMethodology::ForestMgmtMr => "森林经营碳汇",
            CcerMethodology::RenewableMr => "可再生能源并网发电",
            CcerMethodology::IndustrialEffMr => "工业能效提升",
            CcerMethodology::WasteRecoveryMr => "废弃物处理回收",
            CcerMethodology::Custom(_) => "自定义方法学",
        }
    }
}

/// CCER 项目减排量报告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcerReport {
    /// 项目名称
    pub project_name: String,
    /// 计入期开始年份
    pub crediting_start_year: u16,
    /// 计入期结束年份
    pub crediting_end_year: u16,
    /// 方法学
    pub methodology: String,
    /// 方法学说明
    pub methodology_desc: String,
    /// 基准线排放 (tCO₂e/yr)
    pub baseline_tco2e: f64,
    /// 项目排放 (tCO₂e/yr)
    pub project_tco2e: f64,
    /// 泄漏 (tCO₂e/yr)
    pub leakage_tco2e: f64,
    /// 额外性说明
    pub additionality: String,
}

impl CcerReport {
    pub fn new(
        project_name: &str,
        methodology: &CcerMethodology,
        baseline_tco2e: f64,
        project_tco2e: f64,
    ) -> Self {
        Self {
            project_name: project_name.to_string(),
            crediting_start_year: 2025,
            crediting_end_year: 2050,
            methodology: methodology.code().to_string(),
            methodology_desc: methodology.name().to_string(),
            baseline_tco2e,
            project_tco2e,
            leakage_tco2e: 0.0,
            additionality: "项目活动非基准线情景，具有额外性".to_string(),
        }
    }

    pub fn with_leakage(mut self, leakage_tco2e: f64) -> Self {
        self.leakage_tco2e = leakage_tco2e;
        self
    }

    pub fn with_additionality(mut self, additionality: &str) -> Self {
        self.additionality = additionality.to_string();
        self
    }

    pub fn with_crediting_period(mut self, start: u16, end: u16) -> Self {
        self.crediting_start_year = start;
        self.crediting_end_year = end;
        self
    }

    /// 净减排量 (tCO₂e/yr)。
    pub fn net_reduction_tco2e(&self) -> f64 {
        self.baseline_tco2e - self.project_tco2e - self.leakage_tco2e
    }

    /// 报告年份。
    pub fn report_year(&self) -> u16 {
        self.crediting_start_year
    }

    /// 生成格式化的 CCER 项目设计文件报告。
    pub fn generate_report(&self) -> String {
        let net = self.net_reduction_tco2e();
        let status = if net > 0.0 {
            "净减排（碳汇/减排项目）"
        } else {
            "净排放"
        };

        let mut lines = Vec::new();
        lines.push("═══════════════════════════════════════════════".to_string());
        lines.push("  CCER 项目设计文件（PDD）摘要报告".to_string());
        lines.push("═══════════════════════════════════════════════".to_string());
        lines.push(String::new());
        lines.push(format!("  项目名称:      {}", self.project_name));
        lines.push(format!(
            "  方法学:        {} ({})",
            self.methodology, self.methodology_desc
        ));
        lines.push(format!(
            "  计入期:        {} - {} ({} 年)",
            self.crediting_start_year,
            self.crediting_end_year,
            self.crediting_end_year - self.crediting_start_year
        ));
        lines.push(format!("  额外性说明:    {}", self.additionality));
        lines.push(String::new());
        lines.push("───────────────────────────────────────────────".to_string());
        lines.push("  温室气体减排量明细".to_string());
        lines.push("───────────────────────────────────────────────".to_string());
        lines.push(format!(
            "  基准线排放:    {:>12.2} tCO₂e/yr",
            self.baseline_tco2e
        ));
        lines.push(format!(
            "  项目排放:      {:>12.2} tCO₂e/yr",
            self.project_tco2e
        ));
        lines.push(format!(
            "  泄漏:          {:>12.2} tCO₂e/yr",
            self.leakage_tco2e
        ));
        lines.push("  ─────────────────────────────────────────".to_string());
        lines.push(format!("  净减排量:      {:>12.2} tCO₂e/yr", net));
        lines.push(String::new());
        lines.push(format!("  项目类型:      {}", status));
        lines.push(format!("  年减排总量:    {:>12.2} tCO₂e/yr", net));
        lines.push("═══════════════════════════════════════════════".to_string());
        lines.push(String::new());
        lines.push("注：本报告为 CCER 项目设计文件摘要，正式提交须".to_string());
        lines.push("附完整的 PDD 表格、监测计划和环境影响评估文件。".to_string());

        lines.join("\n")
    }

    /// 生成简化的 JSON 摘要（用于工具调用返回）。
    pub fn summary_json(&self) -> serde_json::Value {
        let net = self.net_reduction_tco2e();
        serde_json::json!({
            "project_name": self.project_name,
            "methodology": self.methodology,
            "methodology_desc": self.methodology_desc,
            "crediting_period": format!("{}-{}", self.crediting_start_year, self.crediting_end_year),
            "baseline_tco2e_per_year": self.baseline_tco2e,
            "project_tco2e_per_year": self.project_tco2e,
            "leakage_tco2e_per_year": self.leakage_tco2e,
            "net_reduction_tco2e_per_year": net,
            "is_carbon_sink": net > 0.0,
            "additionality": self.additionality
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_afforestation_report() {
        let report = CcerReport::new(
            "德兴铜矿矿区造林项目",
            &CcerMethodology::AfforestationMr,
            500.0, // baseline
            50.0,  // project
        )
        .with_leakage(10.0)
        .with_crediting_period(2025, 2055);

        let net = report.net_reduction_tco2e();
        // baseline - project - leakage = 500 - 50 - 10 = 440
        assert!((net - 440.0).abs() < 0.01);

        let text = report.generate_report();
        assert!(text.contains("德兴铜矿矿区造林项目"));
        assert!(text.contains("AR-CM-001-V01"));
        assert!(text.contains("440.00"));
        assert!(text.contains("净减排"));
    }

    #[test]
    fn test_renewable_report() {
        let report = CcerReport::new("光伏发电项目", &CcerMethodology::RenewableMr, 8000.0, 200.0);

        assert_eq!(report.methodology, "CM-092-V01");
        let net = report.net_reduction_tco2e();
        assert!((net - 7800.0).abs() < 0.01);

        let json = report.summary_json();
        assert_eq!(json["is_carbon_sink"], true);
    }

    #[test]
    fn test_net_emitter() {
        // Industrial project that increases emissions (no reduction)
        let report = CcerReport::new(
            "工业扩建项目",
            &CcerMethodology::IndustrialEffMr,
            100.0,
            300.0,
        )
        .with_leakage(20.0);

        let net = report.net_reduction_tco2e();
        assert!(net < 0.0); // net emitter
        assert!((net + 220.0).abs() < 0.01); // 100 - 300 - 20 = -220

        let text = report.generate_report();
        assert!(text.contains("净排放"));
    }

    #[test]
    fn test_custom_methodology() {
        let report = CcerReport::new(
            "蓝碳项目",
            &CcerMethodology::Custom("AR-CM-999".to_string()),
            1000.0,
            200.0,
        );

        assert_eq!(report.methodology, "AR-CM-999");
        assert_eq!(report.methodology_desc, "自定义方法学");
    }

    #[test]
    fn test_summary_json_fields() {
        let report = CcerReport::new("测试项目", &CcerMethodology::ForestMgmtMr, 1000.0, 300.0)
            .with_leakage(50.0);

        let json = report.summary_json();
        assert_eq!(json["project_name"], "测试项目");
        assert_eq!(json["methodology"], "AR-CM-003-V01");
        assert!((json["net_reduction_tco2e_per_year"].as_f64().unwrap() - 650.0).abs() < 0.01);
        assert_eq!(json["is_carbon_sink"], true);
    }
}
