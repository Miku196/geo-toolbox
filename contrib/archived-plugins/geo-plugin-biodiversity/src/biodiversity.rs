//! Biodiversity assessment: SDM, habitat connectivity, protected area GAP analysis.

use geo_core::errors::GeoResult;
use geo_core::types::BBox;
use serde::Serialize;

use crate::BiodiversityConfig;

// ── Species Distribution Model (Bioclim envelope) ────────────────

/// Environmental envelope for a species (from occurrence records).
#[derive(Debug, Clone, Serialize)]
pub struct EnvelopeModel {
    pub species: String,
    /// Number of presence points used
    pub presence_points: usize,
    /// Per-variable statistics: (name, min, max, mean)
    pub envelopes: Vec<VariableEnvelope>,
    /// Model AUC (leave-one-out cross-validation estimate)
    pub auc_estimate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariableEnvelope {
    pub variable: String,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std: f64,
}

/// A species occurrence record.
#[derive(Debug, Clone)]
pub struct Occurrence {
    pub lon: f64,
    pub lat: f64,
    pub env_values: Vec<f64>, // values for each environmental variable
}

/// FIT a bioclimatic envelope model from occurrence records.
///
/// For each environmental variable, records the min-max range (the "envelope")
/// and computes basic statistics.
pub fn fit_envelope(
    species: &str,
    occurrences: &[Occurrence],
    var_names: &[String],
) -> GeoResult<EnvelopeModel> {
    if occurrences.is_empty() {
        return Err(geo_core::errors::GeoError::Validation(
            "No occurrence records provided".into(),
        ));
    }

    let n_vars = var_names.len();
    let mut envelopes = Vec::with_capacity(n_vars);

    for v in 0..n_vars {
        let vals: Vec<f64> = occurrences.iter().map(|o| o.env_values[v]).collect();
        if vals.is_empty() {
            continue;
        }
        let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        let variance = vals.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / vals.len() as f64;
        envelopes.push(VariableEnvelope {
            variable: var_names[v].clone(),
            min,
            max,
            mean,
            std: variance.sqrt(),
        });
    }

    // Simple LOO AUC estimate: fraction of points within envelope
    let mut within = 0usize;
    let n = occurrences.len();
    if n > 1 {
        for i in 0..n {
            let mut inside = true;
            for v in 0..n_vars {
                let val = occurrences[i].env_values[v];
                // Leave-one-out: use envelope from all other points
                let other_vals: Vec<f64> = occurrences
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, o)| o.env_values[v])
                    .collect();
                if other_vals.is_empty() {
                    continue;
                }
                let loo_min = other_vals.iter().cloned().fold(f64::INFINITY, f64::min);
                let loo_max = other_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                if val < loo_min || val > loo_max {
                    inside = false;
                    break;
                }
            }
            if inside {
                within += 1;
            }
        }
    }

    Ok(EnvelopeModel {
        species: species.to_string(),
        presence_points: n,
        envelopes,
        auc_estimate: if n > 1 { within as f64 / n as f64 } else { 1.0 },
    })
}

/// Predict habitat suitability using a fitted envelope model.
/// Returns 1.0 if all variables within envelope, 0.0 otherwise.
pub fn predict_suitability(model: &EnvelopeModel, env_values: &[f64]) -> f64 {
    if model.envelopes.len() != env_values.len() {
        return 0.0;
    }
    for (i, env) in model.envelopes.iter().enumerate() {
        if env_values[i] < env.min || env_values[i] > env.max {
            return 0.0;
        }
    }
    1.0
}

/// Predict continuous habitat suitability (0-1) using a Mahalanobis-like distance.
/// Lower distance from the centroid = higher suitability.
pub fn predict_suitability_continuous(model: &EnvelopeModel, env_values: &[f64]) -> f64 {
    if model.envelopes.len() != env_values.len() {
        return 0.0;
    }
    let mut score = 0.0;
    let n = model.envelopes.len() as f64;
    for (i, env) in model.envelopes.iter().enumerate() {
        if env.std > 0.0 {
            let z = (env_values[i] - env.mean).abs() / env.std;
            score += 1.0 / (1.0 + z); // decay with distance
        } else {
            score += 1.0;
        }
    }
    score / n
}

// ── Habitat Connectivity / Fragmentation ─────────────────────────

