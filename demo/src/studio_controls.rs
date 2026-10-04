//! Opt-in private Unix-socket controls remain outside the browser and HTTP host.
use crate::{studio::DemoClock, studio_application::host_actor};
use rom::{Command, Runtime};
use rom_identity::User;
use rom_studio_host::StudioSettings;
use std::{os::unix::fs::PermissionsExt, path::Path, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};

pub fn listener(path: &Path) -> rom::Result<UnixListener> {
    let parent = path.parent().ok_or(rom::Error::Denied)?;
    let metadata = std::fs::symlink_metadata(parent).map_err(|_| rom::Error::Denied)?;
    if !path.is_absolute() || !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(rom::Error::Denied);
    }
    let listener = UnixListener::bind(path).map_err(|_| rom::Error::Storage)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| rom::Error::Storage)?;
    Ok(listener)
}
pub async fn run(
    runtime: Runtime,
    clock: Arc<DemoClock>,
    gate: Arc<crate::studio_blobs::PublicationGate>,
    listener: UnixListener,
) {
    while let Ok((socket, _)) = listener.accept().await {
        let (read, mut write) = socket.into_split();
        let mut reader = BufReader::new(read).take(4097);
        let mut bytes = Vec::new();
        let read = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            reader.read_until(b'\n', &mut bytes),
        )
        .await;
        let result = if !matches!(read,Ok(Ok(n)) if n>0) || bytes.len() > 4096 {
            Err(rom::Error::TooLarge)
        } else {
            match serde_json::from_slice(&bytes) {
                Ok(command) => handle(&runtime, &clock, &gate, &command).await,
                Err(_) => Err(rom::Error::invalid("control", "invalid command")),
            }
        };
        let response = serde_json::to_vec(&match result {
            Ok(value) => serde_json::json!({"control_result":value}),
            Err(error) => serde_json::json!({"control_error":error.to_string()}),
        });
        if let Ok(mut response) = response {
            response.push(b'\n');
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                write.write_all(&response),
            )
            .await;
        }
    }
}
async fn handle(
    runtime: &Runtime,
    clock: &DemoClock,
    gate: &crate::studio_blobs::PublicationGate,
    request: &serde_json::Value,
) -> rom::Result<serde_json::Value> {
    match request["op"].as_str() {
        Some("blob-pause") => {
            gate.pause();
            Ok(serde_json::json!({"paused":true}))
        }
        Some("blob-status") => Ok(serde_json::json!({"entered":gate.entered()})),
        Some("blob-release") => {
            gate.release();
            Ok(serde_json::json!({"released":true}))
        }
        Some("stop") => Err(rom::Error::invalid("control", "send a process signal")),
        Some("advance") => {
            let seconds = request["seconds"]
                .as_u64()
                .filter(|v| *v <= 3600)
                .ok_or(rom::Error::TooLarge)?;
            clock.advance(seconds)?;
            Ok(serde_json::json!({"advanced":seconds}))
        }
        Some("user-enabled") => {
            let id = request["id"]
                .as_str()
                .filter(|id| matches!(*id, "alice-user" | "bob-user"))
                .ok_or(rom::Error::Denied)?;
            let enabled = request["enabled"].as_bool().ok_or(rom::Error::Denied)?;
            let current = runtime.read::<User>(&host_actor(), id).await?;
            let mut value = current.value.ok_or(rom::Error::Missing)?;
            value.enabled = enabled;
            runtime
                .execute(
                    &host_actor(),
                    Command::replace(id, value)
                        .at_revision(current.revision)
                        .idempotency(&format!("fixture-user-{id}-{}", current.revision)),
                )
                .await?;
            Ok(serde_json::json!({"user":id,"enabled":enabled}))
        }
        Some("primary") => {
            let provider = match request.get("provider") {
                Some(serde_json::Value::Null) => None,
                Some(serde_json::Value::String(id)) if id == "local" => Some(id.clone()),
                _ => return Err(rom::Error::Denied),
            };
            let current = runtime
                .read::<StudioSettings>(&host_actor(), "default")
                .await?;
            runtime
                .execute(
                    &host_actor(),
                    Command::replace(
                        "default",
                        StudioSettings {
                            primary_provider: provider.clone(),
                        },
                    )
                    .at_revision(current.revision)
                    .idempotency(&format!("fixture-primary-{}", current.revision)),
                )
                .await?;
            Ok(serde_json::json!({"primary":provider}))
        }
        _ => Err(rom::Error::invalid("control", "unknown command")),
    }
}
