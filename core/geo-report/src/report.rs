//! Markdown / HTML report generation via Tera templates.
//!
//! Renders carbon accounting reports, audit summaries, and
//! spatial data overviews from structured data + templates.

use geo_core::errors::{GeoError, GeoResult};
use serde::Serialize;
use tera::{Context, Tera};

/// Generates reports from templates and structured data.
pub struct ReportGenerator {
    tera: Tera,
}

impl ReportGenerator {
    /// Create a new report generator with embedded templates.
    pub fn new() -> GeoResult<Self> {
        let mut tera = Tera::default();

        // Register built-in templates
        tera.add_raw_template("carbon_report.md", CARBON_REPORT_TEMPLATE)
            .map_err(|e| GeoError::Validation(format!("template: {e}")))?;

        tera.add_raw_template("audit_summary.md", AUDIT_SUMMARY_TEMPLATE)
            .map_err(|e| GeoError::Validation(format!("template: {e}")))?;

        tera.add_raw_template("html_report.html", HTML_REPORT_TEMPLATE)
            .map_err(|e| GeoError::Validation(format!("template: {e}")))?;

        Ok(Self { tera })
    }

    /// Render a carbon accounting report to Markdown.
    pub fn carbon_report(&self, data: &CarbonReportData) -> GeoResult<String> {
        let ctx = Context::from_serialize(data)
            .map_err(|e| GeoError::Other(format!("tera serialize: {e}")))?;
        self.tera
            .render("carbon_report.md", &ctx)
            .map_err(|e| GeoError::Validation(e.to_string()))
    }

    /// Render an audit trail summary.
    pub fn audit_summary(&self, data: &AuditSummaryData) -> GeoResult<String> {
        let ctx = Context::from_serialize(data)
            .map_err(|e| GeoError::Other(format!("tera serialize: {e}")))?;
        self.tera
            .render("audit_summary.md", &ctx)
            .map_err(|e| GeoError::Validation(e.to_string()))
    }

    /// Render a full HTML report with embedded CSS.
    pub fn html_report(&self, data: &CarbonReportData) -> GeoResult<String> {
        let ctx = Context::from_serialize(data)
            .map_err(|e| GeoError::Other(format!("tera serialize: {e}")))?;
        self.tera
            .render("html_report.html", &ctx)
            .map_err(|e| GeoError::Validation(e.to_string()))
    }

    /// Render and save a report to a file.
    pub fn save_report(&self, content: &str, output_path: &str) -> GeoResult<()> {
        std::fs::write(output_path, content)?;
        tracing::info!("Report saved: {output_path}");
        Ok(())
    }

    /// Register a custom template at runtime.
    pub fn add_template(&mut self, name: &str, content: &str) -> GeoResult<()> {
        self.tera
            .add_raw_template(name, content)
            .map_err(|e| GeoError::Validation(format!("template: {e}")))?;
        Ok(())
    }
}

// ── Template data structures ─────────────────────────────────────

/// Data for a carbon accounting report.
#[derive(Debug, Serialize)]
pub struct CarbonReportData {
    /// Report title.
    pub title: String,
    /// AOI identifier.
    pub aoi_name: String,
    /// Target year.
    pub year: u16,
    /// Date generated (ISO-8601).
    pub generated_at: String,
    /// Emission source description.
    pub source: String,
    /// Total emissions in tCO₂e.
    pub total_tco2e: f64,
    /// Breakdown by landcover class.
    pub breakdown: Vec<LandcoverBreakdown>,
    /// Audit trail entries.
    pub audit_trails: Vec<AuditTrailEntry>,
}

/// Per-class breakdown.
#[derive(Debug, Serialize)]
pub struct LandcoverBreakdown {
    /// Landcover class name.
    pub class: String,
    /// Area in hectares.
    pub area_ha: f64,
    /// Emission factor value.
    pub factor: f64,
    /// Emissions in tCO₂e.
    pub tco2e: f64,
}

/// Single audit trail entry.
#[derive(Debug, Serialize)]
pub struct AuditTrailEntry {
    /// Landcover class.
    pub class: String,
    /// DVC hash of remote sensing data.
    pub lc_hash: String,
    /// Factor set UUID.
    pub factor_id: String,
    /// DVC hash of factor data.
    pub factor_hash: String,
    /// Completeness status.
    pub complete: bool,
}

/// Data for an audit summary.
#[derive(Debug, Serialize)]
pub struct AuditSummaryData {
    /// Workflow run ID.
    pub workflow_id: String,
    /// AOI name.
    pub aoi_name: String,
    /// Year.
    pub year: u16,
    /// Total calculations.
    pub total_calculations: u32,
    /// Number with complete audit trails.
    pub complete_count: u32,
    /// Pending review count.
    pub pending_count: u32,
    /// Approved count.
    pub approved_count: u32,
    /// Individual entries.
    pub entries: Vec<AuditTrailEntry>,
}

// ── Built-in templates ───────────────────────────────────────────

const CARBON_REPORT_TEMPLATE: &str = r#"# {{ title }}

**AOI:** {{ aoi_name }}  
**Year:** {{ year }}  
**Source:** {{ source }}  
**Generated:** {{ generated_at }}  

---

## Summary

| Metric | Value |
|--------|-------|
| Total Emissions | **{{ total_tco2e | round(precision=1) }} tCO₂e** |
| Landcover Classes | {{ breakdown | length }} |

## Breakdown by Landcover Class

