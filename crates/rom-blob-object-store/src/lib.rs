//! Object-store adapters for the maintained ROM blob port.
#![doc = include_str!("../README.md")]
use futures_util::StreamExt;
use object_store::{
    ObjectStore, ObjectStoreExt, PutMode, PutOptions,
    aws::{AmazonS3Builder, S3ConditionalPut},
    local::LocalFileSystem,
    path::Path,
};
use rom_blob::{BlobStore, Error, Metadata, ObjectKey, Result, StoreFuture};
use std::{sync::Arc, time::Duration};

/// Provider configuration is host-owned; no SDK type crosses the blob contract.
pub struct Adapter {
    inner: Arc<dyn ObjectStore>,
    max_bytes: usize,
}
pub enum EndpointPolicy {
    HttpsOnly,
    LoopbackTestOnly,
}
pub struct S3Config<'a> {
    pub endpoint: &'a str,
    pub region: &'a str,
    pub bucket: &'a str,
    pub access_key: &'a str,
    pub secret: &'a str,
    pub policy: EndpointPolicy,
}
impl Adapter {
    /// Root and descendants must remain exclusively trusted. The underlying
    /// library follows symlinks; this is explicitly not a filesystem sandbox.
    pub fn trusted_folder(root: &std::path::Path, max_bytes: usize) -> Result<Self> {
        validate_limit(max_bytes)?;
        let inner = LocalFileSystem::new_with_prefix(root)
            .map_err(read_error)?
            .with_fsync(true);
        Ok(Self {
            inner: Arc::new(inner),
            max_bytes,
        })
    }
    /// Enable only for an endpoint certified by the same create-only contract suite.
    pub fn s3(config: S3Config<'_>, max_bytes: usize) -> Result<Self> {
        validate_limit(max_bytes)?;
        let endpoint = url::Url::parse(config.endpoint).map_err(|_| Error::Invalid)?;
        let allow_http = matches!(config.policy, EndpointPolicy::LoopbackTestOnly);
        let permitted = match config.policy {
            EndpointPolicy::HttpsOnly => endpoint.scheme() == "https",
            EndpointPolicy::LoopbackTestOnly => {
                endpoint.scheme() == "http"
                    && match endpoint.host() {
                        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
                        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
                        _ => false,
                    }
            }
        };
        if !permitted
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.fragment().is_some()
            || endpoint.query().is_some()
            || [
                config.region,
                config.bucket,
                config.access_key,
                config.secret,
            ]
            .iter()
            .any(|s| s.is_empty() || s.len() > 4096)
        {
            return Err(Error::Invalid);
        }
        let inner = AmazonS3Builder::new()
            .with_endpoint(config.endpoint)
            .with_region(config.region)
            .with_bucket_name(config.bucket)
            .with_access_key_id(config.access_key)
            .with_secret_access_key(config.secret)
            .with_virtual_hosted_style_request(false)
            .with_conditional_put(S3ConditionalPut::ETagMatch)
            .with_retry(object_store::RetryConfig {
                max_retries: 0,
                ..Default::default()
            })
            .with_client_options(
                object_store::ClientOptions::new()
                    .with_allow_http(allow_http)
                    .with_timeout(Duration::from_secs(3))
                    .with_connect_timeout(Duration::from_secs(1)),
            )
            .build()
            .map_err(read_error)?;
        Ok(Self {
            inner: Arc::new(inner),
            max_bytes,
        })
    }
}
impl BlobStore for Adapter {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            if bytes.len() > self.max_bytes {
                return Err(Error::TooLarge);
            }
            self.inner
                .put_opts(
                    &Path::from(key.as_str()),
                    bytes.into(),
                    PutOptions {
                        mode: PutMode::Create,
                        ..Default::default()
                    },
                )
                .await
                .map_err(write_error)?;
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let limit = max_bytes.min(self.max_bytes);
            let result = self
                .inner
                .get(&Path::from(key.as_str()))
                .await
                .map_err(read_error)?;
            if result.meta.size > limit as u64 {
                return Err(Error::TooLarge);
            }
            let mut stream = result.into_stream();
            let mut bytes = Vec::new();
            let mut chunks = 0usize;
            while let Some(chunk) = stream.next().await {
                chunks += 1;
                let chunk = chunk.map_err(read_error)?;
                if chunks > 65_536 || chunk.len() > limit - bytes.len() {
                    return Err(Error::TooLarge);
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(bytes)
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        Box::pin(async move {
            let meta = self
                .inner
                .head(&Path::from(key.as_str()))
                .await
                .map_err(read_error)?;
            Ok(Metadata { bytes: meta.size })
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            match self.inner.delete(&Path::from(key.as_str())).await {
                Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
                Err(error) => Err(write_error(error)),
            }
        })
    }
}
fn validate_limit(bytes: usize) -> Result<()> {
    if bytes == 0 || bytes > 16 * 1024 * 1024 {
        Err(Error::Invalid)
    } else {
        Ok(())
    }
}
fn read_error(error: object_store::Error) -> Error {
    match error {
        object_store::Error::NotFound { .. } => Error::Missing,
        object_store::Error::PermissionDenied { .. } => Error::Denied,
        object_store::Error::NotImplemented { .. } | object_store::Error::NotSupported { .. } => {
            Error::Unsupported
        }
        _ => Error::Backend,
    }
}
fn write_error(error: object_store::Error) -> Error {
    match error {
        object_store::Error::AlreadyExists { .. } | object_store::Error::Precondition { .. } => {
            Error::Conflict
        }
        object_store::Error::PermissionDenied { .. } => Error::Denied,
        object_store::Error::NotImplemented { .. } | object_store::Error::NotSupported { .. } => {
            Error::Unsupported
        }
        _ => Error::Unknown,
    }
}
