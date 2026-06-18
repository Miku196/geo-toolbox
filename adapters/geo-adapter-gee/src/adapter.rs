use crate::dispatcher::GeeDispatcher;
use crate::mq::FileMq;
use crate::tracker::GeeTracker;
use geo_core::errors::GeoResult;
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
        Ok(true)
    }
    async fn external_version(&self) -> GeoResult<String> {
        Ok("GEE Python worker".into())
    }
    fn requires_network(&self) -> bool {
        true
    }
    async fn push(&self, _table: &str, _data: &[GeoFeature]) -> GeoResult<u64> {
        Ok(0)
    }
    async fn pull(&self, _query: &str) -> GeoResult<Vec<GeoFeature>> {
        Ok(vec![])
    }
    async fn execute(
        &self,
        _command: &str,
        _params: serde_json::Value,
    ) -> GeoResult<serde_json::Value> {
        Ok(serde_json::json!({"status":"ok"}))
    }
}

#[test]
fn test_gee_adapter() {
    let a = GeeAdapter::new("nats://localhost:4222");
    assert_eq!(a.name(), "gee");
    assert_eq!(a.category(), PluginCategory::Adapter);
}
