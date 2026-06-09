//! geo-registry: 插件注册与调度中心。
//!
//! 管理所有 Plugin 和 Adapter 的注册、发现、生命周期。
//! 为 MCP Server 动态生成 tools/list。

#![allow(missing_docs)]

pub mod registry;

pub use registry::PluginRegistry;
