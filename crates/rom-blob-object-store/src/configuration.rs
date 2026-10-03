//! Host-owned folder and S3 provider configuration.
use crate::{Adapter, errors::read_error};
use object_store::{
    aws::{AmazonS3Builder, S3ConditionalPut},
    local::LocalFileSystem,
};
use rom_blob::{Error, Result};
use std::{sync::Arc, time::Duration};

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
fn validate_limit(bytes: usize) -> Result<()> {
    if bytes == 0 || bytes > 16 * 1024 * 1024 {
        Err(Error::Invalid)
    } else {
        Ok(())
    }
}
