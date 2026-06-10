//! 江西德兴铜矿生态修复评估报告
//!
//! 使用 geo-toolbox 完整管线:
//!   1. geo-plugin-ecology 评估管线 → RestorationAssessment
//!   2. geo-report 引擎 + Tera 模板 → 标准格式报告 (GB/T 33802-2017)
//!
//! 运行:
//!   cargo run --package dexing-copper-report

use geo_plugin_ecology::{EcologyConfig, EcologyPlugin, RestorationAssessment};
use geo_raster::RasterBand;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. 加载配置 ──
    let config: EcologyConfig = toml::from_str(include_str!("../rules_dexing.toml"))?;
    let plugin = EcologyPlugin::new(config);

    println!("📋 加载配置: {} v{}", plugin.config().plugin.name, plugin.config().plugin.version);
    println!("   方法学: IPCC Tier 1 — {}", plugin.config().carbon.source);

    // ── 2. 加载 AOI ──
    let aoi_name = "德兴铜矿及周边生态修复区";
    let aoi_geojson = include_str!("../dexing_mine_aoi.geojson");

    // ── 3. 模拟遥感数据 ──
    // 基于: Yu et al. (2023) Env Research, Zhang et al. (2021) Remote Sensing
    println!("🛰️  模拟遥感波段 (2015→2025)...");
    let (red_2015, nir_2015) = make_bands_2015();
    let (red_2025, nir_2025) = make_bands_2025();

    // ── 4. 运行生态修复评估管线 ──
    println!("🔬 运行评估管线...");
    let input = geo_plugin_ecology::AssessmentInput {
        aoi_name,
        aoi_geojson,
        baseline_red: &red_2015,
        baseline_nir: &nir_2015,
        assessment_red: &red_2025,
        assessment_nir: &nir_2025,
        baseline_year: 2015,
        assessment_year: 2025,
    };
    let assessment = plugin.assess_restoration(&input)?;
    println!("   ✅ 评估完成 — 评级: {}", assessment.conclusion.grade);

    // ── 5. 用 Tera 模板渲染报告 ──
    let md = render_report_with_template(&assessment)?;

    // ── 6. 生成 HTML 报告 ──
    let html = wrap_html(&md, &assessment);

    // ── 7. 输出 ──
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("output");
    std::fs::create_dir_all(&out_dir)?;

    let md_path = out_dir.join("德兴铜矿生态修复评估报告.md");
    let html_path = out_dir.join("德兴铜矿生态修复评估报告.html");
    std::fs::write(&md_path, &md)?;
    std::fs::write(&html_path, &html)?;
    println!("\n📄 Markdown: {}", md_path.display());
    println!("📄 HTML:     {}", html_path.display());

    // 保存评估 JSON
    let json = serde_json::to_string_pretty(&assessment)?;
    std::fs::write(out_dir.join("dexing_copper_assessment.json"), &json)?;
    println!("\n💾 JSON 结果: output/dexing_copper_assessment.json");

    Ok(())
}

/// 使用 Tera 模板渲染报告。
fn render_report_with_template(
    a: &RestorationAssessment,
) -> Result<String, Box<dyn std::error::Error>> {
    let template_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/geo-plugin-ecology/templates");

    let pattern = template_dir.join("**/*.tera");
    let mut tera = tera::Tera::new(&pattern.to_string_lossy())
        .map_err(|e| format!("模板加载失败: {e}"))?;

    // 注册自定义过滤器（与 geo-report 引擎一致）
    tera.register_filter("ha_fmt", ha_fmt_filter);
    tera.register_filter("co2_fmt", co2_fmt_filter);
    tera.register_filter("percent_fmt", percent_fmt_filter);
    tera.register_filter("date_fmt", date_fmt_filter);

    let context = serde_json::to_value(a)?;
    let ctx = tera::Context::from_serialize(&context)
        .map_err(|e| format!("上下文序列化失败: {e}"))?;

    let md = tera.render("restoration-report.md.tera", &ctx)
        .map_err(|e| format!("模板渲染失败: {e}"))?;
    Ok(md)
}

