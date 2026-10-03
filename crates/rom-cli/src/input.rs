use crate::{
    Failure,
    args::{Command, Mutation},
    json,
};
use rom::{Invocation, Operation};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{Read, stdin},
};
pub const REQUEST_LIMIT: usize = 64 * 1024;
pub const AUTH_LIMIT: usize = 8 * 1024;
pub struct Request {
    pub route: &'static str,
    pub body: Value,
    pub mutation: bool,
    pub streaming: bool,
    pub kind_filter: Option<String>,
}
pub fn bytes(reader: impl Read, limit: usize) -> Result<Vec<u8>, Failure> {
    let mut out = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| Failure::local("input could not be read"))?;
    if out.len() > limit {
        return Err(Failure::local("input exceeds its byte limit"));
    }
    Ok(out)
}
pub fn file(path: &str) -> Result<Value, Failure> {
    let data = if path == "-" {
        bytes(stdin().lock(), REQUEST_LIMIT)?
    } else {
        bytes(
            File::open(path).map_err(|_| Failure::local("input file could not be opened"))?,
            REQUEST_LIMIT,
        )?
    };
    json::parse(&data).map_err(|_| Failure::local("invalid JSON or duplicate object key"))
}
fn invocation(m: &Mutation, operation: Operation) -> Invocation {
    Invocation {
        retry_epoch: m.retry_epoch,
        kind: m.kind.clone(),
        id: m.id.clone(),
        expected: Some(m.expected),
        idempotency: m.idempotency.clone(),
        operation,
    }
}
pub fn request(command: &Command) -> Result<Request, Failure> {
    if let Command::Work { command } = command {
        return crate::operator::request(command);
    }
    let mut result = Request {
        route: "/invoke",
        body: Value::Null,
        mutation: true,
        streaming: false,
        kind_filter: None,
    };
    let invoke = match command {
        Command::Create {
            kind,
            id,
            retry_epoch,
            idempotency,
            input_file,
        } => Some(Invocation {
            retry_epoch: *retry_epoch,
            kind: kind.clone(),
            id: id.clone(),
            expected: None,
            idempotency: idempotency.clone(),
            operation: Operation::Create(file(input_file)?),
        }),
        Command::Replace {
            mutation,
            input_file,
        } => Some(invocation(mutation, Operation::Replace(file(input_file)?))),
        Command::Patch {
            mutation,
            input_file,
        } => Some(invocation(
            mutation,
            Operation::Patch(
                serde_json::from_value(file(input_file)?)
                    .map_err(|_| Failure::local("invalid patch envelope"))?,
            ),
        )),
        Command::Delete(m) => Some(invocation(m, Operation::Delete)),
        Command::Action {
            mutation,
            name,
            input_file,
        } => Some(invocation(
            mutation,
            Operation::Action {
                name: name.clone(),
                input: file(input_file)?,
            },
        )),
        Command::Invoke { request_file } => Some(
            serde_json::from_value(file(request_file)?)
                .map_err(|_| Failure::local("invalid invocation envelope"))?,
        ),
        _ => None,
    };
    if let Some(invoke) = invoke {
        if invoke.kind.is_empty() || invoke.id.is_empty() || invoke.idempotency.is_empty() {
            return Err(Failure::local(
                "kind, ID and idempotency key must be nonempty",
            ));
        }
        // Null means absence, not an implicit force/overwrite operation.
        if !matches!(invoke.operation, Operation::Create(_)) && invoke.expected.is_none() {
            return Err(Failure::local("mutation requires an expected revision"));
        }
        result.body =
            serde_json::to_value(invoke).map_err(|_| Failure::local("invalid invocation"))?;
    } else {
        result.mutation = false;
        match command {
            Command::Discover { kind } => {
                result.route = "/discover";
                result.body = json!({});
                result.kind_filter = kind.clone();
            }
            Command::Read { kind, id } => {
                result.route = "/read";
                result.body = json!({"kind":kind,"id":id});
            }
            Command::Query(q) | Command::Live(q) => {
                let spec: rom::QuerySpec = match &q.query_file {
                    Some(path) => serde_json::from_value(file(path)?)
                        .map_err(|_| Failure::local("invalid query envelope"))?,
                    None => rom::QuerySpec::all(),
                };
                result.streaming = matches!(command, Command::Live(_));
                result.route = if result.streaming { "/live" } else { "/query" };
                result.body = json!({"kind":q.kind,"query":spec});
            }
            Command::Journal(j) | Command::Subscribe(j) => {
                let after: Option<rom::JournalCursor> = j
                    .after_file
                    .as_ref()
                    .map(|p| {
                        file(p).and_then(|v| {
                            serde_json::from_value(v)
                                .map_err(|_| Failure::local("invalid journal cursor"))
                        })
                    })
                    .transpose()?;
                result.streaming = matches!(command, Command::Subscribe(_));
                result.route = if result.streaming {
                    "/subscribe"
                } else {
                    "/journal"
                };
                result.body = json!({"kind":j.kind,"after":after});
            }
            Command::JournalHead { kind } => {
                result.route = "/journal/head";
                result.body = json!({"kind":kind});
            }
            _ => unreachable!(),
        }
    }
    bound(result)
}
pub(crate) fn bound(result: Request) -> Result<Request, Failure> {
    if serde_json::to_vec(&result.body)
        .map_err(|_| Failure::local("invalid request"))?
        .len()
        > REQUEST_LIMIT
    {
        return Err(Failure::local("request exceeds 64 KiB"));
    }
    Ok(result)
}
