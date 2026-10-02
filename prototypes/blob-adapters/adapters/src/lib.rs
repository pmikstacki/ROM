use blob_contract::{
    BlobStore, Capabilities, Condition, Error, FutureResult, Limits, Receipt, Upload,
};
use futures_util::StreamExt;
use object_store::{
    ObjectStore, ObjectStoreExt, aws::AmazonS3Builder, local::LocalFileSystem, path::Path,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};

pub struct Adapter {
    inner: Arc<dyn ObjectStore>,
    limits: Limits,
}

impl Adapter {
    /// The directory and all its descendants must remain exclusively trusted.
    /// The underlying library follows symlinks: this is not a filesystem sandbox.
    pub fn trusted_folder(root: &std::path::Path, limits: Limits) -> Result<Self, Error> {
        validate_limits(limits)?;
        let inner = LocalFileSystem::new_with_prefix(root)
            .map_err(read_error)?
            .with_fsync(true);
        Ok(Self {
            inner: Arc::new(inner),
            limits,
        })
    }

    /// Deliberately loopback-only: this experiment never needs a cloud endpoint.
    pub fn loopback_s3(
        endpoint: SocketAddr,
        bucket: &str,
        access_key: &str,
        secret: &str,
        limits: Limits,
    ) -> Result<Self, Error> {
        validate_limits(limits)?;
        if !endpoint.ip().is_loopback() {
            return Err(Error::Denied);
        }
        let inner = AmazonS3Builder::new()
            .with_endpoint(format!("http://{endpoint}"))
            .with_bucket_name(bucket)
            .with_region("us-east-1")
            .with_access_key_id(access_key)
            .with_secret_access_key(secret)
            .with_virtual_hosted_style_request(false)
            .with_retry(object_store::RetryConfig {
                max_retries: 0,
                ..Default::default()
            })
            .with_client_options(
                object_store::ClientOptions::new()
                    .with_allow_http(true)
                    .with_timeout(Duration::from_secs(3)),
            )
            .build()
            .map_err(read_error)?;
        Ok(Self {
            inner: Arc::new(inner),
            limits,
        })
    }
}

impl BlobStore for Adapter {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            conditional_write: false,
            limits: self.limits,
        }
    }
    fn put<'a>(
        &'a self,
        key: &'a str,
        mut input: Upload,
        condition: Condition,
    ) -> FutureResult<'a, Receipt> {
        Box::pin(async move {
            let path = key_path(key)?;
            if condition != Condition::Any {
                return Err(Error::UnsupportedCondition);
            }
            // Fixed capped staging allocation. Producer/provider buffers are separate.
            let mut body = Vec::with_capacity(self.limits.max_blob_bytes);
            let mut chunks = 0;
            while let Some(chunk) = input.next().await {
                if chunks == self.limits.max_chunks {
                    return Err(Error::LimitExceeded);
                }
                chunks += 1;
                let chunk = chunk?;
                if chunk.len() > self.limits.max_chunk_bytes
                    || chunk.len() > self.limits.max_blob_bytes - body.len()
                {
                    return Err(Error::LimitExceeded);
                }
                body.extend_from_slice(&chunk);
            }
            let bytes = body.len();
            self.inner
                .put(&path, body.into())
                .await
                .map_err(write_error)?;
            Ok(Receipt { bytes })
        })
    }
    fn get<'a>(&'a self, key: &'a str) -> FutureResult<'a, Vec<u8>> {
        Box::pin(async move {
            let result = self.inner.get(&key_path(key)?).await.map_err(read_error)?;
            if result.meta.size > self.limits.max_blob_bytes as u64 {
                return Err(Error::LimitExceeded);
            }
            let mut stream = result.into_stream();
            let mut body = Vec::with_capacity(self.limits.max_blob_bytes);
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(read_error)?;
                if chunk.len() > self.limits.max_blob_bytes - body.len() {
                    return Err(Error::LimitExceeded);
                }
                body.extend_from_slice(&chunk);
            }
            Ok(body)
        })
    }
    fn delete<'a>(&'a self, key: &'a str) -> FutureResult<'a, ()> {
        Box::pin(async move {
            match self.inner.delete(&key_path(key)?).await {
                Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
                Err(error) => Err(write_error(error)),
            }
        })
    }
}

fn validate_limits(limits: Limits) -> Result<(), Error> {
    if limits.max_blob_bytes == 0
        || limits.max_blob_bytes > 16 * 1024 * 1024
        || limits.max_chunk_bytes == 0
        || limits.max_chunk_bytes > limits.max_blob_bytes
        || limits.max_chunks == 0
        || limits.max_chunks > 65_536
    {
        return Err(Error::LimitExceeded);
    }
    Ok(())
}

fn key_path(key: &str) -> Result<Path, Error> {
    if key.len() > 240
        || key.split('/').any(|part| {
            part.is_empty()
                || part.len() > 64
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        })
    {
        return Err(Error::InvalidKey);
    }
    Ok(Path::from(key))
}

fn read_error(error: object_store::Error) -> Error {
    match error {
        object_store::Error::NotFound { .. } => Error::NotFound,
        object_store::Error::PermissionDenied { .. } => Error::Denied,
        _ => Error::Backend,
    }
}

fn write_error(error: object_store::Error) -> Error {
    match error {
        object_store::Error::PermissionDenied { .. } => Error::Denied,
        // Conservatively retain ambiguity after crossing the provider boundary.
        _ => Error::Unknown,
    }
}
