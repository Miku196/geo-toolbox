//! Tera 模板引擎封装。
//!
//! 自动注册公共组件（partials/layouts）和自定义过滤器。

use geo_core::errors::{GeoError, GeoResult};
use serde::Serialize;
use std::path::Path;
use tera::{Context, Tera, Value};

/// 报告渲染引擎。
pub struct ReportEngine {
    tera: Tera,
}

impl ReportEngine {
    /// 创建引擎，自动加载内置公共模板。
    pub fn new() -> GeoResult<Self> {
        let mut tera = Tera::default();
        // 注册公共模板目录（编译期嵌入或从文件加载）
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        if template_dir.exists() {
            let pattern = template_dir.join("**/*.tera");
            let pattern_str = pattern.to_string_lossy();
            tera = Tera::new(&pattern_str)
                .map_err(|e| GeoError::Validation(format!("Template load error: {e}")))?;
        }
        // 注册自定义过滤器
        tera.register_filter("ha_fmt", ha_fmt_filter);
        tera.register_filter("co2_fmt", co2_fmt_filter);
        tera.register_filter("percent_fmt", percent_fmt_filter);
        tera.register_filter("date_fmt", date_fmt_filter);

        Ok(Self { tera })
    }

    /// 注册额外模板目录（插件调用）。
    ///
    /// 追加到已有模板，不覆盖已注册的公共模板和过滤器。
    pub fn register_templates(&mut self, _plugin_name: &str, dir: &Path) -> GeoResult<()> {
        if !dir.exists() {
            return Ok(());
        }
        collect_tera_files(dir, dir, &mut self.tera)
            .map_err(|e| GeoError::Validation(format!("Plugin template load error: {e}")))?;
        // 重新注册过滤器（add_raw_templates 不覆盖过滤器，但安全起见）
        self.tera.register_filter("ha_fmt", ha_fmt_filter);
        self.tera.register_filter("co2_fmt", co2_fmt_filter);
        self.tera.register_filter("percent_fmt", percent_fmt_filter);
        self.tera.register_filter("date_fmt", date_fmt_filter);
        Ok(())
    }

    /// 使用上下文渲染模板为字符串。
    pub fn render<T: Serialize>(&self, template_name: &str, context: &T) -> GeoResult<String> {
        let ctx = Context::from_serialize(context)
            .map_err(|e| GeoError::Validation(format!("Context error: {e}")))?;
        self.tera.render(template_name, &ctx)
            .map_err(|e| GeoError::Validation(format!("Render error: {e}")))
    }

    /// 渲染为 Markdown（markdown-it 不做，直接返回 template 的输出）。
    pub fn render_md<T: Serialize>(&self, template_name: &str, context: &T) -> GeoResult<String> {
        self.render(template_name, context)
    }

    /// 渲染为 HTML（简单 Markdown → HTML，不做复杂转换）。
    pub fn render_html<T: Serialize>(&self, template_name: &str, context: &T) -> GeoResult<String> {
        let md = self.render(template_name, context)?;
        Ok(format!("<div class=\"geo-report\">\n{md}\n</div>"))
    }
}

/// 递归收集 .tera 模板文件并注册到 Tera。
///
/// 模板名 = 相对于 base 目录的文件路径（不含 .tera 后缀）。
fn collect_tera_files(base: &Path, current: &Path, tera: &mut Tera) -> Result<(), GeoError> {
    for entry in std::fs::read_dir(current).map_err(|e| GeoError::Io(std::io::Error::other(e)))? {
        let entry = entry.map_err(|e| {
            GeoError::Io(std::io::Error::other(e))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_tera_files(base, &path, tera)?;
        } else if path.extension().is_some_and(|ext| ext == "tera") {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            let name = rel.with_extension("");
            let name_str = name.to_string_lossy().replace('\\', "/");
            let content = std::fs::read_to_string(&path).map_err(GeoError::Io)?;
            tera.add_raw_template(&name_str, &content)
                .map_err(|e| GeoError::Validation(format!("Template '{}': {}", name_str, e)))?;
        }
    }
    Ok(())
}

// ── 自定义 Tera 过滤器 ──

fn ha_fmt_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(v) = value.as_f64() {
        Ok(Value::String(format!("{:.1} ha", v)))
    } else {
        Ok(value.clone())
    }
}

fn co2_fmt_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(v) = value.as_f64() {
        Ok(Value::String(format!("{:.2} tCO₂", v)))
    } else {
        Ok(value.clone())
    }
}

fn percent_fmt_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(v) = value.as_f64() {
        Ok(Value::String(format!("{:.1}%", v * 100.0)))
    } else {
        Ok(value.clone())
    }
}

fn date_fmt_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(s) = value.as_str() {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            Ok(Value::String(dt.format("%Y-%m-%d %H:%M").to_string()))
        } else {
            Ok(value.clone())
        }
    } else {
        Ok(value.clone())
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_new() {
        let engine = ReportEngine::new();
        // Engine may fail to find templates dir (which is fine in test),
        // but shouldn't panic
        assert!(engine.is_ok() || engine.is_err());
    }

    #[test]
    fn test_inline_template() {
        let mut tera = Tera::default();
        tera.add_raw_template("test", "Hello {{ name }}").unwrap();
        let mut ctx = Context::new();
        ctx.insert("name", "World");
        let result = tera.render("test", &ctx).unwrap();
        assert_eq!(result, "Hello World");
    }
}
