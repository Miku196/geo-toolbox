//! Forest site index estimation.
//!
//! Site index = dominant height at a reference age (typically 20 years).
//! Supports multiple height-growth models and species-specific lookups.

use std::f64::consts::E;

/// Richards growth model: H = a * (1 - exp(-b * age))^c
pub fn site_index_richards(age: f64, a: f64, b: f64, c: f64) -> f64 {
    if age <= 0.0 || a <= 0.0 {
        return 0.0;
    }
    a * (1.0 - (-b * age).exp()).powf(c)
}

/// Logistic growth model: H = a / (1 + b * exp(-c * age))
pub fn site_index_logistic(age: f64, a: f64, b: f64, c: f64) -> f64 {
    if age <= 0.0 || a <= 0.0 {
        return 0.0;
    }
    a / (1.0 + b * (-c * age).exp())
}

/// Species-specific site index at base age 20 (meters).
#[derive(Debug, Clone)]
pub struct SpeciesParams {
    pub name: &'static str,
    pub site_classes: [f64; 5], // I, II, III, IV, V at base age 20
    pub model_type: ModelType,
    pub a: f64, // asymptote base
    pub b: f64, // growth rate
    pub c: f64, // shape
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelType {
    Richards,
    Logistic,
}

/// Get species parameters.
fn species_params(species: &str) -> Option<SpeciesParams> {
    match species.to_lowercase().as_str() {
        "pinus_massoniana" | "pinus massoniana" | "masson pine" => Some(SpeciesParams {
            name: "pinus_massoniana",
            site_classes: [20.0, 16.0, 12.0, 8.0, 5.0],
            model_type: ModelType::Richards,
            a: 25.0,
            b: 0.15,
            c: 1.5,
        }),
        "cunninghamia" | "cunninghamia lanceolata" | "chinese fir" => Some(SpeciesParams {
            name: "cunninghamia",
            site_classes: [22.0, 17.5, 13.0, 9.0, 5.5],
            model_type: ModelType::Richards,
            a: 28.0,
            b: 0.12,
            c: 1.6,
        }),
        "eucalyptus" | "eucalyptus spp" => Some(SpeciesParams {
            name: "eucalyptus",
            site_classes: [25.0, 20.0, 15.0, 10.0, 6.0],
            model_type: ModelType::Logistic,
            a: 30.0,
            b: 8.0,
            c: 0.25,
        }),
        "quercus" | "oak" => Some(SpeciesParams {
            name: "quercus",
            site_classes: [18.0, 14.0, 10.5, 7.0, 4.0],
            model_type: ModelType::Richards,
            a: 22.0,
            b: 0.10,
            c: 1.4,
        }),
        "poplar" | "populus" => Some(SpeciesParams {
            name: "poplar",
            site_classes: [28.0, 22.0, 16.0, 10.0, 6.0],
            model_type: ModelType::Richards,
            a: 35.0,
            b: 0.18,
            c: 1.3,
        }),
        "larix" | "larch" => Some(SpeciesParams {
            name: "larix",
            site_classes: [21.0, 17.0, 13.0, 9.0, 5.5],
            model_type: ModelType::Richards,
            a: 26.0,
            b: 0.14,
            c: 1.5,
        }),
        _ => None,
    }
}

/// Look up site index for a species and site class at a given base age.
pub fn site_index_lookup(species: &str, site_class: &str, base_age: u32) -> f64 {
    let params = match species_params(species) {
        Some(p) => p,
        None => return 0.0,
    };

    let class_idx = match site_class.to_uppercase().as_str() {
        "I" | "1" | "GOOD" => 0usize,
        "II" | "2" | "MODERATE" => 1,
        "III" | "3" | "MEDIUM" => 2,
        "IV" | "4" | "POOR" => 3,
        "V" | "5" | "VERY POOR" => 4,
        _ => 2, // default medium
    };

    let si = params.site_classes[class_idx];
    // Scale to requested base age if different from 20
    if base_age == 20 {
        return si;
    }
    let age_f = base_age as f64;
    match params.model_type {
        ModelType::Richards => {
            // Invert: find a,b,c that produce si at age 20, then evaluate at new age
            let h20 = site_index_richards(20.0, params.a, params.b, params.c);
            if h20 <= 0.0 {
                return si;
            }
            let ratio = si / h20;
            site_index_richards(age_f, params.a, params.b, params.c) * ratio
        }
        ModelType::Logistic => {
            let h20 = site_index_logistic(20.0, params.a, params.b, params.c);
            if h20 <= 0.0 {
                return si;
            }
            let ratio = si / h20;
            site_index_logistic(age_f, params.a, params.b, params.c) * ratio
        }
    }
}

/// Determine site class from measured height.
pub fn site_class_from_height(
    measured_height_m: f64,
    age: f64,
    species: &str,
) -> serde_json::Value {
    let params = match species_params(species) {
        Some(p) => p,
        None => {
            return serde_json::json!({
                "error": format!("Unknown species: {species}")
            })
        }
    };

    // Expected heights at current age for each class
    let mut expected: Vec<(String, f64)> = Vec::with_capacity(5);
    let class_names = ["I", "II", "III", "IV", "V"];
    for (i, &si) in params.site_classes.iter().enumerate() {
        // Project from base age 20 to current age
        let h20 = match params.model_type {
            ModelType::Richards => site_index_richards(20.0, params.a, params.b, params.c),
            ModelType::Logistic => site_index_logistic(20.0, params.a, params.b, params.c),
        };
        let ratio = if h20 > 0.0 { si / h20 } else { 1.0 };
        let height_at_age = match params.model_type {
            ModelType::Richards => site_index_richards(age, params.a, params.b, params.c) * ratio,
            ModelType::Logistic => site_index_logistic(age, params.a, params.b, params.c) * ratio,
        };
        expected.push((class_names[i].to_string(), height_at_age));
    }

    // Find closest class
    let mut best_class = "III".to_string();
    let mut best_si = params.site_classes[2];
    let mut min_diff = f64::MAX;
    for (name, h) in &expected {
        let diff = (measured_height_m - h).abs();
        if diff < min_diff {
            min_diff = diff;
            best_class = name.clone();
            best_si =
                params.site_classes[expected.iter().position(|(n, _)| n == name).unwrap_or(2)];
        }
    }

    serde_json::json!({
        "species": species,
        "age": age,
        "measured_height_m": (measured_height_m * 100.0).round() / 100.0,
        "site_class": best_class,
        "site_index_m": best_si,
        "expected_heights": expected,
        "best_match_deviation_m": (min_diff * 100.0).round() / 100.0,
    })
}

/// Mean annual increment: MAI = site_index / rotation_age (m/yr)
pub fn site_productivity(site_index_m: f64, rotation_age: u32) -> f64 {
    if rotation_age == 0 {
        return 0.0;
    }
    site_index_m / rotation_age as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_richards_positive() {
        let h = site_index_richards(20.0, 25.0, 0.15, 1.5);
        assert!(h > 0.0 && h < 25.0, "got {h}");
    }

    #[test]
    fn test_richards_zero_age() {
        let h = site_index_richards(0.0, 25.0, 0.15, 1.5);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn test_logistic_s_shape() {
        let h20 = site_index_logistic(20.0, 30.0, 8.0, 0.25);
        let h50 = site_index_logistic(50.0, 30.0, 8.0, 0.25);
        assert!(h20 > 0.0 && h20 < 30.0);
        assert!(h50 > h20, "older should be taller: {h20} vs {h50}");
    }

    #[test]
    fn test_site_index_lookup_pine_i() {
        let si = site_index_lookup("pinus_massoniana", "I", 20);
        assert!((si - 20.0).abs() < 0.1, "got {si}");
    }

    #[test]
    fn test_site_index_lookup_pine_v() {
        let si = site_index_lookup("pinus_massoniana", "V", 20);
        assert!((si - 5.0).abs() < 0.1, "got {si}");
    }

    #[test]
    fn test_site_index_lookup_unknown_species() {
        let si = site_index_lookup("unknown", "I", 20);
        assert_eq!(si, 0.0);
    }

    #[test]
    fn test_site_class_from_height_match() {
        let r = site_class_from_height(20.0, 20.0, "pinus_massoniana");
        assert_eq!(r["site_class"].as_str().unwrap(), "I");
        assert!((r["site_index_m"].as_f64().unwrap() - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_site_class_from_height_poor() {
        let r = site_class_from_height(6.0, 20.0, "pinus_massoniana");
        let cls = r["site_class"].as_str().unwrap();
        // Should be IV or V
        assert!(cls == "IV" || cls == "V", "got {cls}");
    }

    #[test]
    fn test_site_productivity() {
        let mai = site_productivity(20.0, 20);
        assert!((mai - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_site_productivity_zero_rotation() {
        let mai = site_productivity(20.0, 0);
        assert_eq!(mai, 0.0);
    }
}
