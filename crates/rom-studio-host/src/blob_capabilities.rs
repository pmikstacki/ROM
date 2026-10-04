//! Current-authorized transport metadata, separate from operation permissions.
use crate::{
    blobs::{authorize, failure},
    router::{Shared, no_store},
};
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use rom::Resource;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};
pub(crate) async fn read(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let actor = match authorize(&shared, request.headers(), false).await {
        Ok(a) => a,
        Err(e) => return *e,
    };
    let discovery = match shared.runtime.discover(&actor).await {
        Ok(d) => d,
        Err(e) => return failure(e.into()),
    };
    if !discovery
        .resources
        .iter()
        .any(|r| r.kind == rom_blob::Blob::KIND && r.fields.iter().any(|f| f.name == "store"))
    {
        return failure(rom_blob::Error::Denied);
    }
    let Some(service) = &shared.config.blobs else {
        return failure(rom_blob::Error::Missing);
    };
    let mut stores = Vec::new();
    for name in service.store_names() {
        match catch_unwind(AssertUnwindSafe(|| {
            (shared.config.blob_store_discovery)(&actor, name)
        })) {
            Ok(true) => stores.push(name),
            Ok(false) => {}
            Err(_) => return failure(rom_blob::Error::Panicked),
        }
    }
    let limits = service.limits();
    no_store(axum::Json(serde_json::json!({"version":1,"resource_kind":rom_blob::Blob::KIND,"stores":stores,"limits":{"blob_bytes":limits.blob_bytes,"chunk_bytes":limits.chunk_bytes,"chunks":limits.chunks},"operations":["reserve","upload","download","detach"]})).into_response())
}
