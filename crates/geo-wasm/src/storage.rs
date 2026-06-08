//! Browser-side spatial data storage using IndexedDB.
//!
//! Provides persistent local storage for GeoJSON features,
//! emission factors, and carbon calculation results.
//! All data stays on the user's machine.

use wasm_bindgen::prelude::*;

/// Browser-side spatial data store backed by IndexedDB.
///
/// Uses `rexie` (Rust wrapper around IndexedDB) for persistent,
/// structured storage in the browser.
///
/// ## Collections
///
/// - `features` — GeoJSON features (AOI polygons, landcover parcels)
/// - `factors` — Emission factor lookup tables
/// - `results` — Carbon calculation history
/// - `crs_defs` — Custom CRS definitions
#[wasm_bindgen]
pub struct GeoStore {
    db_name: String,
}

#[wasm_bindgen]
impl GeoStore {
    /// Open (or create) the IndexedDB database.
    #[wasm_bindgen(constructor)]
    pub fn new(db_name: &str) -> Self {
        Self { db_name: db_name.to_string() }
    }

    /// Initialize the database and create object stores if needed.
    #[wasm_bindgen(js_name = init)]
    pub async fn init(&self) -> Result<(), JsValue> {
        use rexie::{Rexie, ObjectStore};

        let _rexie = Rexie::builder(&self.db_name)
            .version(1)
            .add_object_store(ObjectStore::new("features")
                .key_path("id")
                .auto_increment(false))
            .add_object_store(ObjectStore::new("factors")
                .key_path("id")
                .auto_increment(false))
            .add_object_store(ObjectStore::new("results")
                .key_path("id")
                .auto_increment(false))
            .add_object_store(ObjectStore::new("crs_defs")
                .key_path("epsg"))
            .build()
            .await
            .map_err(|e| JsValue::from_str(&format!("IndexedDB init: {e}")))?;

        Ok(())
    }

    /// Store a GeoJSON feature.
    #[wasm_bindgen(js_name = putFeature)]
    pub async fn put_feature(&self, id: &str, geojson_feature: &str) -> Result<(), JsValue> {
        let value: serde_json::Value = serde_json::from_str(geojson_feature)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {e}")))?;

        let rexie = rexie::Rexie::builder(&self.db_name).build().await
            .map_err(|e| JsValue::from_str(&format!("DB open: {e}")))?;

        let tx = rexie.transaction(&["features"], rexie::TransactionMode::ReadWrite)
            .map_err(|e| JsValue::from_str(&format!("Transaction: {e}")))?;

        let store = tx.store("features")
            .map_err(|e| JsValue::from_str(&format!("Store: {e}")))?;

        let js_val = serde_wasm_bindgen::to_value(&value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        store.put(&js_val, Some(&JsValue::from_str(id))).await
            .map_err(|e| JsValue::from_str(&format!("Put: {e}")))?;

        tx.done().await
            .map_err(|e| JsValue::from_str(&format!("Commit: {e}")))?;

        Ok(())
    }

    /// Retrieve a stored GeoJSON feature by ID.
    #[wasm_bindgen(js_name = getFeature)]
    pub async fn get_feature(&self, id: &str) -> Result<JsValue, JsValue> {
        let rexie = rexie::Rexie::builder(&self.db_name).build().await
            .map_err(|e| JsValue::from_str(&format!("DB open: {e}")))?;

        let tx = rexie.transaction(&["features"], rexie::TransactionMode::ReadOnly)
            .map_err(|e| JsValue::from_str(&format!("Transaction: {e}")))?;

        let store = tx.store("features")
            .map_err(|e| JsValue::from_str(&format!("Store: {e}")))?;

        // rexie::store::Store::get takes JsValue by value
        let result = store.get(JsValue::from_str(id)).await
            .map_err(|e| JsValue::from_str(&format!("Get: {e}")))?;

        tx.done().await.ok();

        Ok(result.unwrap_or(JsValue::NULL))
    }

    /// Get all stored features as a GeoJSON FeatureCollection JSON string.
    #[wasm_bindgen(js_name = getAllFeatures)]
    pub async fn get_all_features(&self) -> Result<String, JsValue> {
        let rexie = rexie::Rexie::builder(&self.db_name).build().await
            .map_err(|e| JsValue::from_str(&format!("DB open: {e}")))?;

        let tx = rexie.transaction(&["features"], rexie::TransactionMode::ReadOnly)
            .map_err(|e| JsValue::from_str(&format!("Transaction: {e}")))?;

        let store = tx.store("features")
            .map_err(|e| JsValue::from_str(&format!("Store: {e}")))?;

        let all = store.get_all(None, None).await
            .map_err(|e| JsValue::from_str(&format!("GetAll: {e}")))?;

        tx.done().await.ok();

        // Convert JsValue array → Rust Vec → GeoJSON
        let features: Vec<serde_json::Value> = serde_wasm_bindgen::from_value(
            JsValue::from(all)
        ).map_err(|e| JsValue::from_str(&e.to_string()))?;

        let fc = serde_json::json!({
            "type": "FeatureCollection",
            "features": features,
        });

        serde_json::to_string(&fc)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Delete a feature by ID.
    #[wasm_bindgen(js_name = deleteFeature)]
    pub async fn delete_feature(&self, id: &str) -> Result<(), JsValue> {
        let rexie = rexie::Rexie::builder(&self.db_name).build().await
            .map_err(|e| JsValue::from_str(&format!("DB open: {e}")))?;

        let tx = rexie.transaction(&["features"], rexie::TransactionMode::ReadWrite)
            .map_err(|e| JsValue::from_str(&format!("Transaction: {e}")))?;

        let store = tx.store("features")
            .map_err(|e| JsValue::from_str(&format!("Store: {e}")))?;

        store.delete(JsValue::from_str(id)).await
            .map_err(|e| JsValue::from_str(&format!("Delete: {e}")))?;

        tx.done().await
            .map_err(|e| JsValue::from_str(&format!("Commit: {e}")))?;

        Ok(())
    }

    /// Clear all stored features.
    #[wasm_bindgen(js_name = clearAll)]
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let rexie = rexie::Rexie::builder(&self.db_name).build().await
            .map_err(|e| JsValue::from_str(&format!("DB open: {e}")))?;

        let tx = rexie.transaction(&["features"], rexie::TransactionMode::ReadWrite)
            .map_err(|e| JsValue::from_str(&format!("Transaction: {e}")))?;

        let store = tx.store("features")
            .map_err(|e| JsValue::from_str(&format!("Store: {e}")))?;

        store.clear().await
            .map_err(|e| JsValue::from_str(&format!("Clear: {e}")))?;

        tx.done().await
            .map_err(|e| JsValue::from_str(&format!("Commit: {e}")))?;

        Ok(())
    }

    /// Get the number of stored features.
    #[wasm_bindgen(js_name = count)]
    pub async fn count(&self) -> Result<u32, JsValue> {
        let rexie = rexie::Rexie::builder(&self.db_name).build().await
            .map_err(|e| JsValue::from_str(&format!("DB open: {e}")))?;

        let tx = rexie.transaction(&["features"], rexie::TransactionMode::ReadOnly)
            .map_err(|e| JsValue::from_str(&format!("Transaction: {e}")))?;

        let store = tx.store("features")
            .map_err(|e| JsValue::from_str(&format!("Store: {e}")))?;

        let count = store.count(None).await
            .map_err(|e| JsValue::from_str(&format!("Count: {e}")))?;

        tx.done().await.ok();

        Ok(count)
    }
}
