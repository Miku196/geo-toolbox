//! Declarative macros for plugin + tool registration.
//!
//! Reduces boilerplate in each crate's `tools.rs`.

/// Register a plugin and its sync tools in one declarative block.
///
/// # Example
///
/// ```ignore
/// pub fn register_tools(reg: &mut PluginRegistry) {
///     register_plugin!(reg, "vector", "Pure-Rust vector ops", PluginCategory::Process, [
///         sync "vector_buffer" => "Create buffer" ; json!({...}) => |args| { ... },
///     ]);
/// }
/// ```
#[macro_export]
macro_rules! register_plugin {
    // ── Sync-only variant ──
    (
        $reg:ident,
        $name:literal,
        $desc:literal,
        PluginCategory::$cat:ident,
        [
            $(
                sync $tool_name:literal => $tool_desc:literal ; $schema:expr => $handler:expr
            ),+ $(,)?
        ]
    ) => {
        $reg.register(geo_core::plugin::PluginMeta {
            name: $name.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: $desc.into(),
            category: geo_core::plugin::PluginCategory::$cat,
            healthy: true,
            extra: serde_json::json!({}),
        });
        $(
            $reg.register_tool_sync(
                $name,
                geo_registry::registry::ToolDef {
                    name: $tool_name.into(),
                    description: $tool_desc.into(),
                    input_schema: $schema,
                },
                $handler,
            );
        )+
    };

    // ── Async-only variant ──
    (
        $reg:ident,
        $name:literal,
        $desc:literal,
        PluginCategory::$cat:ident,
        [
            $(
                async $tool_name:literal => $tool_desc:literal ; $schema:expr => $handler:expr
            ),+ $(,)?
        ]
    ) => {
        $reg.register(geo_core::plugin::PluginMeta {
            name: $name.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: $desc.into(),
            category: geo_core::plugin::PluginCategory::$cat,
            healthy: true,
            extra: serde_json::json!({}),
        });
        $(
            $reg.register_tool_async(
                $name,
                geo_registry::registry::ToolDef {
                    name: $tool_name.into(),
                    description: $tool_desc.into(),
                    input_schema: $schema,
                },
                $handler,
            );
        )+
    };
}

/// Register sync tools for an already-registered plugin (no PluginMeta).
#[macro_export]
macro_rules! register_sync_tools {
    (
        $reg:ident,
        $name:literal,
        [
            $(
                $tool_name:literal => $tool_desc:literal ; $schema:expr => $handler:expr
            ),+ $(,)?
        ]
    ) => {
        $(
            $reg.register_tool_sync(
                $name,
                geo_registry::registry::ToolDef {
                    name: $tool_name.into(),
                    description: $tool_desc.into(),
                    input_schema: $schema,
                },
                $handler,
            );
        )+
    };
}

/// Register async tools for an already-registered plugin (no PluginMeta).
#[macro_export]
macro_rules! register_async_tools {
    (
        $reg:ident,
        $name:literal,
        [
            $(
                $tool_name:literal => $tool_desc:literal ; $schema:expr => $handler:expr
            ),+ $(,)?
        ]
    ) => {
        $(
            $reg.register_tool_async(
                $name,
                geo_registry::registry::ToolDef {
                    name: $tool_name.into(),
                    description: $tool_desc.into(),
                    input_schema: $schema,
                },
                $handler,
            );
        )+
    };
}
