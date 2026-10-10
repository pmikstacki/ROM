use serde::Deserialize;
use std::{io::Read, path::PathBuf};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub mode: String,
    pub adapter: String,
    pub directory: PathBuf,
    pub source_directory: PathBuf,
    pub backup_directory: PathBuf,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub verified_synthetic_subject: String,
    pub stop_file: PathBuf,
}
pub fn read() -> Result<Configuration, Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("configuration required")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65_537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err("configuration bound".into());
    }
    let value: Configuration = serde_json::from_slice(&bytes)?;
    if !["prepare", "post-snapshot", "restore", "serve"].contains(&value.mode.as_str())
        || !["sqlite", "redb"].contains(&value.adapter.as_str())
        || value.issuer != "https://127.0.0.1:44392/application/o/rom-synthetic-identity/"
    {
        return Err("closed fixture configuration required".into());
    }
    for path in [
        &value.directory,
        &value.source_directory,
        &value.backup_directory,
        &value.stop_file,
    ] {
        validate_private_path(path)?;
    }
    Ok(value)
}

fn validate_private_path(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let relative = path.strip_prefix("/var/tmp/rom-010-authentik-20261007/run/volume/private")?;
    let mut parts = relative.components();
    let Some(std::path::Component::Normal(root)) = parts.next() else {
        return Err("private nonce required".into());
    };
    let nonce = root
        .to_str()
        .and_then(|value| value.strip_prefix("application-recovery-"))
        .ok_or("recovery nonce required")?;
    if nonce.len() != 24
        || !nonce
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("recovery nonce invalid".into());
    }
    let Some(std::path::Component::Normal(name)) = parts.next() else {
        return Err("recovery endpoint required".into());
    };
    if ![
        "source",
        "destination",
        "backup",
        "source-stop",
        "destination-stop",
    ]
    .iter()
    .any(|allowed| name == *allowed)
        || parts.next().is_some()
    {
        return Err("closed recovery endpoint required".into());
    }
    Ok(())
}
#[cfg(test)]
#[path = "configuration_tests.rs"]
mod tests;
