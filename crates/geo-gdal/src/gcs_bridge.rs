//! GCS Bridge: Google Cloud Storage → MinIO / local storage bridge.
//!
//! Downloads exported raster/vector data from GCS (where GEE puts results),
//! optionally converts to COG, and uploads to MinIO or local filesystem.
//!
//! Uses `gsutil cp` for download and the `object_store` crate for MinIO upload
//! when the `minio` feature is enabled.

use crate::raster::RasterOps;
use geo_core::errors::{GeoError, GeoResult};
use std::path::{Path, PathBuf};

/// Data type of the asset being bridged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetType {
    /// Raster (GeoTIFF, COG, etc.)
    Raster,
    /// Vector (GeoJSON, GPKG, etc.)
    Vector,
    /// Tabular (CSV, Parquet, etc.)
    Table,
}

impl AssetType {
    /// Guess from file extension.
    pub fn from_path(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".tif") || lower.ends_with(".tiff") || lower.ends_with(".cog.tif") {
            AssetType::Raster
        } else if lower.ends_with(".geojson")
            || lower.ends_with(".gpkg")
            || lower.ends_with(".shp")
        {
            AssetType::Vector
        } else {
            AssetType::Table
        }
    }

    /// Prefix for organizing assets in MinIO.
    pub fn prefix(&self) -> &str {
        match self {
            AssetType::Raster => "cog",
            AssetType::Vector => "vector",
            AssetType::Table => "table",
        }
    }
}

/// Bridge configuration.
#[derive(Debug, Clone)]
pub struct GcsBridgeConfig {
    /// MinIO / S3 endpoint URL.
    pub minio_endpoint: Option<String>,
    /// MinIO access key.
    pub minio_access_key: Option<String>,
    /// MinIO secret key.
    pub minio_secret_key: Option<String>,
    /// MinIO bucket name.
    pub minio_bucket: Option<String>,
    /// Local download directory (used when no MinIO config).
    pub local_dir: Option<PathBuf>,
    /// Number of retries for downloads.
    pub retries: u8,
    /// Timeout for gsutil operations (seconds).
    pub timeout_secs: u64,
}

impl Default for GcsBridgeConfig {
    fn default() -> Self {
        Self {
            minio_endpoint: std::env::var("MINIO_ENDPOINT").ok(),
            minio_access_key: std::env::var("MINIO_ACCESS_KEY").ok(),
            minio_secret_key: std::env::var("MINIO_SECRET_KEY").ok(),
            minio_bucket: std::env::var("MINIO_BUCKET")
                .or_else(|_| std::env::var("GEO_DATA_BUCKET"))
                .ok(),
            local_dir: Some(
                dirs_next().unwrap_or_else(|| PathBuf::from(".")),
            ),
            retries: 3,
            timeout_secs: 600,
        }
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("GEO_DATA_DIR")
        .ok()
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from("./geo_data")))
}

/// GCS → target storage bridge.
pub struct GcsBridge {
    config: GcsBridgeConfig,
}

impl GcsBridge {
    /// Create a new bridge with the given configuration.
    pub fn new(config: GcsBridgeConfig) -> Self {
        Self { config }
    }

    /// Sync a file from GCS to local storage (and optionally MinIO).
    ///
    /// Steps:
    /// 1. Download from GCS using `gsutil cp`
    /// 2. If raster and convert_to_cog is true, run gdal_translate -of COG
    /// 3. If MinIO configured, upload; otherwise keep local copy
    ///
    /// Returns the destination path (local or `s3://` URI).
    pub async fn sync(
        &self,
        gcs_uri: &str,
        target_prefix: &str,
        convert_to_cog: bool,
    ) -> GeoResult<String> {
        // Validate input.
        if !gcs_uri.starts_with("gs://") {
            return Err(GeoError::GcsBridge(format!(
                "Invalid GCS URI (must start with gs://): {gcs_uri}"
            )));
        }

        // Determine file name and asset type.
        let file_name = gcs_uri
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .to_string();
        let asset_type = AssetType::from_path(&file_name);

        // Local download directory.
        let local_dir = self
            .config
            .local_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("./geo_data"));

        let download_dir = local_dir.join("downloads");
        tokio::fs::create_dir_all(&download_dir).await?;

        let suffix = if convert_to_cog && asset_type == AssetType::Raster {
            ".cog.tif"
        } else {
            ""
        };

        let base_name = file_name.trim_end_matches(".tif").trim_end_matches(".tiff");
        let local_path = download_dir.join(format!("{base_name}{suffix}"));

        // Step 1: Download from GCS.
        self.download_from_gcs(gcs_uri, &local_path).await?;

        // Step 2: COG conversion for rasters.
        let final_local = if convert_to_cog && asset_type == AssetType::Raster {
            let cog_path = download_dir.join(format!("{base_name}.cog.tif"));
            RasterOps::to_cog(&local_path, &cog_path, None).await?;
            // Clean up the original download.
            let _ = tokio::fs::remove_file(&local_path).await;
            cog_path
        } else {
            local_path
        };