/// Landscape patch metrics.
#[derive(Debug, Clone, Serialize)]
pub struct PatchMetrics {
    /// Number of distinct habitat patches
    pub num_patches: usize,
    /// Total habitat area
    pub total_area: f64,
    /// Mean patch area
    pub mean_patch_area: f64,
    /// Largest patch index (% of total landscape)
    pub largest_patch_index: f64,
    /// Edge density (edge length / area)
    pub edge_density: f64,
    /// Core area (area farther than edge_depth from any edge)
    pub core_area: f64,
}

/// Compute patch metrics from a binary habitat raster (1=habitat, 0=non-habitat).
///
/// Uses a simple 4-connected flood-fill approach.
pub fn compute_patch_metrics(
    habitat: &[u8],
    rows: usize,
    cols: usize,
    cell_area: f64,      // area per cell
    _edge_depth: f64,    // edge effect depth
    _min_core_area: f64, // minimum core patch area
) -> GeoResult<PatchMetrics> {
    if habitat.len() != rows * cols {
        return Err(geo_core::errors::GeoError::Validation(
            "habitat array length != rows * cols".into(),
        ));
    }

    // Flood-fill to find patches
    let mut visited = vec![false; rows * cols];
    let mut patches: Vec<usize> = Vec::new(); // area (cell count) of each patch
    let mut total_habitat_cells = 0usize;
    let mut total_edge_cells = 0usize;

    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            if habitat[idx] == 1 && !visited[idx] {
                // Flood fill
                let mut stack = vec![(r, c)];
                visited[idx] = true;
                let mut patch_cells = 0usize;
                let mut edge_cells = 0usize;

                while let Some((cr, cc)) = stack.pop() {
                    patch_cells += 1;
                    // Check if cell is within edge_depth of boundary
                    let is_edge = is_edge_cell(habitat, rows, cols, cr, cc);
                    if is_edge {
                        edge_cells += 1;
                    }

                    // 4-neighbor flood fill
                    for (nr, nc) in [
                        (cr.wrapping_sub(1), cc),
                        (cr + 1, cc),
                        (cr, cc.wrapping_sub(1)),
                        (cr, cc + 1),
                    ] {
                        if nr < rows && nc < cols {
                            let nidx = nr * cols + nc;
                            if habitat[nidx] == 1 && !visited[nidx] {
                                visited[nidx] = true;
                                stack.push((nr, nc));
                            }
                        }
                    }
                }

                patches.push(patch_cells);
                total_habitat_cells += patch_cells;
                total_edge_cells += edge_cells;
            }
        }
    }

    if patches.is_empty() {
        return Ok(PatchMetrics {
            num_patches: 0,
            total_area: 0.0,
            mean_patch_area: 0.0,
            largest_patch_index: 0.0,
            edge_density: 0.0,
            core_area: 0.0,
        });
    }

    let total_area = total_habitat_cells as f64 * cell_area;
    let mean_patch_area = total_area / patches.len() as f64;
    let max_patch_cells = *patches.iter().max().unwrap_or(&0) as f64;
    let largest_patch_index = max_patch_cells / (rows * cols) as f64;
    let total_landscape_area = (rows * cols) as f64 * cell_area;

    // Core area: non-edge cells
    let core_cells = total_habitat_cells.saturating_sub(total_edge_cells);
    let core_area = core_cells as f64 * cell_area;

    // Edge density
    let mut perimeter = 0usize;
    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            if habitat[idx] == 1 {
                for (nr, nc) in [
                    (r.wrapping_sub(1), c),
                    (r + 1, c),
                    (r, c.wrapping_sub(1)),
                    (r, c + 1),
                ] {
                    if nr >= rows || nc >= cols || habitat[nr * cols + nc] == 0 {
                        perimeter += 1;
                        break; // count each edge cell once
                    }
                }
            }
        }
    }
    let edge_density = perimeter as f64 / total_landscape_area.max(1.0);

    Ok(PatchMetrics {
        num_patches: patches.len(),
        total_area,
        mean_patch_area,
        largest_patch_index,
        edge_density,
        core_area,
    })
}

fn is_edge_cell(habitat: &[u8], rows: usize, cols: usize, r: usize, c: usize) -> bool {
    // A cell is on the edge if any 4-neighbor is non-habitat or out of bounds
    for (nr, nc) in [
        (r.wrapping_sub(1), c),
        (r + 1, c),
        (r, c.wrapping_sub(1)),
        (r, c + 1),
    ] {
        if nr >= rows || nc >= cols || habitat[nr * cols + nc] == 0 {
            return true;
        }
    }
    false
}

