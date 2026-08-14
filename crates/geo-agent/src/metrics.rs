use crate::providers::TokenUsage;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
/// Daily usage record
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DayUsage {
    pub date: String,
    pub calls: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    /// Estimated cost in USD (approximate)
    pub estimated_cost_usd: f64,
    /// Provider breakdown
    pub by_provider: Vec<ProviderUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub provider: String,
    pub model: String,
    pub calls: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
}

/// Cost per 1M tokens (approximate, as of 2025)
const COST_PER_M_TOKEN: fn(&str, &str) -> (f64, f64) = |provider: &str, model: &str| match provider {
    "openai" => match model {
        "gpt-4o" => (2.50, 10.00),
        "gpt-4o-mini" => (0.15, 0.60),
        _ => (2.50, 10.00),
    },
    "claude" => match model {
        "claude-sonnet-4-20250514" => (3.00, 15.00),
        "claude-haiku-3-5-20241022" => (0.80, 4.00),
        _ => (3.00, 15.00),
    },
    "deepseek" => match model {
        "deepseek-chat" => (0.27, 1.10),
        _ => (0.27, 1.10),
    },
    _ => (1.00, 5.00),
};

pub struct MetricsStore {
    log_dir: PathBuf,
    today: Mutex<DayUsage>,
}

impl MetricsStore {
    pub fn new(log_dir: PathBuf) -> Self {
        fs::create_dir_all(&log_dir).ok();
        let today_str = Local::now().format("%Y-%m-%d").to_string();

        // Try to load today's existing log
        let today = Self::load_day(&log_dir, &today_str).unwrap_or(DayUsage {
            date: today_str,
            ..Default::default()
        });

        MetricsStore {
            log_dir,
            today: Mutex::new(today),
        }
    }

    fn load_day(log_dir: &PathBuf, date: &str) -> Option<DayUsage> {
        let path = log_dir.join(format!("usage-{}.json", date));
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        }
    }

    fn save_day(log_dir: &PathBuf, usage: &DayUsage) {
        let path = log_dir.join(format!("usage-{}.json", usage.date));
        if let Ok(json) = serde_json::to_string_pretty(usage) {
            let _ = fs::write(&path, json);
        }
    }

    /// Record a new API call
    pub fn record(&self, provider: &str, model: &str, usage: &TokenUsage, fallback: bool) {
        let today_str = Local::now().format("%Y-%m-%d").to_string();
        if fallback {
            return; // don't count fallback routes as API calls
        }

        let (input_cost, output_cost) = COST_PER_M_TOKEN(provider, model);
        let cost = (usage.prompt_tokens as f64 / 1_000_000.0) * input_cost
            + (usage.completion_tokens as f64 / 1_000_000.0) * output_cost;

        let mut today = self.today.lock().unwrap();

        // Handle date rollover
        if today.date != today_str {
            Self::save_day(&self.log_dir, &today);
            *today = DayUsage {
                date: today_str,
                ..Default::default()
            };
        }

        today.calls += 1;
        today.prompt_tokens += usage.prompt_tokens;
        today.completion_tokens += usage.completion_tokens;
        today.total_tokens += usage.total_tokens;
        today.estimated_cost_usd += cost;

        // Update or create provider entry
        let prov = today.by_provider.iter_mut().find(|p| p.provider == provider && p.model == model);
        if let Some(p) = prov {
            p.calls += 1;
            p.prompt_tokens += usage.prompt_tokens;
            p.completion_tokens += usage.completion_tokens;
            p.total_tokens += usage.total_tokens;
            p.estimated_cost_usd += cost;
        } else {
            today.by_provider.push(ProviderUsage {
                provider: provider.to_string(),
                model: model.to_string(),
                calls: 1,
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
                estimated_cost_usd: cost,
            });
        }

        // Persist
        Self::save_day(&self.log_dir, &today);
    }

    /// Get today's usage snapshot
    pub fn snapshot(&self) -> DayUsage {
        self.today.lock().unwrap().clone()
    }
}

/// Write a line to the text usage log
pub fn log_to_file(log_dir: &PathBuf, line: &str) {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let path = log_dir.join(format!("usage-{}.log", today));
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{}", line);
    }
}