fn ha_fmt_filter(value: &tera::Value, _: &std::collections::HashMap<String, tera::Value>) -> std::result::Result<tera::Value, tera::Error> {
    if let Some(v) = value.as_f64() {
        Ok(tera::Value::String(format!("{:.1} ha", v)))
    } else {
        Ok(value.clone())
    }
}
fn co2_fmt_filter(value: &tera::Value, _: &std::collections::HashMap<String, tera::Value>) -> std::result::Result<tera::Value, tera::Error> {
    if let Some(v) = value.as_f64() {
        Ok(tera::Value::String(format!("{:.2} tCO₂e", v)))
    } else {
        Ok(value.clone())
    }
}
fn percent_fmt_filter(value: &tera::Value, _: &std::collections::HashMap<String, tera::Value>) -> std::result::Result<tera::Value, tera::Error> {
    if let Some(v) = value.as_f64() {
        Ok(tera::Value::String(format!("{:.1}%", v * 100.0)))
    } else {
        Ok(value.clone())
    }
}
fn date_fmt_filter(value: &tera::Value, _: &std::collections::HashMap<String, tera::Value>) -> std::result::Result<tera::Value, tera::Error> {
    if let Some(s) = value.as_str() {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            Ok(tera::Value::String(dt.format("%Y-%m-%d %H:%M").to_string()))
        } else {
            Ok(value.clone())
        }
    } else {
        Ok(value.clone())
    }
}

