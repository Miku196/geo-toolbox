use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::warn;

/// A single keyword routing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordRule {
    /// Tool name to return
    pub tool: String,
    /// Keywords that trigger this tool (case-insensitive match)
    pub keywords: Vec<String>,
    /// Optional default parameters
    #[serde(default)]
    pub default_params: serde_json::Value,
}

/// Keyword router for offline fallback
#[derive(Debug, Clone)]
pub struct KeywordRouter {
    rules: Vec<KeywordRule>,
    /// keyword (lowercased) → rule index
    #[allow(dead_code)]
    index: HashMap<String, usize>,
}

impl KeywordRouter {
    /// Load rules from keywords.yaml content
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        #[derive(Deserialize)]
        struct Config {
            rules: Vec<KeywordRule>,
        }

        let config: Config = serde_yaml::from_str(yaml)?;
        let mut index = HashMap::new();

        for (i, rule) in config.rules.iter().enumerate() {
            for kw in &rule.keywords {
                index.insert(kw.to_lowercase(), i);
            }
        }

        Ok(KeywordRouter {
            rules: config.rules,
            index,
        })
    }

    /// Find the best matching tool for a query.
    /// Returns (tool_name, default_params) or None if no match.
    pub fn match_query(&self, query: &str) -> Option<(String, serde_json::Value)> {
        let lower = query.to_lowercase();

        // Score each rule by number of keyword matches
        let mut scores: Vec<(usize, usize)> = Vec::new();
        for (i, rule) in self.rules.iter().enumerate() {
            let mut score = 0;
            for kw in &rule.keywords {
                if lower.contains(&kw.to_lowercase()) {
                    score += kw.len(); // longer keywords = better match
                }
            }
            if score > 0 {
                scores.push((i, score));
            }
        }

        scores.sort_by_key(|score| std::cmp::Reverse(score.1));

        if let Some((idx, score)) = scores.first() {
            warn!(
                tool = %self.rules[*idx].tool,
                score = score,
                "FALLBACK: API fallback to keyword routing"
            );
            Some((
                self.rules[*idx].tool.clone(),
                self.rules[*idx].default_params.clone(),
            ))
        } else {
            None
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_matching() {
        let yaml = r#"
rules:
  - tool: calculate_ndvi
    keywords: ["ndvi", "植被", "vegetation index", "归一化植被指数"]
    default_params: {}
  - tool: calculate_slope
    keywords: ["slope", "坡度", "dem", "高程"]
    default_params: {"band": 1}
  - tool: carbon_scenario
    keywords: ["碳", "carbon", "carbon stock", "森林碳汇"]
    default_params: {}
"#;

        let router = KeywordRouter::from_yaml(yaml).unwrap();

        assert!(router.match_query("计算NDVI").is_some());
        assert!(router.match_query("vegetation index calculation").is_some());
        assert!(router.match_query("hello world").is_none());
        assert_eq!(
            router.match_query("slope坡度分析").unwrap().0,
            "calculate_slope"
        );
    }
}
