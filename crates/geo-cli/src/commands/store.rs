//! Store subcommand handler.

use super::super::StoreAction;

/// Handle `store migrate | write | read | dvc-*`.
pub async fn handle(action: StoreAction) -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://geo:geo@localhost:5432/geo_test".to_string());

    match action {
        StoreAction::Migrate => {
            let store = geo_store::PostgisStore::connect(&db_url).await?;

            match store.check_postgis().await {
                Ok(version) => println!("PostGIS version: {version}"),
                Err(e) => eprintln!("Warning: {e}"),
            }

            geo_store::run_migrations(store.pool()).await?;
            println!("Migrations applied successfully.");

            let tables: Vec<(String,)> =
                sqlx::query_as("SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename")
                    .fetch_all(store.pool())
                    .await?;

            println!("\nTables in public schema:");
            for (name,) in &tables {
                println!("  • {name}");
            }
        }

        StoreAction::Write { table, file } => {
            println!("[store] Writing {file} → {table} ...");

            let store = geo_store::PostgisStore::connect(&db_url).await?;
            let content = tokio::fs::read_to_string(&file).await?;
            let geojson: serde_json::Value = serde_json::from_str(&content)?;

            let features = geojson["features"]
                .as_array()
                .ok_or_else(|| geo_core::GeoError::Validation("not a FeatureCollection".into()))?;

            let mut count = 0u64;
            for feat in features {
                let props = &feat["properties"];
                let coords = &feat["geometry"]["coordinates"];

                let mut props_with_coords = props.clone();
                if let (Some(lon), Some(lat)) = (coords[0].as_f64(), coords[1].as_f64()) {
                    props_with_coords["lon"] = serde_json::json!(lon);
                    props_with_coords["lat"] = serde_json::json!(lat);
                }

                sqlx::query(
                    "INSERT INTO spatial_assets (source, properties) VALUES ($1, $2)"
                )
                .bind(&file)
                .bind(&props_with_coords)
                .execute(store.pool())
                .await?;
                count += 1;
            }

            println!("  Wrote {count} rows to {table}");
        }

        StoreAction::Read { sql } => {
            let store = geo_store::PostgisStore::connect(&db_url).await?;
            match store.query_json(&sql).await {
                Ok(rows) => println!("{}", serde_json::to_string_pretty(&rows)?),
                Err(e) => eprintln!("Query failed: {e}"),
            }
        }

        StoreAction::DvcSnapshot { file } => {
            if !geo_store::dvc_available() {
                eprintln!("Error: DVC CLI not found. Install with: pip install dvc");
                return Ok(());
            }
            let snapshot = geo_store::dvc_snapshot(&file)?;
            println!("DVC snapshot: {} → {}", snapshot.file, snapshot.dvc_hash);
        }

        StoreAction::DvcPull { target } => {
            if !geo_store::dvc_available() {
                eprintln!("Error: DVC CLI not found.");
                return Ok(());
            }
            geo_store::dvc_pull(target.as_deref())?;
            println!("DVC pull complete");
        }

        StoreAction::DvcHash { file } => {
            match geo_store::dvc_hash(&file) {
                Ok(hash) => println!("{hash}"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }

    Ok(())
}
