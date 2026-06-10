//! MinIO / S3 / GCS object store — unified interface via `object_store`.
//!
//! Requires the `minio` feature. Provides COG (Cloud-Optimized GeoTIFF) upload,
//! GeoJSON blob storage, and presigned URL generation for sharing.
//!
//! ## Authentication (Risk 5)
//!
//! Priority order:
//! 1. `--gcs-key-file` / env `GCS_KEY_FILE` — explicit service account JSON
//! 2. `GOOGLE_APPLICATION_CREDENTIALS` — standard ADC
//! 3. AWS credentials env vars (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
//! 4. IAM instance role (GCE / EC2)

use geo_core::errors::{GeoError, GeoResult};
use object_store::aws::AmazonS3Builder;
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::path::Path as ObjectPath;
use object_store::ObjectStore;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Supported object storage backends.
#[derive(Debug, Clone)]
pub enum StorageBackend {
    /// Amazon S3 or S3-compatible (MinIO).
    S3 {
        /// Endpoint URL (e.g. `http://localhost:9000` for MinIO).
        endpoint: String,
        /// Access key.
        access_key: String,
        /// Secret key.
        secret_key: String,
        /// Bucket name.
        bucket: String,
        /// Region (default `us-east-1` for MinIO).
        region: String,
        /// Use HTTP instead of HTTPS (MinIO default).
        allow_http: bool,
    },
    /// Google Cloud Storage.
    Gcs {
        /// Bucket name.
        bucket: String,
        /// Optional explicit service account key file path.
        service_account_key: Option<String>,
    },
}

/// High-level object store client.
///
/// ## Usage
/// ```ignore
/// let minio = ObjectStoreClient::s3(
///     "http://localhost:9000", "minioadmin", "minioadmin", "geo-data"
/// )?;
/// let url = minio.put_geotiff(b"COG bytes").await?;
/// ```
pub struct ObjectStoreClient {
    inner: Box<dyn ObjectStore>,
    bucket: String,
}

impl ObjectStoreClient {
    /// Create an S3-compatible client (MinIO / AWS S3).
    pub fn s3(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
    ) -> GeoResult<Self> {
        let store = AmazonS3Builder::new()
            .with_endpoint(endpoint)
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key)
            .with_region("us-east-1")
            .with_allow_http(true)
            .with_bucket_name(bucket)
            .build()
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;

