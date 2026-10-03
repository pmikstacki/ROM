#![cfg(feature = "provider-profile")]

#[path = "provider_auth/admission.rs"]
mod admission;
#[path = "provider_auth/approval.rs"]
mod approval;
#[path = "provider_auth/files.rs"]
mod files;
#[path = "provider_auth/outage.rs"]
mod outage;
#[path = "provider_auth/scratch.rs"]
mod scratch;
#[path = "provider_auth/support.rs"]
mod support;
