//! Emission factor methodology (IPCC Tier 1).
//!
//! The core carbon accounting engine. One SQL query performs
//! spatial aggregation + factor lookup + area calculation + emission
//! computation in a single round-trip to PostGIS.

use geo_core::errors::{GeoError, GeoResult};
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::audit::AuditTrail;

/// Result of one emission calculation for a landcover class.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmissionResult {
    /// Unique calculation ID (written to DB).
    pub calc_id: Uuid,
    /// AOI identifier.
    pub aoi_id: Uuid,
    /// Target year.
    pub year: u16,
    /// Landcover class (e.g. "forest", "grassland").
    pub landcover_class: String,
    /// Total area in hectares (EPSG:3405).
    pub area_ha: f64,
    /// Emission factor value (tCO₂e/ha/yr).
    pub factor_value: f64,
    /// Total emissions (area × factor).
    pub emission_tco2e: f64,
    /// Source of the factor (e.g. "IPCC_2019").
    pub factor_source: String,
    /// UUID of the factor_set used.
    pub factor_set_id: Uuid,
    /// Full audit trail for traceability.
    pub audit: AuditTrail,
}

/// Row as returned by the calculation SQL query.
#[derive(Debug, sqlx::FromRow)]
pub struct EmissionFactorRow {
    /// Landcover class.
    pub landcover_class: String,
    /// Total area in hectares.
    pub total_area_ha: f64,
    /// Emission factor value.
    pub factor_value: f64,
    /// Emission tCO₂e.
    pub emission_tco2e: f64,
    /// Factor source name.
    pub factor_source: String,
    /// Factor set UUID.
    pub factor_set_id: Uuid,
    /// DVC hash of remote sensing data.
    pub lc_dvc_hash: Option<String>,
    /// DVC hash of emission factor data.
    pub factor_dvc_hash: Option<String>,
}

/// Summary of a registered emission factor.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct FactorInfo {
    /// UUID.
    pub factor_set_id: Uuid,
    /// Source name.
    pub source: String,
    /// Landcover category.
    pub category: String,
    /// Factor value.
    pub factor_value: f64,
    /// Unit.
    pub unit: String,
    /// Valid from year.
    pub valid_from_year: i32,
    /// Valid to year (None =至今有效).
    pub valid_to_year: Option<i32>,
    /// Geographic region.
    pub region: Option<String>,
}

/// Input parameters for registering a new emission factor.
#[derive(Debug, Clone)]
pub struct FactorInput {
    /// Source name (e.g., IPCC_2019).
    pub source: String,
    /// Land cover category.
    pub category: String,
    /// Factor value in tCO₂e/ha (negative = carbon sink).
    pub factor_value: f64,
    /// Unit of measurement.
    pub unit: String,
    /// Year the factor becomes valid.
    pub valid_from_year: i32,
    /// Year the factor expires (None = no expiry).
    pub valid_to_year: Option<i32>,
    /// Geographic region code (e.g., CN-51).
    pub region: Option<String>,
}

/// The carbon accounting engine.
pub struct CarbonEngine {
    pool: PgPool,
}

impl CarbonEngine {
    /// Create a new engine with a database pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ── Emission factor calculation ──────────────────────────────