/// 将 Markdown 报告转为独立 HTML 文件（含中文字体、响应式样式）。
fn wrap_html(md: &str, a: &RestorationAssessment) -> String {
    let parser = pulldown_cmark::Parser::new_ext(md, pulldown_cmark::Options::all());
    let mut html_body = String::new();
    pulldown_cmark::html::push_html(&mut html_body, parser);

    let title = format!("{} 生态修复评估报告 ({}→{})", a.aoi_name, a.baseline_year, a.assessment_year);

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
*{{box-sizing:border-box;margin:0;padding:0}}
body{{font-family:"Microsoft YaHei","微软雅黑","PingFang SC","Hiragino Sans GB",sans-serif;background:#f5f5f0;color:#333;line-height:1.8;padding:20px}}
.container{{max-width:900px;margin:0 auto;background:#fff;padding:40px 50px;box-shadow:0 2px 12px rgba(0,0,0,0.08);border-radius:4px}}
h1{{font-size:22px;text-align:center;margin-bottom:6px;color:#1a1a1a}}
h2{{font-size:16px;text-align:center;color:#555;font-weight:400;margin-bottom:20px;border-bottom:1px solid #e0e0e0;padding-bottom:12px}}
h3{{font-size:14px;color:#2c3e50;margin:24px 0 8px;border-left:3px solid #27ae60;padding-left:8px}}
h4{{font-size:13px;color:#444;margin:16px 0 6px}}
table{{width:100%;border-collapse:collapse;margin:10px 0 16px;font-size:12px}}
th,td{{padding:6px 10px;text-align:left;border:1px solid #ddd}}
th{{background:#f0f7f0;font-weight:600;color:#2c3e50}}
tr:nth-child(even){{background:#fafafa}}
hr{{border:none;border-top:1px solid #e0e0e0;margin:16px 0}}
strong{{color:#1a1a1a}}
blockquote{{border-left:3px solid #27ae60;padding:6px 14px;margin:10px 0;background:#f9fcf9;font-size:13px;color:#555}}
p{{margin:6px 0;font-size:13px}}
ol,ul{{margin:8px 0 8px 20px;font-size:13px}}
li{{margin:3px 0}}
.footer{{margin-top:30px;padding-top:14px;border-top:1px solid #e0e0e0;font-size:11px;color:#999;text-align:center}}
</style>
</head>
<body>
<div class="container">
{html_body}
<div class="footer">本报告由 geo-toolbox 自动评估系统生成 | 监测标准: GB/T 32893-2016 / GB/T 33802-2017 / GB/T 51208-2017</div>
</div>
</body>
</html>"#,
    )
}

// ═══════════════════════════════════════════════════
// 遥感数据模拟（基于已发表论文参数）
// ═══════════════════════════════════════════════════

fn make_bands_2015() -> (RasterBand, RasterBand) {
    let rows = 14; let cols = 18; let n = rows * cols;
    #[rustfmt::skip]
    let lc: [[u8; 18]; 14] = [
        [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
        [0,0,0,0,3,3,0,0,0,0,0,0,3,3,0,0,0,0],
        [0,0,0,0,3,3,0,0,0,4,4,0,0,3,3,0,0,0],
        [0,0,0,5,5,5,0,0,2,2,2,2,0,0,5,5,5,0],
        [0,0,5,5,5,0,0,1,1,1,1,1,1,0,0,5,5,5],
        [0,0,5,5,1,1,1,1,1,1,1,1,1,1,1,1,5,5],
        [0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0],
        [0,0,0,1,1,1,1,1,1,2,2,1,1,1,1,1,1,0],
        [0,0,0,0,1,1,1,1,1,2,2,1,1,1,1,1,0,0],
        [0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,0,0,0],
        [0,0,0,0,0,0,0,6,6,6,6,6,6,0,0,0,0,0],
        [0,0,0,0,0,0,0,6,6,6,6,6,6,0,0,0,0,0],
        [0,0,0,0,0,0,4,4,4,4,4,4,4,4,0,0,0,0],
        [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    ];
    gen_bands(n, rows, cols, &lc, &[
        (0.03, 0.52),  // 0=天然林
        (0.25, 0.30),  // 1=采场/裸地
        (0.18, 0.22),  // 2=工业区
        (0.06, 0.35),  // 3=农田
        (0.05, 0.02),  // 4=水体
        (0.28, 0.30),  // 5=废石场
        (0.30, 0.31),  // 6=尾矿库
    ], "2015")
}

fn make_bands_2025() -> (RasterBand, RasterBand) {
    let rows = 14; let cols = 18; let n = rows * cols;
    #[rustfmt::skip]
    let lc: [[u8; 18]; 14] = [
        [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
        [0,0,0,0,3,3,0,0,0,0,0,0,3,3,0,0,0,0],
        [0,0,0,0,3,3,0,0,0,4,4,0,0,3,3,0,0,0],
        [0,0,0,7,7,7,0,0,2,2,2,2,0,0,7,7,7,0],  // 废石场→灌草修复
        [0,0,7,7,7,0,0,8,8,8,8,8,8,0,0,7,7,7],  // 边缘→森林修复
        [0,0,7,7,8,8,8,8,8,8,8,8,8,8,8,8,7,7],
        [0,0,0,8,8,8,8,8,8,8,8,8,8,8,8,8,8,0],  // 杨桃坞复垦基地
        [0,0,0,8,8,8,1,1,1,2,2,1,1,1,8,8,8,0],
        [0,0,0,0,8,8,1,1,1,2,2,1,1,1,8,8,0,0],
        [0,0,0,0,0,8,8,8,8,1,1,8,8,8,8,0,0,0],
        [0,0,0,0,0,0,0,8,8,8,8,8,8,0,0,0,0,0],
        [0,0,0,0,0,0,0,6,6,6,6,6,6,0,0,0,0,0],
        [0,0,0,0,0,0,4,4,4,4,4,4,4,4,0,0,0,0],
        [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    ];
    gen_bands(n, rows, cols, &lc, &[
        (0.03, 0.55),  // 0=天然林
        (0.24, 0.31),  // 1=采场
        (0.17, 0.23),  // 2=工业区
        (0.05, 0.37),  // 3=农田
        (0.05, 0.02),  // 4=水体
        (0.12, 0.35),  // 5=废石场(部分恢复)
        (0.28, 0.31),  // 6=尾矿库
        (0.10, 0.40),  // 7=修复灌草地
        (0.05, 0.48),  // 8=修复森林
    ], "2025")
}

fn gen_bands(
    n: usize, rows: usize, cols: usize,
    lc: &[[u8; 18]; 14],
    refl: &[(f64, f64)],
    year: &str,
) -> (RasterBand, RasterBand) {
    let (mut red, mut nir) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for r in 0..rows {
        for c in 0..cols {
            let (rb, nb) = refl[lc[r][c] as usize];
            let noise = |v: f64| (v + (fastrand::f64() - 0.5) * 0.015).clamp(0.0, 1.0);
            red.push(noise(rb));
            nir.push(noise(nb));
        }
    }
    (RasterBand::new(format!("RED_{year}"), rows, cols, red, -999.0),
     RasterBand::new(format!("NIR_{year}"), rows, cols, nir, -999.0))
}
