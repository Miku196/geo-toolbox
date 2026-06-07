#![warn(missing_docs)]

//! geo-gee: GEE (Google Earth Engine) task dispatcher.
//!
//! Rust 端 **不直接调用 GEE API**。只负责通过消息队列将任务分发给
//! Python `gee-worker`，并通过回调 topic 追踪结果。
//!
//! ## 架构
//!
//! ```text
//! geo-toolbox process gee dispatch
//!   → MQ publish to "gee.tasks" (NATS or file queue)
//!   → Python gee-worker subscribes & executes
//!   → Worker publishes result to "gee.callbacks"
//!   → geo-toolbox tracker reads callback
//! ```
//!
//! ## 消息队列
//!
//! - **NATS** (默认): 设置环境变量 `GEO_NATS_URL=nats://localhost:4222`
//! - **文件队列** (回退): 写入 `./queue/gee-tasks.jsonl`
//! - **Kafka** (可选): feature flag `kafka`
//!
//! ## Task types
//!
//! - `landcover_classification` — 随机森林土地覆盖分类
//! - `ndvi_timeseries` — NDVI 时间序列合成
//! - `change_detection` — 双年变化检测

pub mod dispatcher;
pub mod mq;
pub mod tracker;

pub use dispatcher::{GeeCallback, GeeDispatcher, GeeTask};
pub use mq::{create_mq, FileMq, GeeMq};
#[cfg(feature = "nats")]
pub use mq::NatsMq;
pub use tracker::{GeeTracker, TaskStatus, TrackedTask};
