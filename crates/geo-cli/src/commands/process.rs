//! Process subcommand handler (GEE / GDAL / QGIS).

use super::super::{GdalAction, GeeAction, ProcessAction, QgisAction};

/// Handle `process gee | gdal | qgis`.
pub async fn handle(action: ProcessAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ProcessAction::Gee { action } => handle_gee(action).await,
        ProcessAction::Gdal { action } => handle_gdal(action).await,
        ProcessAction::Qgis { action } => handle_qgis(action).await,
    }
}

// ── GEE ────────────────────────────────────────────────────

#[allow(unused_variables)]
async fn handle_gee(action: GeeAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        GeeAction::Classify { aoi, year, output_gcs, params } => {
            #[cfg(feature = "gee")]
            {
                use geo_adapter_gee::{dispatcher::GeeDispatcher, mq::create_mq};
                let mq = create_mq().await?;
                let dispatcher = GeeDispatcher::new(mq);
                let extra = params
                    .as_deref()
                    .and_then(|p| serde_json::from_str(p).ok());
                let cid = dispatcher
                    .dispatch_classification(&aoi, year, &output_gcs, extra)
                    .await?;
                println!("Task dispatched: {cid}");
                println!("Check status: geo-toolbox process gee status --cid {cid}");
            }
            #[cfg(not(feature = "gee"))]
            println!(
                "[gee] Landcover classification — feature 'gee' not enabled.\n\
                 Build with: cargo build --features gee"
            );
        }
        GeeAction::Ndvi { aoi, year, output_gcs } => {
            #[cfg(feature = "gee")]
            {
                use geo_adapter_gee::{dispatcher::GeeDispatcher, mq::create_mq};
                let mq = create_mq().await?;
                let dispatcher = GeeDispatcher::new(mq);
                let cid = dispatcher
                    .dispatch_ndvi_timeseries(&aoi, year, &output_gcs)
                    .await?;
                println!("NDVI task dispatched: {cid}");
            }
            #[cfg(not(feature = "gee"))]
            println!("[gee] NDVI — feature 'gee' not enabled.");
        }
        GeeAction::Change { aoi, from, to, output_gcs } => {
            #[cfg(feature = "gee")]
            {
                use geo_adapter_gee::{dispatcher::GeeDispatcher, mq::create_mq};
                let mq = create_mq().await?;
                let dispatcher = GeeDispatcher::new(mq);
                let cid = dispatcher
                    .dispatch_change_detection(&aoi, from, to, &output_gcs)
                    .await?;
                println!("Change detection task dispatched: {cid}");
            }
            #[cfg(not(feature = "gee"))]
            println!("[gee] Change detection — feature 'gee' not enabled.");
        }
        GeeAction::Status { cid } => {
            #[cfg(feature = "gee")]
            {
                use geo_adapter_gee::tracker::GeeTracker;
                let queue_dir = std::env::var("GEO_QUEUE_DIR")
                    .unwrap_or_else(|_| "./queue".to_string());
                let tracker = GeeTracker::new_file(&queue_dir);
                match tracker.check_task(&cid).await? {
                    Some(task) => println!("{}", serde_json::to_string_pretty(&task)?),
                    None => println!("Task {cid} not found in callbacks.\n\
                        Check: tail queue/gee-callbacks.jsonl"),
                }
            }
            #[cfg(not(feature = "gee"))]
            println!("[gee] Status — feature 'gee' not enabled.");
        }
        GeeAction::Summary => {
            #[cfg(feature = "gee")]
            {
                use geo_adapter_gee::tracker::GeeTracker;
                let queue_dir = std::env::var("GEO_QUEUE_DIR")
                    .unwrap_or_else(|_| "./queue".to_string());
                let tracker = GeeTracker::new_file(&queue_dir);
                let summary = tracker.summary().await?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            #[cfg(not(feature = "gee"))]
            println!("[gee] Summary — feature 'gee' not enabled.");
        }
    }
    Ok(())
}

// ── GDAL ───────────────────────────────────────────────────

