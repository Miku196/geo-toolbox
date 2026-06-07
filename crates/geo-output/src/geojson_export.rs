//! GeoJSON file export from PostGIS spatial queries.
//!
//! High-performance FeatureCollection generation using PostGIS's
//! built-in `ST_AsGeoJSON` aggregation.

use geo_core::errors::{GeoError, GeoResult};
use sqlx::postgres::PgPool;
use sqlx::Row;

/// Exports PostGIS query results to GeoJSON files.
pub struct GeoJsonExporter {
    pool: PgPool,
}

impl GeoJsonExporter {
    /// Create a new GeoJSON exporter.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Export query results as a GeoJSON FeatureCollection.
    ///
    /// Uses PostGIS `jsonb_build_object` + `ST_AsGeoJSON` for fast
    /// server-side GeoJSON generation (avoids Rust-side serialization).
    ///
    /// ## Example SQL
    /// ```sql
    /// SELECT
    ///   jsonb_build_object(
    ///     'type', 'Feature',
    ///     'geometry', ST_AsGeoJSON(geom)::jsonb,
    ///     'properties', properties
    ///   ) AS feature
    /// FROM spatial_assets
    /// ```
    pub async fn from_sql(&self, sql: &str, output_path: &str) -> GeoResult<usize> {
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let features: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| {
                let json_str: Option<String> = row.try_get("feature").ok();
                json_str.and_then(|s| serde_json::from_str(&s).ok())
            })
            .collect();

        let fc = serde_json::json!({
            "type": "FeatureCollection",
            "features": features,
        });

        let json = serde_json::to_string_pretty(&fc)?;
        tokio::fs::write(output_path, &json).await?;

        let count = fc["features"].as_array().map(|a| a.len()).unwrap_or(0);
        tracing::info!("GeoJSON exported: {output_path} ({count} features)");

        Ok(count)
    }

    /// Export a query result as raw GeoJSON using PostGIS aggregation.
    ///
    /// This is the fastest path: PostGIS does all the work in one
    /// `jsonb_agg(ST_AsGeoJSON(...))` call.
    ///
    /// ## Example
    /// ```sql
    /// SELECT jsonb_build_object(
    ///   'type', 'FeatureCollection',
    ///   'features', jsonb_agg(
    ///     jsonb_build_object(
    ///       'type', 'Feature',
    ///       'geometry', ST_AsGeoJSON(geom)::jsonb,
    ///       'properties', properties
    ///     )
    ///   )
    /// ) AS geojson
    /// FROM spatial_assets
    /// WHERE aoi_id = '...'
    /// ```
    pub async fn from_aggregate_sql(&self, sql: &str, output_path: &str) -> GeoResult<usize> {
        let geojson_str: Option<String> = sqlx::query_scalar(sql)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?
            .flatten();

        let geojson_str = geojson_str.unwrap_or_else(|| {
            r#"{"type":"FeatureCollection","features":[]}"#.to_string()
        });

        // Pretty-print
        let parsed: serde_json::Value = serde_json::from_str(&geojson_str)?;
        let pretty = serde_json::to_string_pretty(&parsed)?;
        tokio::fs::write(output_path, &pretty).await?;

        let count = parsed["features"].as_array().map(|a| a.len()).unwrap_or(0);
        tracing::info!("GeoJSON exported: {output_path} ({count} features)");
        Ok(count)
    }

    /// Export an entire AOI as GeoJSON with all properties.
    pub async fn export_aoi(
        &self,
        aoi_id: uuid::Uuid,
        output_path: &str,
    ) -> GeoResult<usize> {
        let sql = format!(
            r#"
            SELECT jsonb_build_object(
                'type', 'FeatureCollection',
                'features', COALESCE(jsonb_agg(
                    jsonb_build_object(
                        'type', 'Feature',
                        'geometry', ST_AsGeoJSON(geom)::jsonb,
                        'properties', properties
                    )
                ), '[]'::jsonb)
            ) AS geojson
            FROM spatial_assets
            WHERE aoi_id = '{aoi_id}'::uuid
            "#
        );

        self.from_aggregate_sql(&sql, output_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_fc() {
        let fc = serde_json::json!({
            "type": "FeatureCollection",
            "features": []
        });
        assert_eq!(fc["type"], "FeatureCollection");
        assert!(fc["features"].as_array().unwrap().is_empty());
    }
}
