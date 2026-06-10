//! PostGIS connection pool, migration runner, and spatial query helpers.

use geo_core::errors::{GeoError, GeoResult};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use sqlx::Column;
use uuid::Uuid;

/// Wraps a `PgPool` with geo-toolbox–specific convenience methods.
#[derive(Clone)]
pub struct PostgisStore {
    pool: PgPool,
}

impl PostgisStore {
    /// Connect to PostgreSQL with sensible defaults for geo workloads.
    ///
    /// ```ignore
    /// let store = PostgisStore::connect("postgres://geo:geo@localhost/geo_test").await?;
    /// ```
    pub async fn connect(url: &str) -> GeoResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(2)
            .connect(url)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Expose the inner pool for use by BatchWriter / custom queries.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check that the PostGIS extension is available.
    pub async fn check_postgis(&self) -> GeoResult<String> {
        let row = sqlx::query("SELECT PostGIS_Version()")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GeoError::Database(format!("PostGIS not available: {e}")))?;

        Ok(row.get::<String, _>(0))
    }

    /// Execute a SQL query and return rows as JSON.
    ///
    /// Security: rejects destructive SQL keywords to prevent injection.
    /// For production, connect with a read-only database user.
    pub async fn query_json(&self, sql: &str) -> GeoResult<Vec<serde_json::Value>> {
        geo_core::errors::validate_select_sql(sql)?;
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;

        let result: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                let mut map = serde_json::Map::new();
                for (i, col) in row.columns().iter().enumerate() {
                    // Try multiple types, falling back to debug string
                    let val = if let Ok(v) = row.try_get::<i64, _>(i) {
                        serde_json::Value::Number(v.into())
                    } else if let Ok(v) = row.try_get::<f64, _>(i) {
                        serde_json::json!(v)
                    } else if let Ok(v) = row.try_get::<String, _>(i) {
                        serde_json::Value::String(v)
                    } else {
                        serde_json::Value::String("<unknown type>".into())
                    };
                    map.insert(col.name().to_string(), val);
                }
                serde_json::Value::Object(map)
            })
            .collect();
        Ok(result)
    }

    /// Simple health check.
    pub async fn ping(&self) -> GeoResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| GeoError::Database(e.to_string()))?;
        Ok(())
    }

    /// Insert a single geometry row (for testing; use BatchWriter for production).
    pub async fn insert_geometry(
        &self,
        aoi_id: Option<Uuid>,
        source: &str,
        wkb: &[u8],
        properties: &serde_json::Value,
    ) -> GeoResult<Uuid> {
        let row = sqlx::query(
            "INSERT INTO spatial_assets (aoi_id, source, geom, properties)
             VALUES ($1, $2, ST_GeomFromWKB($3, 4326), $4)
             RETURNING id",
        )
        .bind(aoi_id)
        .bind(source)
        .bind(wkb)
        .bind(properties)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?;

        Ok(row.get(0))
    }
}

/// Run embedded SQL migrations.
///
/// Skips PostGIS-dependent statements when PostGIS is not available.
pub async fn run_migrations(pool: &PgPool) -> GeoResult<()> {
    // Check if PostGIS is available
    let has_postgis = sqlx::query("SELECT 1 FROM pg_extension WHERE extname = 'postgis'")
        .fetch_optional(pool)
        .await
        .map_err(|e| GeoError::Database(e.to_string()))?
        .is_some();

    if has_postgis {
        let migrations: &[(&str, &str)] = &[
            ("001_factor_registry", include_str!("../migrations/001_factor_registry.sql")),
            ("002_carbon_accounting_results", include_str!("../migrations/002_carbon_accounting_results.sql")),
            ("003_spatial_assets", include_str!("../migrations/003_spatial_assets.sql")),
        ];
        for (name, sql) in migrations {
            sqlx::raw_sql(sql).execute(pool).await.map_err(|e| {
                GeoError::Database(format!("migration {name} failed: {e}"))
            })?;
            tracing::info!("Migration {name} applied");
        }
        tracing::info!("All migrations complete (3 tables with PostGIS)");
    } else {
        tracing::warn!("PostGIS not available — basic tables only");
        for sql in [
            "CREATE TABLE IF NOT EXISTS spatial_assets (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), aoi_id UUID, source TEXT NOT NULL, geom TEXT, properties JSONB DEFAULT '{}', file_path TEXT, crs TEXT DEFAULT 'EPSG:4326', ingested_at TIMESTAMPTZ DEFAULT now(), ingested_by TEXT DEFAULT 'geo-toolbox')",
            "CREATE TABLE IF NOT EXISTS factor_registry (factor_set_id UUID PRIMARY KEY DEFAULT gen_random_uuid(), source TEXT NOT NULL, category TEXT NOT NULL, factor_value DOUBLE PRECISION NOT NULL, unit TEXT DEFAULT 'tCO2e/ha/yr', valid_from_year INT NOT NULL, valid_to_year INT, region TEXT, dvc_hash TEXT)",
            "CREATE TABLE IF NOT EXISTS carbon_accounting_results (calc_id UUID PRIMARY KEY DEFAULT gen_random_uuid(), workflow_run_id UUID NOT NULL, calculation_at TIMESTAMPTZ DEFAULT now(), aoi_id UUID NOT NULL, area_ha DOUBLE PRECISION, landcover_src TEXT NOT NULL, landcover_class TEXT NOT NULL, lc_dvc_hash TEXT, factor_set_id UUID, emission_tco2e DOUBLE PRECISION NOT NULL, audit_status TEXT DEFAULT 'pending', created_at TIMESTAMPTZ DEFAULT now())",
        ] {
            sqlx::raw_sql(sql).execute(pool).await.map_err(|e| GeoError::Database(format!("migration failed: {e}")))?;
        }
        tracing::info!("Basic tables created");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_migration_sql_embedded() {
        let sql = include_str!("../migrations/001_factor_registry.sql");
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("factor_registry"));

        let sql = include_str!("../migrations/002_carbon_accounting_results.sql");
        assert!(sql.contains("carbon_accounting_results"));

        let sql = include_str!("../migrations/003_spatial_assets.sql");
        assert!(sql.contains("spatial_assets"));
    }
}