    /// Calculate emissions using the emission factor method.
    ///
    /// Full pipeline in one SQL query:
    /// 1. Read landcover polygons from `spatial_assets` for the AOI
    /// 2. Join with `factor_registry` by class + year
    /// 3. Compute area in EPSG:3405 (equal-area projection)
    /// 4. Multiply area × factor → tCO₂e
    /// 5. Write results to `carbon_accounting_results`
    /// 6. Return results with full audit trail
    pub async fn calculate_emission_factor(
        &self,
        aoi_id: Uuid,
        year: u16,
        factor_source: &str,
    ) -> GeoResult<Vec<EmissionResult>> {
        let year_i32 = year as i32;

        let rows = self.query_calculation_rows(aoi_id, year_i32, factor_source).await?;

        if rows.is_empty() {
            return Err(GeoError::Validation(format!(
                "No landcover data found for AOI {aoi_id} in year {year}\n\
                 Hint: ensure spatial_assets has rows with aoi_id={aoi_id} and properties->>'class' set"
            )));
        }

        // Write results to DB
        let workflow_run_id = Uuid::new_v4();
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            let calc_id = Uuid::new_v4();

            sqlx::query(
                r#"
                INSERT INTO carbon_accounting_results
                    (calc_id, workflow_run_id, aoi_id,
                     landcover_src, landcover_class,
                     lc_dvc_hash, factor_set_id,
                     area_ha, emission_tco2e, audit_status)
                VALUES ($1, $2, $3, 'GEE_RF', $4, $5, $6, $7, $8, 'pending')
                "#,
            )
            .bind(calc_id)
            .bind(workflow_run_id)
            .bind(aoi_id)
            .bind(&row.landcover_class)
            .bind(&row.lc_dvc_hash)
            .bind(row.factor_set_id)
            .bind(row.total_area_ha)
            .bind(row.emission_tco2e)
            .execute(&self.pool)
            .await
            .map_err(|e| GeoError::Database(format!("insert result: {e}")))?;

            results.push(self.build_result(aoi_id, year, row, calc_id));
        }

        let total_tco2e: f64 = results.iter().map(|r| r.emission_tco2e).sum();
        tracing::info!(
            "Carbon: {:.1} tCO₂e total ({} classes, AOI={aoi_id}, year={year})",
            total_tco2e, results.len(),
        );