/// Compute a connectivity index based on patch proximity.
/// Higher values = more connected landscape (simplified nearest-neighbor ratio).
pub fn connectivity_index(patches: &[BBox], // bounding box of each patch
) -> f64 {
    if patches.len() < 2 {
        return 1.0;
    }
    let mut sum_dist = 0.0f64;
    let mut count = 0usize;
    for i in 0..patches.len() {
        let ci = bbox_center(&patches[i]);
        let mut min_dist = f64::MAX;
        for j in 0..patches.len() {
            if i == j {
                continue;
            }
            let cj = bbox_center(&patches[j]);
            let dx = ci.0 - cj.0;
            let dy = ci.1 - cj.1;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < min_dist {
                min_dist = dist;
            }
        }
        if min_dist.is_finite() {
            sum_dist += min_dist;
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let mean_nn = sum_dist / count as f64;
    // Normalize: typical range 0-100km spatial scale
    1.0 / (1.0 + mean_nn / 0.01)
}

fn bbox_center(bbox: &BBox) -> (f64, f64) {
    (
        (bbox.min_x + bbox.max_x) / 2.0,
        (bbox.min_y + bbox.max_y) / 2.0,
    )
}

// ── Protected Area GAP Analysis ──────────────────────────────────

/// GAP analysis result.
#[derive(Debug, Clone, Serialize)]
pub struct GapResult {
    pub species: String,
    /// Total range area
    pub total_range_area_km2: f64,
    /// Area already protected
    pub protected_area_km2: f64,
    /// Protection percentage
    pub protection_pct: f64,
    /// Target percentage
    pub target_pct: f64,
    /// Gap area (area still needed)
    pub gap_area_km2: f64,
    /// Whether the protection target is met
    pub target_met: bool,
    /// Recommendation
    pub recommendation: String,
}

/// Perform GAP analysis: how much of a species' range falls within protected areas.
///
/// Parameters:
/// - `species`: species name
/// - `range_polygons`: array of polygon bounding boxes representing the species range
/// - `protected_areas`: array of polygon bounding boxes representing protected areas
/// - `target_pct`: target protection percentage (e.g., 17 for 17%)
pub fn gap_analysis(
    species: &str,
    range_polygons: &[BBox],
    protected_areas: &[BBox],
    target_pct: f64,
) -> GapResult {
    // Simplified: use bounding box area overlap approximation
    let total_range: f64 = range_polygons.iter().map(|b| bbox_area_km2(b)).sum();

    let mut protected_overlap = 0.0f64;
    for rp in range_polygons {
        for pa in protected_areas {
            protected_overlap += bbox_overlap_area_km2(rp, pa);
        }
    }

    let protection_pct = if total_range > 0.0 {
        (protected_overlap / total_range * 100.0).min(100.0)
    } else {
        0.0
    };
    let gap_area = (target_pct / 100.0 * total_range - protected_overlap).max(0.0);
    let target_met = protection_pct >= target_pct;

    GapResult {
        species: species.to_string(),
        total_range_area_km2: total_range,
        protected_area_km2: protected_overlap,
        protection_pct,
        target_pct,
        gap_area_km2: gap_area,
        target_met,
        recommendation: if target_met {
            "Protection target met. Monitor for threats.".into()
        } else {
            format!(
                "Need {:.1} km² additional protection to reach {:.0}% target.",
                gap_area, target_pct
            )
        },
    }
}

/// Shannon diversity index from species abundance data.
#[derive(Debug, Clone, Serialize)]
pub struct DiversityIndex {
    pub shannon_h: f64,
    pub simpson_d: f64,
    pub species_richness: usize,
    pub evenness: f64,
}

pub fn compute_diversity(abundances: &[f64]) -> DiversityIndex {
    let total: f64 = abundances.iter().sum();
    if total <= 0.0 {
        return DiversityIndex {
            shannon_h: 0.0,
            simpson_d: 0.0,
            species_richness: 0,
            evenness: 0.0,
        };
    }
    let _n = abundances.len();
    let mut shannon = 0.0f64;
    let mut simpson = 0.0f64;
    let mut richness = 0usize;

    for &a in abundances {
        if a > 0.0 {
            richness += 1;
            let p = a / total;
            shannon -= p * p.ln();
            simpson += p * p;
        }
    }

    let evenness = if richness > 1 {
        shannon / (richness as f64).ln()
    } else if richness == 1 {
        1.0
    } else {
        0.0
    };

    DiversityIndex {
        shannon_h: shannon,
        simpson_d: simpson,
        species_richness: richness,
        evenness,
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn bbox_area_km2(bbox: &BBox) -> f64 {
    let dx = (bbox.max_x - bbox.min_x).abs() * 111_320.0 * (bbox.min_y.to_radians().cos());
    let dy = (bbox.max_y - bbox.min_y).abs() * 111_320.0;
    (dx * dy).abs() / 1_000_000.0 // m² → km²
}

fn bbox_overlap_area_km2(a: &BBox, b: &BBox) -> f64 {
    let ox = (a.max_x.min(b.max_x) - a.min_x.max(b.min_x)).max(0.0);
    let oy = (a.max_y.min(b.max_y) - a.min_y.max(b.min_y)).max(0.0);
    if ox <= 0.0 || oy <= 0.0 {
        return 0.0;
    }
    let dx = ox * 111_320.0 * (a.min_y.to_radians().cos());
    let dy = oy * 111_320.0;
    (dx * dy).abs() / 1_000_000.0
}

// ── Plugin struct ────────────────────────────────────────────────

pub struct BiodiversityPlugin {
    pub config: BiodiversityConfig,
}

impl BiodiversityPlugin {
    pub fn new(config: BiodiversityConfig) -> Self {
        Self { config }
    }

    pub fn fit_sdm(
        &self,
        species: &str,
        occurrences: &[Occurrence],
        var_names: &[String],
    ) -> GeoResult<EnvelopeModel> {
        fit_envelope(species, occurrences, var_names)
    }

    pub fn assess_habitat(
        &self,
        habitat: &[u8],
        rows: usize,
        cols: usize,
        cell_area: f64,
    ) -> GeoResult<PatchMetrics> {
        compute_patch_metrics(
            habitat,
            rows,
            cols,
            cell_area,
            self.config.connectivity.edge_depth,
            self.config.connectivity.min_core_area,
        )
    }

    pub fn gap_analysis(
        &self,
        species: &str,
        range_polygons: &[BBox],
        protected_areas: &[BBox],
    ) -> GapResult {
        gap_analysis(
            species,
            range_polygons,
            protected_areas,
            self.config.gap.target_pct,
        )
    }

    pub fn diversity(&self, abundances: &[f64]) -> DiversityIndex {
        compute_diversity(abundances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fit_envelope() {
        let occs = vec![
            Occurrence {
                lon: 104.0,
                lat: 30.5,
                env_values: vec![20.0, 1000.0],
            },
            Occurrence {
                lon: 104.1,
                lat: 30.6,
                env_values: vec![22.0, 1100.0],
            },
            Occurrence {
                lon: 103.9,
                lat: 30.4,
                env_values: vec![18.0, 900.0],
            },
        ];
        let vars = vec!["temp".into(), "precip".into()];
        let model = fit_envelope("test_sp", &occs, &vars).unwrap();
        assert_eq!(model.envelopes.len(), 2);
        assert_eq!(model.envelopes[0].min, 18.0);
        assert_eq!(model.envelopes[0].max, 22.0);
        // Within envelope
        assert!(predict_suitability(&model, &[20.0, 1000.0]) > 0.0);
        // Outside envelope
        assert_eq!(predict_suitability(&model, &[30.0, 500.0]), 0.0);
    }

    #[test]
    fn test_patch_metrics() {
        // 5x5 grid: habitat in top-left 2x2 block
        let mut habitat = vec![0u8; 25];
        habitat[0] = 1;
        habitat[1] = 1;
        habitat[5] = 1;
        habitat[6] = 1;
        let metrics = compute_patch_metrics(&habitat, 5, 5, 1.0, 10.0, 1.0).unwrap();
        assert_eq!(metrics.num_patches, 1);
        assert_eq!(metrics.total_area, 4.0);
    }

    #[test]
    fn test_gap_analysis() {
        let range = vec![BBox::new(104.0, 30.0, 105.0, 31.0)];
        let pas = vec![BBox::new(104.2, 30.2, 104.6, 30.6)];
        let result = gap_analysis("test_sp", &range, &pas, 17.0);
        assert!(result.protection_pct > 0.0);
    }

    #[test]
    fn test_diversity() {
        let div = compute_diversity(&[10.0, 20.0, 30.0, 0.0]);
        assert!(div.shannon_h > 0.0);
        assert!(div.simpson_d < 1.0);
        assert_eq!(div.species_richness, 3);
    }

    #[test]
    fn test_connectivity() {
        let patches = vec![
            BBox::new(0.0, 0.0, 1.0, 1.0),
            BBox::new(0.5, 0.5, 1.5, 1.5),
            BBox::new(10.0, 10.0, 11.0, 11.0),
        ];
        let ci = connectivity_index(&patches);
        assert!(ci > 0.0 && ci <= 1.0);
    }
}
