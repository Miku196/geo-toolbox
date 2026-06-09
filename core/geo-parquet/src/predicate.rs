//! Spatial predicate and filter types.
//!
//! Enables predicate pushdown: filter features by spatial extent
//! before reading full data from storage (critical for cloud-native
//! performance on object storage).

use serde::{Serialize, Deserialize};

/// Filter features based on spatial predicates.
///
/// Applied at the Parquet row-group level using GeoParquet
/// bounding box metadata — rows from non-intersecting groups
/// are skipped without reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialFilter {
    /// Filter to features intersecting this bounding box.
    Bbox {
        /// Minimum longitude/x.
        min_x: f64,
        /// Minimum latitude/y.
        min_y: f64,
        /// Maximum longitude/x.
        max_x: f64,
        /// Maximum latitude/y.
        max_y: f64,
    },

    /// Filter to features within radius of a point (meters).
    Radius {
        /// Center longitude.
        center_x: f64,
        /// Center latitude.
        center_y: f64,
        /// Radius in meters.
        radius_m: f64,
    },

    /// Filter to features matching exact geometry types.
    GeometryTypes {
        /// Allowed geometry types, e.g. ["Polygon", "MultiPolygon"].
        types: Vec<String>,
    },
}

/// Result of evaluating a spatial predicate against a bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialPredicate {
    /// The bbox definitely intersects the filter — read this data.
    Intersects,
    /// The bbox definitely does NOT intersect — skip this data.
    Disjoint,
    /// Cannot determine from bbox alone — must read and check.
    Unknown,
}

impl SpatialFilter {
    /// Evaluate this filter against a bounding box.
    ///
    /// Returns `Intersects`, `Disjoint`, or `Unknown`.
    /// Used for row-group-level predicate pushdown.
    pub fn evaluate_bbox(&self, bbox_min_x: f64, bbox_min_y: f64, bbox_max_x: f64, bbox_max_y: f64) -> SpatialPredicate {
        match self {
            SpatialFilter::Bbox { min_x, min_y, max_x, max_y } => {
                if bbox_max_x >= *min_x
                    && bbox_min_x <= *max_x
                    && bbox_max_y >= *min_y
                    && bbox_min_y <= *max_y
                {
                    SpatialPredicate::Intersects
                } else {
                    SpatialPredicate::Disjoint
                }
            }
            SpatialFilter::Radius { center_x, center_y, radius_m } => {
                // Convert radius from meters to approximate degrees
                let radius_deg = *radius_m / 111_320.0;
                let expanded_min_x = center_x - radius_deg;
                let expanded_min_y = center_y - radius_deg;
                let expanded_max_x = center_x + radius_deg;
                let expanded_max_y = center_y + radius_deg;

                if bbox_max_x >= expanded_min_x
                    && bbox_min_x <= expanded_max_x
                    && bbox_max_y >= expanded_min_y
                    && bbox_min_y <= expanded_max_y
                {
                    // Intersects the expanded bbox — need to check exact distance
                    SpatialPredicate::Unknown
                } else {
                    SpatialPredicate::Disjoint
                }
            }
            SpatialFilter::GeometryTypes { .. } => {
                // Cannot filter by geometry type at bbox level
                SpatialPredicate::Unknown
            }
        }
    }

    /// Returns true if this filter can skip reading geometry data entirely.
    pub fn requires_geometry(&self) -> bool {
        matches!(self, SpatialFilter::Radius { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbox_filter_intersects() {
        let filter = SpatialFilter::Bbox {
            min_x: 103.0, min_y: 30.0, max_x: 105.0, max_y: 31.0,
        };

        // Data bbox fully inside filter
        assert_eq!(
            filter.evaluate_bbox(103.5, 30.2, 104.5, 30.8),
            SpatialPredicate::Intersects
        );

        // Data bbox overlaps filter
        assert_eq!(
            filter.evaluate_bbox(102.0, 30.2, 104.0, 30.8),
            SpatialPredicate::Intersects
        );
    }

    #[test]
    fn test_bbox_filter_disjoint() {
        let filter = SpatialFilter::Bbox {
            min_x: 103.0, min_y: 30.0, max_x: 105.0, max_y: 31.0,
        };

        // Data bbox completely outside
        assert_eq!(
            filter.evaluate_bbox(106.0, 30.2, 107.0, 30.8),
            SpatialPredicate::Disjoint
        );
    }

    #[test]
    fn test_radius_filter() {
        let filter = SpatialFilter::Radius {
            center_x: 104.0, center_y: 30.5, radius_m: 5000.0,
        };

        // Bbox completely inside radius → Unknown (need geometry check)
        assert_eq!(
            filter.evaluate_bbox(103.95, 30.45, 104.05, 30.55),
            SpatialPredicate::Unknown
        );

        // Bbox very far away → Disjoint
        assert_eq!(
            filter.evaluate_bbox(110.0, 30.5, 111.0, 31.5),
            SpatialPredicate::Disjoint
        );
    }
}