#[allow(unused_variables)]
async fn handle_gdal(action: GdalAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        GdalAction::Cog { input, output, compression } => {
            #[cfg(feature = "gdal")]
            {
                use geo_adapter_cli::raster::{CogOptions, RasterOps};
                let opts = CogOptions {
                    compression,
                    ..CogOptions::default()
                };
                let result = RasterOps::to_cog(&input, &output, Some(opts)).await?;
                println!("COG created: {}", result.display());
            }
            #[cfg(not(feature = "gdal"))]
            println!(
                "[gdal] COG conversion — feature 'gdal' not enabled.\n\
                 Build with: cargo build --features gdal"
            );
        }
        GdalAction::Reproject { input, output, epsg } => {
            #[cfg(feature = "gdal")]
            {
                use geo_adapter_cli::raster::RasterOps;
                let result = RasterOps::reproject(&input, &output, epsg, None).await?;
                println!("Reprojected: {}", result.display());
            }
            #[cfg(not(feature = "gdal"))]
            println!("[gdal] Reproject — feature 'gdal' not enabled.");
        }
        GdalAction::Ogr2Ogr { input, output, epsg, r#where, overwrite } => {
            #[cfg(feature = "gdal")]
            {
                use geo_adapter_cli::vector::{Ogr2OgrOptions, VectorOps};
                let opts = Ogr2OgrOptions {
                    target_epsg: epsg,
                    where_clause: r#where,
                    overwrite,
                    ..Default::default()
                };
                let result = VectorOps::convert(&input, &output, Some(opts)).await?;
                println!("Converted: {}", result.display());
            }
            #[cfg(not(feature = "gdal"))]
            println!("[gdal] ogr2ogr — feature 'gdal' not enabled.");
        }
        GdalAction::GcsBridge { gcs_uri, prefix, cog, local } => {
            #[cfg(feature = "gdal")]
            {
                use geo_adapter_cli::gcs_bridge::{GcsBridge, GcsBridgeConfig};
                let mut config = GcsBridgeConfig::default();
                if local {
                    config.minio_endpoint = None;
                    config.minio_bucket = None;
                }
                let bridge = GcsBridge::new(config);
                let result = bridge.sync(&gcs_uri, &prefix, cog).await?;
                println!("Synced: {result}");
            }
            #[cfg(not(feature = "gdal"))]
            println!("[gdal] GCS Bridge — feature 'gdal' not enabled.");
        }
    }
    Ok(())
}

// ── QGIS ───────────────────────────────────────────────────

#[allow(unused_variables)]
async fn handle_qgis(action: QgisAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        QgisAction::Submit { algorithm, params, input, output, server } => {
            #[cfg(feature = "qgis")]
            {
                use geo_adapter_qgis::grpc_client::{QgisClient, QgisInput, QgisJob, QgisToolStep};
                let client = QgisClient::new(&server);
                let params_value: serde_json::Value =
                    serde_json::from_str(&params)?;
                let job = QgisJob {
                    model: None,
                    tools: vec![QgisToolStep {
                        algorithm: algorithm.clone(),
                        params: params_value,
                        label: Some(algorithm.clone()),
                    }],
                    inputs: vec![QgisInput {
                        name: "input_layer".into(),
                        path: input.clone(),
                        crs: None,
                    }],
                    output_format: "gpkg".into(),
                };
                let job_id = client.submit(&job).await?;
                println!("Job submitted: {job_id}");
                let result = client
                    .wait_for_job(&job_id, std::time::Duration::from_secs(2))
                    .await?;
                println!("Result: {}", result.output_path.unwrap_or(output));
            }
            #[cfg(not(feature = "qgis"))]
            println!(
                "[qgis] Submit — feature 'qgis' not enabled.\n\
                 Build with: cargo build --features qgis"
            );
        }
        QgisAction::Batch { algorithm, input, output, extra } => {
            #[cfg(feature = "qgis")]
            {
                use geo_adapter_qgis::process_runner::{BatchQgisRunner, QgisProcessConfig, QgisTool};
                let runner = BatchQgisRunner::new(QgisProcessConfig::default());
                let extra_pairs: Vec<[String; 2]> = serde_json::from_str(&extra)?;
                let mut params = vec![
                    ("INPUT".into(), input.clone()),
                ];
                for pair in &extra_pairs {
                    if pair.len() == 2 {
                        params.push((pair[0].clone(), pair[1].clone()));
                    }
                }
                params.push(("OUTPUT".into(), output.clone()));
                let result = runner
                    .run_tool(&QgisTool { algorithm: algorithm.clone(), params })
                    .await?;
                println!("Batch complete: {}", result.display());
            }
            #[cfg(not(feature = "qgis"))]
            println!("[qgis] Batch — feature 'qgis' not enabled.");
        }
        QgisAction::Health { server } => {
            #[cfg(feature = "qgis")]
            {
                use geo_adapter_qgis::grpc_client::QgisClient;
                let client = QgisClient::new(&server);
                let healthy = client.health_check().await?;
                println!("PyQGIS service ({server}): {}", if healthy { "✓ healthy" } else { "✗ unreachable" });
            }
            #[cfg(not(feature = "qgis"))]
            println!("[qgis] Health — feature 'qgis' not enabled.");
        }
    }
    Ok(())
}
