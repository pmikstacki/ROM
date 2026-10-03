use crate::{
    Failure,
    args::{Cli, Format},
    input::{self, Request},
    json, output,
    response::{prior, valid},
    sse,
};
use reqwest::{
    Url,
    header::{AUTHORIZATION, HeaderValue},
};
use serde_json::Value;
use std::{
    fs::File,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub struct Client {
    http: reqwest::Client,
    base: Url,
    auth: Option<HeaderValue>,
    request: Duration,
    idle: Duration,
}
impl Client {
    pub fn new(cli: &Cli) -> Result<Self, Failure> {
        let mut base = Url::parse(
            cli.endpoint
                .as_deref()
                .ok_or_else(|| Failure::local("--endpoint is required"))?,
        )
        .map_err(|_| Failure::local("invalid endpoint URL"))?;
        if !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(Failure::local(
                "endpoint must not contain credentials, a query or a fragment",
            ));
        }
        let loopback = base
            .host_str()
            .and_then(|s| s.trim_matches(['[', ']']).parse::<std::net::IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if base.scheme() != "https" && !(base.scheme() == "http" && loopback) {
            return Err(Failure::local(
                "endpoint requires HTTPS or numeric loopback HTTP",
            ));
        }
        if !base.path().ends_with('/') {
            let path = format!("{}/", base.path());
            base.set_path(&path);
        }
        let auth = cli
            .auth_file
            .as_ref()
            .map(|path| {
                let mut bytes = input::bytes(
                    File::open(path)
                        .map_err(|_| Failure::local("authorization file could not be opened"))?,
                    input::AUTH_LIMIT,
                )?;
                if bytes.ends_with(b"\n") {
                    bytes.pop();
                    if bytes.ends_with(b"\r") {
                        bytes.pop();
                    }
                }
                if bytes.is_empty() || bytes.iter().any(|b| matches!(b, b'\r' | b'\n')) {
                    return Err(Failure::local("invalid authorization file"));
                }
                let mut header = HeaderValue::from_bytes(&bytes)
                    .map_err(|_| Failure::local("invalid authorization file"))?;
                header.set_sensitive(true);
                Ok(header)
            })
            .transpose()?;
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(cli.connect_timeout))
            .build()
            .map_err(|_| Failure::local("HTTP client could not be initialized"))?;
        Ok(Self {
            http,
            base,
            auth,
            request: Duration::from_secs(cli.request_timeout),
            idle: Duration::from_secs(cli.idle_timeout),
        })
    }
    pub async fn run(
        &self,
        request: Request,
        format: Format,
        submitted: Arc<AtomicBool>,
    ) -> Result<(), Failure> {
        let url = self
            .base
            .join(request.route.trim_start_matches('/'))
            .map_err(|_| Failure::local("invalid endpoint path"))?;
        let body =
            serde_json::to_vec(&request.body).map_err(|_| Failure::local("invalid request"))?;
        let mut pending = self
            .http
            .post(url)
            .header("content-type", "application/json")
            .body(body);
        if let Some(auth) = &self.auth {
            pending = pending.header(AUTHORIZATION, auth);
        }
        submitted.store(request.mutation, Ordering::SeqCst);
        let response = tokio::time::timeout(self.request, pending.send())
            .await
            .map_err(|_| failure(request.mutation, "request timed out"))?
            .map_err(|_| failure(request.mutation, "HTTP transport failed"))?;
        if request.streaming && response.status() == reqwest::StatusCode::OK {
            let media = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim();
            if media != "text/event-stream" {
                return Err(Failure::transport("expected an SSE response"));
            }
            return self.stream(response, &request, format).await;
        }
        tokio::time::timeout(self.request, async {
            let status = response.status().as_u16();
            let bytes = bounded(response)
                .await
                .map_err(|_| failure(request.mutation, "invalid or oversized HTTP response"))?;
            let mut value: Value = json::parse(&bytes)
                .map_err(|_| failure(request.mutation, "invalid JSON response"))?;
            if status != 200 {
                return Err(remote(
                    value.get("error").and_then(Value::as_str),
                    Some(status),
                    request.mutation,
                ));
            }
            if !valid(&request, &value, prior(&request).as_ref()) {
                return Err(failure(request.mutation, "invalid response envelope"));
            }
            if let Some(kind) = &request.kind_filter {
                value = value["resources"]
                    .as_array()
                    .and_then(|rows| {
                        rows.iter()
                            .find(|r| r["kind"].as_str() == Some(kind.as_str()))
                    })
                    .cloned()
                    .ok_or(Failure {
                        code: 3,
                        message: "resource metadata is unavailable",
                    })?;
            }
            submitted.store(false, Ordering::SeqCst);
            output::emit(&value, format)?;
            Ok(())
        })
        .await
        .map_err(|_| failure(request.mutation, "response timed out"))?
    }
    async fn stream(
        &self,
        mut response: reqwest::Response,
        request: &Request,
        format: Format,
    ) -> Result<(), Failure> {
        let mut parser = sse::Parser::default();
        let mut after = prior(request);
        loop {
            let chunk = tokio::time::timeout(self.idle, response.chunk())
                .await
                .map_err(|_| Failure::transport("stream inactivity timeout"))?
                .map_err(|_| Failure::transport("stream transport failed"))?
                .ok_or_else(|| Failure::transport("stream ended; resume explicitly"))?;
            for byte in chunk {
                if let Some(frame) = parser.byte(byte)? {
                    match frame.event.as_str() {
                        "error" => return Err(remote(Some(&frame.data), None, false)),
                        "data" | "" => {
                            let value: Value = json::parse(frame.data.as_bytes())
                                .map_err(|_| Failure::transport("invalid stream JSON"))?;
                            if !valid(request, &value, after.as_ref()) {
                                return Err(Failure::transport("invalid stream envelope"));
                            }
                            if !output::emit(&value, format)? {
                                return Ok(());
                            }
                            if request.route == "/subscribe" {
                                after = serde_json::from_value(value["cursor"].clone()).ok();
                            }
                        }
                        _ => return Err(Failure::transport("unsupported stream event")),
                    }
                }
            }
        }
    }
}
fn failure(mutation: bool, message: &'static str) -> Failure {
    if mutation {
        Failure::uncertain()
    } else {
        Failure::transport(message)
    }
}
async fn bounded(mut response: reqwest::Response) -> Result<Vec<u8>, ()> {
    if response
        .content_length()
        .is_some_and(|n| n > sse::FRAME_LIMIT as u64)
    {
        return Err(());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
        if chunk.len() > sse::FRAME_LIMIT - bytes.len() {
            return Err(());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
fn remote(category: Option<&str>, status: Option<u16>, mutation: bool) -> Failure {
    let (expected, message) = match category {
        Some("invalid") => (400, "invalid request"),
        Some("missing") => (404, "resource is unavailable"),
        Some("conflict") => (409, "revision conflict"),
        Some("identity_mismatch") => (
            409,
            "idempotency identity does not match the original input",
        ),
        Some("identity_expired") => (
            410,
            "retry identity expired; do not retry the old operation with a new epoch",
        ),
        Some("too_large") => (413, "server size limit exceeded"),
        Some("not_committed") => (503, "server category: not_committed"),
        Some("history_gap") => (
            410,
            "history gap; recover explicitly without skipping missing history",
        ),
        Some("denied") => (403, "access denied"),
        Some("overloaded") => (429, "server overloaded"),
        Some("closed") => (503, "server closed"),
        Some("outcome_unknown") => (503, "server reports unresolved outcome"),
        Some("internal") => (500, "server failure"),
        _ => return failure(mutation, "unrecognized remote failure"),
    };
    if status.is_some_and(|s| s != expected) {
        return failure(mutation, "ambiguous remote failure");
    }
    // HTTP error categories do not carry an execution phase. ActorGate and
    // disclosure failures can return any category after a durable commit.
    if mutation {
        return Failure { code: 5, message };
    }
    Failure {
        code: if category == Some("history_gap") {
            6
        } else {
            3
        },
        message,
    }
}
