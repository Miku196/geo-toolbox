//! GEE task dispatcher over NATS/file MQ — delegates to Python gee-worker, tracks callbacks.
#![allow(missing_docs)]
pub mod gee_adapter;
pub mod gee_dispatcher;
pub mod gee_mq;
pub mod gee_tools;
pub mod gee_tracker;

pub use gee_adapter::GeeAdapter;
pub use gee_tools::register_tools;
pub use gee_dispatcher::{GeeCallback, GeeDispatcher, GeeTask};
#[cfg(feature = "nats")]
pub use gee_mq::NatsMq;
pub use gee_mq::{create_mq, FileMq, GeeMq};
pub use gee_tracker::{GeeTracker, TaskStatus, TrackedTask};
