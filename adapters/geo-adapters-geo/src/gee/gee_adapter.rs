use super::gee_dispatcher::GeeDispatcher;
use super::gee_mq::FileMq;
use super::gee_tracker::GeeTracker;
use geo_core::errors::{GeoError, GeoResult};
use geo_core::plugin::{ExternalAdapter, GeoFeature, Plugin, PluginCategory};

pub struct GeeAdapter {
    endpoint: String,
    dispatcher: GeeDispatcher,
    tracker: GeeTracker,
}

impl GeeAdapter {
    pub fn new(endpoint: &str) -> Self {
        let mq = Box::new(FileMq::new("queue/gee-tasks.jsonl"));
        Self {
            endpoint: endpoint.to_string(),
            dispatcher: GeeDispatcher::new(mq),
            tracker: GeeTracker::new_file("queue/gee-callbacks.jsonl"),
        }
    }

    pub async fn new_default() -> GeoResult<Self> {
        Ok(Self::new("file://queue"))
    }

    /// Submit a landcover classification task.
    pub async fn submit_classification(
        &self,
        aoi: &str,
        year: u16,
        collection: &str,
        output_gcs: &str,
    ) -> GeoResult<String> {
        self.dispatcher
            .dispatch_classification(
                aoi,
                year,
                output_gcs,
                Some(serde_json::json!({
                    "collection": collection
                })),
            )
            .await
    }

    /// Check the status of a submitted task.
    pub async fn job_status(&self, cid: &str) -> GeoResult<String> {
        self.tracker.check_task(cid).await.map(|opt| {
            opt.map(|t| format!("{:?}", t.status))
                .unwrap_or_else(|| "not_found".into())
        })
    }
}
impl Plugin for GeeAdapter {
    type Config = geo_core::plugin::EmptyConfig;
    fn new(_config: Self::Config) -> Self {
        Self::new("file://queue")
    }
    fn name(&self) -> &str {
        "gee"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "GEE task dispatcher via message queue"
    }
    fn category(&self) -> PluginCategory {
        PluginCategory::Adapter
    }
}
impl ExternalAdapter for GeeAdapter {
    fn external_endpoint(&self) -> &str {
        &self.endpoint
    }
    async fn health_check(&self) -> GeoResult<bool> {
        Err(GeoError::Unimplemented(
            "GeeAdapter: health_check not implemented — no gee-worker liveness probe wired".into(),
        ))
    }
    async fn external_version(&self) -> GeoResult<String> {
        Err(GeoError::Unimplemented(
            "GeeAdapter: external_version not queried — worker version query not wired".into(),
        ))
    }
    fn requires_network(&self) -> bool {
        true
    }
    async fn push(&self, _table: &str, _data: &[GeoFeature]) -> GeoResult<u64> {
        Err(GeoError::Unimplemented(
            "GeeAdapter: push not supported — use submit_classification/execute to dispatch a task"
                .into(),
        ))
    }
    async fn pull(&self, _query: &str) -> GeoResult<Vec<GeoFeature>> {
        Err(GeoError::Unimplemented(
            "GeeAdapter: pull not supported — use job_status/callbacks to read task results".into(),
        ))
    }
    async fn execute(
        &self,
        command: &str,
        params: serde_json::Value,
    ) -> GeoResult<serde_json::Value> {
        let aoi = params["aoi"]
            .as_str()
            .or_else(|| params["aoi_path"].as_str())
            .map(str::to_owned)
            .ok_or_else(|| GeoError::invalid_input("aoi", "GEE execute requires an AOI path"))?;
        let year = params["year"]
            .as_u64()
            .ok_or_else(|| GeoError::invalid_input("year", "GEE execute requires a year"))?
            as u16;
        let output_gcs = params["output_gcs"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| {
                GeoError::invalid_input("output_gcs", "GEE execute requires an output GCS URI")
            })?;
        let cid = self
            .dispatcher
            .dispatch_custom(command, &aoi, year, &output_gcs, params)
            .await?;
        Ok(serde_json::json!({
            "correlation_id": cid,
            "task_type": command,
        }))
    }
}

#[test]
fn test_gee_adapter() {
    let a = GeeAdapter::new("nats://localhost:4222");
    assert_eq!(a.name(), "gee");
    assert_eq!(a.category(), PluginCategory::Adapter);
}

#[tokio::test]
async fn test_gee_execute_dispatches_real_task() {
    let adapter = GeeAdapter::new("file://queue");
    let result = adapter
        .execute(
            "landcover_extra",
            serde_json::json!({
                "aoi": "s3://geo-data/vector/sites.gpkg",
                "year": 2025,
                "output_gcs": "gs://gee-exports/lc_2025.tif",
            }),
        )
        .await
        .expect("execute should dispatch a real GEE task");

    let cid = result["correlation_id"]
        .as_str()
        .expect("execute result should carry a correlation_id");
    assert!(!cid.is_empty(), "correlation_id must not be empty");

    // FileMq::new("queue/gee-tasks.jsonl") nests the JSONL file under a subdirectory,
    // so the task file lives at queue/gee-tasks.jsonl/gee-tasks.jsonl.
    let qdir = std::env::current_dir().unwrap().join("queue");
    let task_path = qdir.join("gee-tasks.jsonl").join("gee-tasks.jsonl");
    let content = tokio::fs::read_to_string(&task_path)
        .await
        .expect("queue task file should exist after dispatch");
    assert!(
        content.contains(cid),
        "queue file must contain dispatched task id"
    );
    assert!(content.contains("landcover_extra"));

    let _ = std::fs::remove_dir_all(&qdir);
}

#[tokio::test]
async fn test_gee_execute_requires_aoi_and_output_gcs() {
    let adapter = GeeAdapter::new("file://queue");
    let err = adapter
        .execute("landcover", serde_json::json!({}))
        .await
        .expect_err("missing aoi/output_gcs must error");
    assert!(matches!(err, GeoError::InvalidInput { .. }), "got {err:?}");
}

#[tokio::test]
async fn test_gee_push_pull_not_supported() {
    let adapter = GeeAdapter::new("file://queue");
    assert!(matches!(
        adapter.push("t", &[]).await,
        Err(GeoError::Unimplemented(_))
    ));
    assert!(matches!(
        adapter.pull("q").await,
        Err(GeoError::Unimplemented(_))
    ));
}

#[tokio::test]
async fn test_gee_health_check_not_faked() {
    let adapter = GeeAdapter::new("file://queue");
    assert!(matches!(
        adapter.health_check().await,
        Err(GeoError::Unimplemented(_))
    ));
}