        Ok(results)
    }

    /// Dry-run: calculate without writing to the database.
    ///
    /// Useful for previewing results before committing, or for
    /// scenarios where `carbon_accounting_results` table doesn't exist.
    pub async fn calculate_dry_run(
        &self,
        aoi_id: Uuid,
        year: u16,
        factor_source: &str,
    ) -> GeoResult<Vec<EmissionResult>> {
        let year_i32 = year as i32;
        let rows = self.query_calculation_rows(aoi_id, year_i32, factor_source).await?;

        if rows.is_empty() {
            return Err(GeoError::Validation(format!(
                "No landcover data found for AOI {aoi_id} in year {year}"
            )));
        }

        Ok(rows
            .iter()
            .map(|r| {
                let calc_id = Uuid::new_v4();
                self.build_result(aoi_id, year, r, calc_id)
            })
            .collect())
    }

    // ── Factor registry ──────────────────────────────────────────

    /// Register a single emission factor.
    pub async fn register_factor(&self, input: FactorInput) -> GeoResult<Uuid> {
        let region_key = input.region.as_deref().unwrap_or("__GLOBAL__");

        let overlap: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT factor_set_id FROM factor_registry
            WHERE source = $1
              AND category = $2
              AND COALESCE(region, '__GLOBAL__') = $3
              AND int4range(valid_from_year, COALESCE(valid_to_year, 9999), '[]')
                  && int4range($4, COALESCE($5, 9999), '[]')
            LIMIT 1
            "#,
        )
        .bind(&input.source)
        .bind(&input.category)
        .bind(region_key)
        .bind(input.valid_from_year)
        .bind(input.valid_to_year)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        if let Some((existing_id,)) = overlap {
            let to_str = input.valid_to_year.map_or("∞".to_string(), |y| y.to_string());
            return Err(GeoError::Validation(format!(
                "Overlapping factor already exists: {existing_id}\n\
                 source={src} category={cat} region={rkey} [{from}..{to})",
                src = input.source, cat = input.category, rkey = region_key, from = input.valid_from_year, to = to_str
            )));
        }

        let factor_set_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO factor_registry
                (factor_set_id, source, category, factor_value, unit,
                 valid_from_year, valid_to_year, region)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(factor_set_id)
        .bind(&input.source)
        .bind(&input.category)
        .bind(input.factor_value)
        .bind(&input.unit)
        .bind(input.valid_from_year)
        .bind(input.valid_to_year)
        .bind(input.region.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        tracing::info!(
            "Registered factor {factor_set_id}: {}/{cat} = {val} {unit}",
            input.source, cat = input.category, val = input.factor_value, unit = input.unit
        );
        Ok(factor_set_id)
    }

    /// Import emission factors from a CSV file.
    ///
    /// Columns: `source,category,factor_value,unit,valid_from_year,valid_to_year,region`
    ///
    /// Skips rows that would violate the EXCLUDE constraint (overlapping time ranges).
    pub async fn import_factors_csv(&self, csv_path: &str) -> GeoResult<usize> {
        let mut reader = csv::Reader::from_path(csv_path)
            .map_err(|e| GeoError::Csv(e.to_string()))?;

        let mut imported = 0usize;
        let mut skipped = 0usize;

        for result in reader.deserialize() {
            let record: serde_json::Value = result
                .map_err(|e| GeoError::Csv(e.to_string()))?;

            let source = record["source"].as_str().unwrap_or("IPCC_2019");
            let category = record["category"].as_str().unwrap_or("unknown");
            let factor_value: f64 = record["factor_value"].as_f64().unwrap_or(0.0);
            let unit = record["unit"].as_str().unwrap_or("tCO2e/ha/yr");
            let valid_from: i32 = record["valid_from_year"].as_i64().unwrap_or(2000) as i32;
            let valid_to: Option<i32> = record["valid_to_year"].as_i64().map(|v| v as i32);
            let region = record["region"].as_str();

            match self
                .register_factor(FactorInput {
                    source: source.to_string(),
                    category: category.to_string(),
                    factor_value,
                    unit: unit.to_string(),
                    valid_from_year: valid_from,
                    valid_to_year: valid_to,
                    region: region.map(|s| s.to_string()),
                })
                .await
            {
                Ok(_) => imported += 1,
                Err(e) => {
                    tracing::warn!("Skipping {source}/{category}: {e}");
                    skipped += 1;
                }
            }
        }

        tracing::info!("Imported {imported} factors from {csv_path} ({skipped} skipped)");
        Ok(imported)
    }

    /// Query all emission factors valid for a given year.
    pub async fn query_factors(
        &self,
        year: i32,
        source_filter: Option<&str>,
    ) -> GeoResult<Vec<FactorInfo>> {
        let rows: Vec<FactorInfo> = sqlx::query_as(
            r#"
            SELECT
                factor_set_id, source, category, factor_value,
                unit, valid_from_year, valid_to_year, region
            FROM factor_registry
            WHERE valid_from_year <= $1
              AND (valid_to_year IS NULL OR valid_to_year >= $1)
              AND ($2::text IS NULL OR source = $2)
            ORDER BY category, source
            "#,
        )
        .bind(year)
        .bind(source_filter)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        Ok(rows)
    }

    /// Query historical calculations for an AOI.
    pub async fn query_by_aoi(
        &self,
        aoi_id: Uuid,
    ) -> GeoResult<Vec<EmissionResult>> {
        let rows = sqlx::query_as::<_, EmissionFactorRow>(
            r#"
            SELECT
                car.landcover_class,
                car.area_ha AS total_area_ha,
                0.0 AS factor_value,
                car.emission_tco2e,
                '' AS factor_source,
                car.factor_set_id,
                car.lc_dvc_hash,
                '' AS factor_dvc_hash
            FROM carbon_accounting_results car
            WHERE car.aoi_id = $1
            ORDER BY car.calculation_at DESC
            "#,
        )
        .bind(aoi_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| EmissionResult {
                calc_id: Uuid::nil(),
                aoi_id,
                year: 0,
                landcover_class: r.landcover_class,
                area_ha: r.total_area_ha,
                factor_value: r.factor_value,
                emission_tco2e: r.emission_tco2e,
                factor_source: r.factor_source,
                factor_set_id: r.factor_set_id,
                audit: AuditTrail {
                    lc_dvc_hash: r.lc_dvc_hash,
                    factor_dvc_hash: r.factor_dvc_hash,
                    factor_set_id: r.factor_set_id.to_string(),
                },
            })
            .collect())
    }

    // ── Internal helpers ─────────────────────────────────────────

    async fn query_calculation_rows(
        &self,
        aoi_id: Uuid,
        year_i32: i32,
        factor_source: &str,
    ) -> GeoResult<Vec<EmissionFactorRow>> {
        sqlx::query_as(
            r#"
            WITH landcover AS (
                SELECT
                    COALESCE(properties->>'class', properties->>'type', 'unknown') AS landcover_class,
                    ST_Area(
                        CASE
                            WHEN ST_SRID(geom) = 4326 THEN ST_Transform(geom, 3405)
                            WHEN ST_SRID(geom) = 3405 THEN geom
                            ELSE ST_Transform(ST_SetSRID(geom, 4326), 3405)
                        END
                    ) / 10000.0 AS area_ha,
                    dvc_hash AS lc_dvc_hash
                FROM spatial_assets
                WHERE aoi_id = $1
                  AND geom IS NOT NULL
            ),
            factors AS (
                SELECT
                    factor_set_id,
                    category,
                    factor_value,
                    source AS factor_source,
                    dvc_hash AS factor_dvc_hash
                FROM factor_registry
                WHERE valid_from_year <= $2
                  AND (valid_to_year IS NULL OR valid_to_year >= $2)
                  AND ($3 = '' OR source = $3)
            )
            SELECT
                l.landcover_class,
                SUM(l.area_ha) AS total_area_ha,
                f.factor_value,
                SUM(l.area_ha) * f.factor_value AS emission_tco2e,
                f.factor_source,
                f.factor_set_id,
                l.lc_dvc_hash,
                f.factor_dvc_hash
            FROM landcover l
            JOIN factors f ON l.landcover_class = f.category
            GROUP BY
                l.landcover_class,
                f.factor_value,
                f.factor_source,
                f.factor_set_id,
                f.factor_dvc_hash,
                l.lc_dvc_hash
            "#,
        )
        .bind(aoi_id)
        .bind(year_i32)
        .bind(factor_source)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoError::Database(format!("carbon calculation: {e}")))
    }

    fn build_result(
        &self,
        aoi_id: Uuid,
        year: u16,
        row: &EmissionFactorRow,
        calc_id: Uuid,
    ) -> EmissionResult {
        EmissionResult {
            calc_id,
            aoi_id,
            year,
            landcover_class: row.landcover_class.clone(),
            area_ha: row.total_area_ha,
            factor_value: row.factor_value,
            emission_tco2e: row.emission_tco2e,
            factor_source: row.factor_source.clone(),
            factor_set_id: row.factor_set_id,
            audit: AuditTrail {
                lc_dvc_hash: row.lc_dvc_hash.clone(),
                factor_dvc_hash: row.factor_dvc_hash.clone(),
                factor_set_id: row.factor_set_id.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_result_serialization() {
        let result = EmissionResult {
            calc_id: Uuid::new_v4(),
            aoi_id: Uuid::new_v4(),
            year: 2025,
            landcover_class: "forest".into(),
            area_ha: 1000.0,
            factor_value: 5.0,
            emission_tco2e: 5000.0,
            factor_source: "IPCC_2019".into(),
            factor_set_id: Uuid::new_v4(),
            audit: AuditTrail {
                lc_dvc_hash: Some("abc123".into()),
                factor_dvc_hash: Some("def456".into()),
                factor_set_id: "ghi789".into(),
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("forest"));
        assert!(json.contains("5000.0"));
        assert!(json.contains("abc123"));
    }
}
