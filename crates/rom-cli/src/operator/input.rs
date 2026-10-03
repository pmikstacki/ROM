//! Bounded operator input through the existing file and strict JSON reader.
use super::WorkCommand;
use crate::{
    Failure,
    input::{self, Request},
};
use rom::operator::{
    WorkControlOperation, WorkControlRequest, WorkHandle, WorkQuery, WorkResponseLimits,
};
use serde::Serialize;
use serde_json::{Value, json};

fn encode(value: impl Serialize) -> Result<Value, Failure> {
    serde_json::to_value(value).map_err(|_| Failure::local("invalid operator request"))
}
fn object_file<T: serde::de::DeserializeOwned>(
    path: &str,
    invalid: &'static str,
) -> Result<T, Failure> {
    let value = input::file(path)?;
    if !value.is_object() {
        return Err(Failure::local("expected an operator request object"));
    }
    serde_json::from_value(value).map_err(|_| Failure::local(invalid))
}
pub fn request(command: &WorkCommand) -> Result<Request, Failure> {
    let (route, body, mutation) = match command {
        WorkCommand::Capabilities => ("/work/capabilities", json!({}), false),
        WorkCommand::List { query_file } => {
            let query: WorkQuery = match query_file {
                Some(path) => object_file(path, "invalid work query envelope")?,
                None => WorkQuery {
                    state: None,
                    category: None,
                    definition: None,
                    limit: 64,
                    cursor: None,
                },
            };
            query
                .validate(&WorkResponseLimits {
                    max_records: query.limit,
                    max_bytes: crate::sse::FRAME_LIMIT,
                })
                .map_err(|_| Failure::local("invalid work query bounds"))?;
            ("/work/list", encode(query)?, false)
        }
        WorkCommand::Show { handle } => {
            let handle = WorkHandle::try_from(handle.clone())
                .map_err(|_| Failure::local("invalid work handle"))?;
            ("/work/read", json!({"handle":handle}), false)
        }
        WorkCommand::Retry { request_file } | WorkCommand::Reconcile { request_file } => {
            let request: WorkControlRequest =
                object_file(request_file, "invalid work control envelope")?;
            request
                .validate()
                .map_err(|_| Failure::local("invalid work control envelope"))?;
            if !matches!(
                (command, &request.operation),
                (WorkCommand::Retry { .. }, WorkControlOperation::Retry)
                    | (
                        WorkCommand::Reconcile { .. },
                        WorkControlOperation::Reconcile { .. }
                    )
            ) {
                return Err(Failure::local(
                    "work control operation does not match the command",
                ));
            }
            ("/work/control", encode(request)?, true)
        }
    };
    input::bound(Request {
        route,
        body,
        mutation,
        streaming: false,
        kind_filter: None,
    })
}