        Ok(Self {
            inner: Box::new(store),
            bucket: bucket.to_string(),
        })
    }

    /// Create a GCS client using Application Default Credentials.
    ///
    /// For explicit service account keys, set
    /// `GOOGLE_APPLICATION_CREDENTIALS` before calling this, or pass
    /// `service_account_key` as `Some(path)` to override.
    pub fn gcs(
        bucket: &str,
        service_account_key: Option<&str>,
    ) -> GeoResult<Self> {
        // Override ADC if explicit key provided
        if let Some(key_path) = service_account_key {
            std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", key_path);
        }

        let store = GoogleCloudStorageBuilder::from_env()
            .with_bucket_name(bucket)
            .build()
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;

        Ok(Self {
            inner: Box::new(store),
            bucket: bucket.to_string(),
        })
    }

    /// Auto-detect backend from a URI scheme.
    ///
    /// - `s3://bucket/prefix` → S3 (MinIO)  
    /// - `gs://bucket/prefix` → GCS
    pub fn from_uri(
        uri: &str,
        s3_endpoint: Option<&str>,
        s3_access_key: Option<&str>,
        s3_secret_key: Option<&str>,
    ) -> GeoResult<Self> {
        if uri.starts_with("gs://") {
            let bucket = uri
                .strip_prefix("gs://")
                .and_then(|s| s.split('/').next())
                .unwrap_or("default");
            Self::gcs(bucket, None)
        } else if uri.starts_with("s3://") {
            let bucket = uri
                .strip_prefix("s3://")
                .and_then(|s| s.split('/').next())
                .unwrap_or("default");
            Self::s3(
                s3_endpoint.unwrap_or("http://localhost:9000"),
                s3_access_key.unwrap_or(""),
                s3_secret_key.unwrap_or(""),
                bucket,
            )
        } else {
            Err(GeoError::ObjectStore(format!(
                "Unsupported URI scheme: {uri}"
            )))
        }
    }

    /// Put a Cloud-Optimized GeoTIFF (COG) and return its object path.
    ///
    /// Files are stored under `cog/<uuid>.tif` for deduplication.
    pub async fn put_geotiff(
        &self,
        data: bytes::Bytes,
    ) -> GeoResult<String> {
        let id = Uuid::new_v4();
        let path = format!("cog/{id}.tif");
        self.put(&path, data).await?;
        Ok(path)
    }

    /// Put a GeoJSON file and return its object path.
    pub async fn put_geojson(
        &self,
        name: &str,
        data: bytes::Bytes,
    ) -> GeoResult<String> {
        let path = format!("geojson/{name}");
        self.put(&path, data).await?;
        Ok(path)
    }

    /// Upload arbitrary bytes to a key.
    pub async fn put(
        &self,
        key: &str,
        data: bytes::Bytes,
    ) -> GeoResult<()> {
        let path = ObjectPath::from(key);
        self.inner
            .put(&path, data)
            .await
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;

        tracing::info!("Object uploaded: {key} ({} bytes)", data.len());
        Ok(())
    }

    /// Get an object as bytes.
    pub async fn get(&self, key: &str) -> GeoResult<bytes::Bytes> {
        let path = ObjectPath::from(key);
        let result = self
            .inner
            .get(&path)
            .await
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;
        let data = result
            .bytes()
            .await
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;
        Ok(data)
    }

    /// Check if an object exists.
    pub async fn exists(&self, key: &str) -> GeoResult<bool> {
        let path = ObjectPath::from(key);
        match self.inner.head(&path).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(GeoError::ObjectStore(e.to_string())),
        }
    }

    /// Delete an object.
    pub async fn delete(&self, key: &str) -> GeoResult<()> {
        let path = ObjectPath::from(key);
        self.inner
            .delete(&path)
            .await
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;
        Ok(())
    }

    /// Generate a presigned download URL (S3 only, 1-hour expiry).
    #[cfg(feature = "minio")]
    pub async fn presigned_get(
        &self,
        key: &str,
        expiry: Duration,
    ) -> GeoResult<String> {
        use object_store::signer::Signer;
        let path = ObjectPath::from(key);
        // object_store 0.11 signer API
        let url = self
            .inner
            .signed_url(object_store::signer::SignOptions {
                expires_in: expiry,
                ..Default::default()
            },
            &path,
            )
            .await
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?;
        Ok(url.to_string())
    }

    /// List objects under a prefix.
    pub async fn list(
        &self,
        prefix: &str,
    ) -> GeoResult<Vec<String>> {
        let path = ObjectPath::from(prefix);
        let mut stream = self.inner.list(Some(&path));
        let mut keys = Vec::new();

        while let Some(item) = stream
            .next()
            .await
            .transpose()
            .map_err(|e| GeoError::ObjectStore(e.to_string()))?
        {
            keys.push(item.location.to_string());
        }

        Ok(keys)
    }

    /// Full S3/GCS URI for an object.
    pub fn uri(&self, key: &str) -> String {
        format!("s3://{}/{key}", self.bucket)
    }
}

// object_store 0.11 uses Stream for list
use futures::StreamExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_uri_s3() {
        let key = std::env::var("MINIO_TEST_KEY").unwrap_or_else(|_| "test_key".into());
        let secret = std::env::var("MINIO_TEST_SECRET").unwrap_or_else(|_| "test_secret".into());
        let client = ObjectStoreClient::from_uri(
            "s3://my-bucket/data/",
            Some("http://localhost:9000"),
            Some(&key),
            Some(&secret),
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_from_uri_gcs() {
        let client = ObjectStoreClient::from_uri(
            "gs://my-bucket/data/",
            None,
            None,
            None,
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_invalid_uri() {
        let client = ObjectStoreClient::from_uri(
            "ftp://bad/",
            None, None, None,
        );
        assert!(client.is_err());
    }
}
