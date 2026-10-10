//! Protected host key and length-delimited, domain-separated correlation.
use super::DiagnosticToken;
use crate::{Error, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// A host-supplied diagnostic key. Deliberately has no Debug or serialization implementation.
/// This type protects API disclosure; it does not promise allocator-level memory erasure.
pub struct DiagnosticKey([u8; 32]);
impl DiagnosticKey {
    pub fn new(key: [u8; 32]) -> Self {
        Self(key)
    }
}

pub(super) struct Correlation {
    mac: HmacSha256,
}
#[derive(Clone, Copy)]
pub(super) enum Domain {
    Operation,
    Root,
    Work,
    Claim,
    Lifecycle,
}
impl Domain {
    fn tag(self) -> &'static [u8] {
        match self {
            Self::Operation => b"operation",
            Self::Root => b"root",
            Self::Work => b"work",
            Self::Claim => b"claim",
            Self::Lifecycle => b"lifecycle",
        }
    }
}
impl Correlation {
    pub(super) fn new(key: DiagnosticKey) -> Result<Self> {
        let mac = HmacSha256::new_from_slice(&key.0).map_err(|_| Error::Storage)?;
        Ok(Self { mac })
    }
    pub(super) fn token(&self, domain: Domain, components: &[&[u8]]) -> DiagnosticToken {
        let mut mac = self.mac.clone();
        mac.update(b"ROM-diagnostics\0v1\0");
        let tag = domain.tag();
        mac.update(&(tag.len() as u64).to_be_bytes());
        mac.update(tag);
        mac.update(&(components.len() as u64).to_be_bytes());
        for component in components {
            mac.update(&(component.len() as u64).to_be_bytes());
            mac.update(component);
        }
        DiagnosticToken(mac.finalize().into_bytes().into())
    }
}
