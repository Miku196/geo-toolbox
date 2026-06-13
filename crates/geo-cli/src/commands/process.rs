//! Process subcommand handler (GEE / GDAL / QGIS).

#[cfg(feature = "gdal")]
use super::super::GdalAction;
#[cfg(feature = "gee")]
use super::super::GeeAction;
#[cfg(any(feature = "gee", feature = "gdal", feature = "qgis"))]
use super::super::ProcessAction;
#[cfg(feature = "qgis")]
use super::super::QgisAction;

/// Handle `process gee | gdal | qgis`.
pub async fn handle(action: ProcessAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        #[cfg(feature = "gee")]
        ProcessAction::Gee { action } => handle_gee(action).await,
        #[cfg(feature = "gdal")]
        ProcessAction::Gdal { action } => handle_gdal(action).await,
        #[cfg(feature = "qgis")]
        ProcessAction::Qgis { action } => handle_qgis(action).await,
    }
}

// ── GEE ────────────────────────────────────────────────────
#[cfg(feature = "gee")]
async fn handle_gee(action: GeeAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        GeeAction::Classify {
            aoi,
            year,
            output_gcs,
            params,
        } => {
            use geo_adapter_gee::{dispatcher::GeeDispatcher, mq::create_mq};
            let mq = create_mq().await?;
            let dispatcher = GeeDispatcher::new(mq);
            let extra = params.as_deref().and_then(|p| serde_json::from_str(p).ok());
            let cid = dispatcher
                .dispatch_classification(&aoi, year, &output_gcs, extra)
                .await?;
            println!("Task dispatched: {cid}");
            println!("Check status: geo-toolbox process gee status --cid {cid}");
        }
        GeeAction::Ndvi {
            aoi,
            year,
            output_gcs,
        } => {
            use geo_adapter_gee::{dispatcher::GeeDispatcher, mq::create_mq};
            let mq = create_mq().await?;
            let dispatcher = GeeDispatcher::new(mq);
            let cid = dispatcher
                .dispatch_ndvi_timeseries(&aoi, year, &output_gcs)
                .await?;
            println!("NDVI task dispatched: {cid}");
        }
        GeeAction::Change {
            aoi,
            from,
            to,
            output_gcs,
        } => {
            use geo_adapter_gee::{dispatcher::GeeDispatcher, mq::create_mq};
            let mq = create_mq().await?;
            let dispatcher = GeeDispatcher::new(mq);
            let cid = dispatcher
                .dispatch_change_detection(&aoi, from, to, &output_gcs)
                .await?;
            println!("Change detection task dispatched: {cid}");
        }
        GeeAction::Status { cid } => {
            use geo_adapter_gee::tracker::GeeTracker;
            let queue_dir =
                std::env::var("GEO_QUEUE_DIR").unwrap_or_else(|_| "./queue".to_string());
            let tracker = GeeTracker::new_file(&queue_dir);
            match tracker.check_task(&cid).await? {
                Some(task) => println!("{}", serde_json::to_string_pretty(&task)?),
                None => println!("Task {cid} not found in callbacks."),
            }
        }
        GeeAction::Summary => {
            use geo_adapter_gee::tracker::GeeTracker;
            let queue_dir =
                std::env::var("GEO_QUEUE_DIR").unwrap_or_else(|_| "./queue".to_string());
            let tracker = GeeTracker::new_file(&queue_dir);
            let summary = tracker.summary().await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
    }
    Ok(())
}

// ── GDAL ───────────────────────────────────────────────────
#[cfg(feature = "gdal")]
async fn handle_gdal(action: GdalAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        GdalAction::Cog {
            input,
            output,
            compression,
        } => {
            use geo_adapter_cli::raster::{CogOptions, RasterOps};
            let opts = CogOptions {
                compression,
                ..CogOptions::default()
            };
            let result = RasterOps::to_cog(&input, &output, Some(opts)).await?;
            println!("COG created: {}", result.display());
        }
        GdalAction::Reproject {
            input,
            output,
            epsg,
        } => {
            use geo_adapter_cli::raster::RasterOps;
            let result = RasterOps::reproject(&input, &output, epsg, None).await?;
            println!("Reprojected: {}", result.display());
        }
        GdalAction::Ogr2Ogr {
            input,
            output,
            epsg,
            r#where,
            overwrite,
        } => {
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
        GdalAction::GcsBridge {
            gcs_uri,
            prefix,
            cog,
            local,
        } => {
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
    }
    Ok(())
}

// ── QGIS ───────────────────────────────────────────────────
#[cfg(feature = "qgis")]
async fn handle_qgis(action: QgisAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        QgisAction::Submit {
            algorithm,
            params,
            input,
            output,
            server,
        } => {
            use geo_adapter_qgis::grpc_client::{QgisClient, QgisInput, QgisJob, QgisToolStep};
            let client = QgisClient::new(&server);
            let params_value: serde_json::Value = serde_json::from_str(&params)?;
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
        QgisAction::Batch {
            algorithm,
            input,
            output,
            extra,
        } => {
            use geo_adapter_qgis::process_runner::{BatchQgisRunner, QgisProcessConfig, QgisTool};
            let runner = BatchQgisRunner::new(QgisProcessConfig::default());
            let extra_pairs: Vec<[String; 2]> = serde_json::from_str(&extra)?;
            let mut params = vec![("INPUT".into(), input.clone())];
            for pair in &extra_pairs {
                if pair.len() == 2 {
                    params.push((pair[0].clone(), pair[1].clone()));
                }
            }
            params.push(("OUTPUT".into(), output.clone()));
            let result = runner
                .run_tool(&QgisTool {
                    algorithm: algorithm.clone(),
                    params,
                })
                .await?;
            println!("Batch complete: {}", result.display());
        }
        QgisAction::Health { server } => {
            use geo_adapter_qgis::grpc_client::QgisClient;
            let client = QgisClient::new(&server);
            let healthy = client.health_check().await?;
            println!(
                "PyQGIS service ({server}): {}",
                if healthy {
                    "✓ healthy"
                } else {
                    "✗ unreachable"
                }
            );
        }
    }
    Ok(())
}