| Class | Area (ha) | Factor | tCO₂e |
|-------|-----------|--------|-------|
{% for item in breakdown -%}
| {{ item.class }} | {{ item.area_ha | round(precision=1) }} | {{ item.factor | round(precision=2) }} | {{ item.tco2e | round(precision=1) }} |
{% endfor %}

## Audit Trail

| Class | LC Hash | Factor ID | Factor Hash | Complete |
|-------|---------|-----------|-------------|----------|
{% for audit in audit_trails -%}
| {{ audit.class }} | {{ audit.lc_hash }} | {{ audit.factor_id | truncate(length=8) }} | {{ audit.factor_hash }} | {% if audit.complete %}✅{% else %}❌{% endif %} |
{% endfor %}

---

*Generated by geo-toolbox v0.1.0*
"#;

const AUDIT_SUMMARY_TEMPLATE: &str = r#"# Audit Summary: {{ aoi_name }} ({{ year }})

**Workflow:** `{{ workflow_id }}`

## Statistics

| Status | Count |
|--------|-------|
| Complete | {{ complete_count }} |
| Pending | {{ pending_count }} |
| Approved | {{ approved_count }} |
| **Total** | **{{ total_calculations }}** |

## Entries

{% for entry in entries -%}
- **{{ entry.class }}**: {% if entry.complete %}✅ Verified{% else %}❌ Incomplete{% endif %}
  - LC: `{{ entry.lc_hash }}`
  - Factor: `{{ entry.factor_id }}`
  - Version: `{{ entry.factor_hash }}`

{% endfor %}
"#;

const HTML_REPORT_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>{{ title }}</title>
<style>
  body { font-family: system-ui, sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; color: #333; }
  h1 { color: #1a5276; border-bottom: 3px solid #2980b9; padding-bottom: 0.5rem; }
  h2 { color: #2c3e50; }
  table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
  th, td { border: 1px solid #ddd; padding: 8px 12px; text-align: left; }
  th { background: #2980b9; color: white; }
  tr:nth-child(even) { background: #f2f2f2; }
  .metric { font-size: 1.5rem; font-weight: bold; color: #c0392b; }
  .footer { margin-top: 2rem; font-size: 0.8rem; color: #999; }
  .ok { color: #27ae60; }
  .warn { color: #e67e22; }
</style>
</head>
<body>

<h1>{{ title }}</h1>

<p><strong>AOI:</strong> {{ aoi_name }} &nbsp;|&nbsp;
   <strong>Year:</strong> {{ year }} &nbsp;|&nbsp;
   <strong>Source:</strong> {{ source }}</p>
<p><strong>Generated:</strong> {{ generated_at }}</p>

<h2>Summary</h2>
<p>Total Emissions: <span class="metric">{{ total_tco2e | round(precision=1) }} tCO₂e</span></p>

<h2>Breakdown</h2>
<table>
<tr><th>Class</th><th>Area (ha)</th><th>Factor</th><th>tCO₂e</th></tr>
{% for item in breakdown %}
<tr>
  <td>{{ item.class }}</td>
  <td>{{ item.area_ha | round(precision=1) }}</td>
  <td>{{ item.factor | round(precision=2) }}</td>
  <td>{{ item.tco2e | round(precision=1) }}</td>
</tr>
{% endfor %}
</table>

<h2>Audit Trail</h2>
<table>
<tr><th>Class</th><th>LC Hash</th><th>Factor ID</th><th>Complete</th></tr>
{% for audit in audit_trails %}
<tr>
  <td>{{ audit.class }}</td>
  <td><code>{{ audit.lc_hash }}</code></td>
  <td><code>{{ audit.factor_id | truncate(length=12) }}...</code></td>
  <td>{% if audit.complete %}<span class="ok">✅</span>{% else %}<span class="warn">❌</span>{% endif %}</td>
</tr>
{% endfor %}
</table>

<div class="footer">Generated by geo-toolbox v0.1.0</div>

</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_carbon_report_markdown() {
        let gen = ReportGenerator::new().unwrap();
        let data = CarbonReportData {
            title: "Test Report".into(),
            aoi_name: "Shenzhen Bay".into(),
            year: 2025,
            generated_at: "2026-06-06T12:00:00Z".into(),
            source: "IPCC_2019".into(),
            total_tco2e: 5000.0,
            breakdown: vec![
                LandcoverBreakdown {
                    class: "forest".into(),
                    area_ha: 1000.0,
                    factor: 4.85,
                    tco2e: 4850.0,
                },
                LandcoverBreakdown {
                    class: "grassland".into(),
                    area_ha: 100.0,
                    factor: 1.42,
                    tco2e: 142.0,
                },
            ],
            audit_trails: vec![AuditTrailEntry {
                class: "forest".into(),
                lc_hash: "abc123".into(),
                factor_id: "def45678-1234-5678-abcd-ef1234567890".into(),
                factor_hash: "def456".into(),
                complete: true,
            }],
        };

        let md = gen.carbon_report(&data).unwrap();
        assert!(md.contains("Shenzhen Bay"));
        assert!(md.contains("5000"));
        assert!(md.contains("forest"));
        assert!(md.contains("✅"));
    }

    #[test]
    fn test_html_report() {
        let gen = ReportGenerator::new().unwrap();
        let data = CarbonReportData {
            title: "HTML Test".into(),
            aoi_name: "Test".into(),
            year: 2025,
            generated_at: "2026-06-06".into(),
            source: "IPCC".into(),
            total_tco2e: 100.0,
            breakdown: vec![],
            audit_trails: vec![],
        };

        let html = gen.html_report(&data).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("HTML Test"));
    }
}
