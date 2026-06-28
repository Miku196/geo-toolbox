#![allow(dead_code)]

use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static DEBUG_ENABLED: Mutex<bool> = Mutex::new(false);
static LOG_BUFFER: Mutex<Vec<LogEntry>> = Mutex::new(Vec::new());

#[derive(serde::Serialize)]
struct LogEntry {
    level: String,
    tag: String,
    msg: String,
}

/// Enable or disable debug mode.
/// When enabled, all WASM function calls are logged to browser console
/// and accumulated in an internal buffer (last 500 entries).
#[wasm_bindgen(js_name = setDebugEnabled)]
pub fn set_debug_enabled(enabled: bool) {
    *DEBUG_ENABLED.lock().unwrap() = enabled;
    if enabled {
        web_sys::console::log_1(&"[geo-wasm] Debug mode enabled".into());
        internal_log("info", "debug", "Debug mode enabled");
    }
}

/// Check if debug mode is currently enabled.
#[wasm_bindgen(js_name = isDebugEnabled)]
pub fn is_debug_enabled() -> bool {
    *DEBUG_ENABLED.lock().unwrap()
}

/// Get all buffered debug log entries as a JSON array.
/// Each entry: { level, tag, msg }
#[wasm_bindgen(js_name = getDebugLog)]
pub fn get_debug_log() -> String {
    let buf = LOG_BUFFER.lock().unwrap();
    serde_json::to_string(&*buf).unwrap_or_else(|e| format!("[] // serialize error: {e}"))
}

/// Clear the internal debug log buffer.
#[wasm_bindgen(js_name = clearDebugLog)]
pub fn clear_debug_log() {
    LOG_BUFFER.lock().unwrap().clear();
    internal_log("info", "debug", "Log buffer cleared");
}

/// Write a log entry (public JS-accessible version).
#[wasm_bindgen(js_name = writeLog)]
pub fn write_log(level: String, tag: String, msg: String) {
    if "error" == level {
        // Errors always log, even when debug is off
        let entry = format!("[{}] [{}] {}", level, tag, msg);
        web_sys::console::error_1(&entry.into());
    }
    internal_log(&level, &tag, &msg);
}

pub(crate) fn internal_log(level: &str, tag: &str, msg: &str) {
    let entry = LogEntry {
        level: level.to_string(),
        tag: tag.to_string(),
        msg: msg.to_string(),
    };
    let enabled = *DEBUG_ENABLED.lock().unwrap();

    // Always buffer errors; buffer others only when debug enabled
    if enabled || level == "error" {
        if let Ok(mut buf) = LOG_BUFFER.lock() {
            buf.push(entry);
            if buf.len() > 500 {
                buf.remove(0);
            }
        }
    }

    if enabled || level == "error" {
        let console_msg = format!("[geo-wasm:{}] [{}] {}", level, tag, msg);
        match level {
            "error" => web_sys::console::error_1(&console_msg.into()),
            "warn" => web_sys::console::warn_1(&console_msg.into()),
            _ => web_sys::console::log_1(&console_msg.into()),
        }
    }
}

/// Log a function call with its arguments (for internal use).
/// Returns a guard that logs the function result on drop.
#[allow(dead_code)]
pub(crate) fn log_fn_call(name: &str, args: &[&dyn std::fmt::Debug]) -> FnGuard {
    let arg_str = args
        .iter()
        .map(|a| format!("{:?}", a))
        .collect::<Vec<_>>()
        .join(", ");
    internal_log("log", name, &format!("→ ({})", arg_str));
    FnGuard {
        name: name.to_string(),
        started: true,
    }
}

pub(crate) struct FnGuard {
    name: String,
    started: bool,
}

impl FnGuard {
    pub fn ok<T: std::fmt::Debug>(self, result: &T) {
        internal_log("info", &self.name, &format!("✓ = {:?}", result));
    }

    pub fn err(self, error: &str) {
        internal_log("error", &self.name, &format!("✗ {error}"));
    }
}

impl Drop for FnGuard {
    fn drop(&mut self) {
        if self.started {
            // If neither ok() nor err() was called, note that it returned
            internal_log("warn", &self.name, "✓ (returned, result not captured)");
        }
    }
}