        // Step 3: Upload to MinIO if configured, otherwise return local path.
        if let (Some(_endpoint), Some(bucket)) =
            (&self.config.minio_endpoint, &self.config.minio_bucket)
        {
            let minio_path = format!(
                "{}/{}/{file_name}",
                target_prefix,
                asset_type.prefix()
            );

            self.upload_to_minio(&final_local, &minio_path).await?;

            let uri = format!("s3://{bucket}/{minio_path}");
            tracing::info!("GCS → MinIO sync complete: {uri}");
            Ok(uri)
        } else {
            let path_str = final_local.to_string_lossy().to_string();
            tracing::info!("GCS → local sync complete: {path_str}");
            Ok(path_str)
        }
    }

    /// Download a file from GCS using `gsutil cp`.
    async fn download_from_gcs(
        &self,
        gcs_uri: &str,
        local_path: &Path,
    ) -> GeoResult<()> {
        let mut last_error = None;

        for attempt in 0..=self.config.retries {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                tracing::warn!("Retrying GCS download (attempt {attempt}/{} after {delay:?})...", self.config.retries);
                tokio::time::sleep(delay).await;
            }

            match tokio::process::Command::new("gsutil")
                .args([
                    "-o",
                    &format!(
                        "GSUtil:default_api_scheme_timeout_secs={}",
                        self.config.timeout_secs
                    ),
                    "cp",
                    gcs_uri,
                    &local_path.to_string_lossy(),
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .await
            {
                Ok(output) if output.status.success() => {
                    tracing::info!("Downloaded {gcs_uri} → {}", local_path.display());
                    return Ok(());
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    last_error = Some(stderr.trim().to_string());
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }
        }

        Err(GeoError::GcsBridge(format!(
            "Failed to download {gcs_uri} after {} retries: {}",
            self.config.retries + 1,
            last_error.unwrap_or_else(|| "unknown error".into())
        )))
    }

    /// Upload a local file to MinIO/S3 using the object_store crate.
    #[cfg(feature = "object_store")]
    async fn upload_to_minio(
        &self,
        local_path: &Path,
        minio_path: &str,
    ) -> GeoResult<()> {
        use object_store::aws::AmazonS3Builder;
        use object_store::path::Path as ObjectPath;
        use object_store::ObjectStore;

        let endpoint = self.config.minio_endpoint.as_ref().unwrap();
        let access_key = self.config.minio_access_key.as_deref().unwrap_or("");
        let secret_key = self.config.minio_secret_key.as_deref().unwrap_or("");
        let bucket = self.config.minio_bucket.as_ref().unwrap();

        let store = AmazonS3Builder::new()
            .with_endpoint(endpoint)
            .with_bucket_name(bucket)
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key)
            .with_allow_http(true)
            .with_region("us-east-1")
            .build()
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;

        let data = tokio::fs::read(local_path).await?;
        let path = ObjectPath::from(minio_path);

        store
            .put(&path, data.into())
            .await
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;

        tracing::info!("Uploaded to MinIO: s3://{bucket}/{minio_path}");
        Ok(())
    }

    /// No-op upload when object_store feature is disabled.
    #[cfg(not(feature = "object_store"))]
    async fn upload_to_minio(
        &self,
        _local_path: &Path,
        minio_path: &str,
    ) -> GeoResult<()> {
        tracing::warn!(
            "MinIO upload requested but `minio` feature is not enabled. \
             File will remain local. Destination would be: {minio_path}"
        );
        Ok(())
    }

    /// Bulk sync: download multiple files from a list of GCS URIs.
    pub async fn sync_bulk(
        &self,
        gcs_uris: &[(String, String)], // (gcs_uri, target_prefix)
        convert_to_cog: bool,
    ) -> GeoResult<Vec<String>> {
        let mut handles = Vec::new();

        for (uri, prefix) in gcs_uris {
            let bridge = GcsBridge {
                config: self.config.clone(),
            };
            let uri = uri.clone();
            let prefix = prefix.clone();

            handles.push(tokio::spawn(async move {
                bridge.sync(&uri, &prefix, convert_to_cog).await
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for h in handles {
            results.push(
                h.await
                    .map_err(|e| GeoError::Other(format!("Join error: {e}")))??,
            );
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_detection() {
        assert_eq!(
            AssetType::from_path("landcover_2025.tif"),
            AssetType::Raster
        );
        assert_eq!(
            AssetType::from_path("sites.geojson"),
            AssetType::Vector
        );
        assert_eq!(
            AssetType::from_path("landcover.gpkg"),
            AssetType::Vector
        );
        assert_eq!(
            AssetType::from_path("emissions.csv"),
            AssetType::Table
        );
    }

    #[test]
    fn test_asset_type_prefix() {
        assert_eq!(AssetType::Raster.prefix(), "cog");
        assert_eq!(AssetType::Vector.prefix(), "vector");
        assert_eq!(AssetType::Table.prefix(), "table");
    }

    #[test]
    fn test_reject_non_gcs_uri() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let bridge = GcsBridge::new(GcsBridgeConfig::default());
        let result = rt.block_on(bridge.sync(
            "https://example.com/file.tif",
            "test",
            false,
        ));
        assert!(result.is_err());
    }
}
