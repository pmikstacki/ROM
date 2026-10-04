//! Trusted HTTPS demo profile. Credentials stay in a private external file.
use crate::smoke::SmokeResult;
use rom_studio_host::{OidcProviderConfig, TrustedLoopbackBackchannel};
use std::{io::Read, os::unix::fs::MetadataExt, path::Path};

pub struct TrustedProfile {
    pub origin: String,
    pub provider: OidcProviderConfig,
    pub backchannel: Option<TrustedLoopbackBackchannel>,
}
fn text(value: &serde_json::Value, key: &str) -> SmokeResult<String> {
    let text = value[key]
        .as_str()
        .ok_or("missing trusted Studio profile field")?;
    if text.is_empty() || text.len() > 4096 {
        return Err("invalid trusted Studio profile field".into());
    }
    Ok(text.into())
}
fn bounded_file(path: &Path, max: u64, private: bool) -> SmokeResult<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !path.is_absolute() || !metadata.is_file() || metadata.len() > max {
        return Err("profile requires an absolute bounded regular file".into());
    }
    if private
        && (metadata.mode() & 0o077 != 0
            || metadata.uid() != std::fs::metadata("/proc/self")?.uid())
    {
        return Err("credential file must be private and owned by the process owner".into());
    }
    let opened = std::fs::File::open(path)?;
    let actual = opened.metadata()?;
    if !actual.is_file()
        || actual.len() > max
        || (private && (actual.mode() & 0o077 != 0 || actual.uid() != metadata.uid()))
    {
        return Err("profile file changed during admission".into());
    }
    let mut bytes = Vec::new();
    opened.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err("profile file exceeds limit".into());
    }
    String::from_utf8(bytes).map_err(|_| "profile file must use UTF-8".into())
}
impl TrustedProfile {
    pub fn read(path: &Path) -> SmokeResult<Self> {
        let config: serde_json::Value = serde_json::from_str(&bounded_file(path, 16384, false)?)?;
        let object = config.as_object().ok_or("profile must be an object")?;
        let fields = [
            "public_origin",
            "issuer",
            "authorization_endpoint",
            "token_endpoint",
            "jwks_endpoint",
            "client_secret_file",
            "backchannel_token_endpoint",
            "backchannel_jwks_endpoint",
        ];
        if object.keys().any(|key| !fields.contains(&key.as_str())) {
            return Err("unknown trusted Studio profile field".into());
        }
        let origin = text(&config, "public_origin")?;
        let issuer = text(&config, "issuer")?;
        // HostConfig validates exact origins and endpoint binding. This mode never enables HTTP publicly.
        if !origin.starts_with("https://") || !issuer.starts_with("https://") {
            return Err("trusted Studio profile requires public HTTPS".into());
        }
        let secret_path = text(&config, "client_secret_file")?;
        let secret = bounded_file(Path::new(&secret_path), 4096, true)?;
        let secret = secret.strip_suffix('\n').unwrap_or(&secret).to_owned();
        if secret.is_empty() {
            return Err("empty Studio credential file".into());
        }
        let backchannel = match (
            config.get("backchannel_token_endpoint"),
            config.get("backchannel_jwks_endpoint"),
        ) {
            (None, None) => None,
            (Some(_), Some(_)) => Some(TrustedLoopbackBackchannel::new(
                &issuer,
                &text(&config, "backchannel_token_endpoint")?,
                &text(&config, "backchannel_jwks_endpoint")?,
            )),
            _ => return Err("both trusted backchannel endpoints are required".into()),
        };
        Ok(Self {
            origin,
            provider: OidcProviderConfig {
                authority: "local".into(),
                label: "Local demo provider".into(),
                issuer,
                client_id: "studio".into(),
                authorization_endpoint: text(&config, "authorization_endpoint")?,
                token_endpoint: text(&config, "token_endpoint")?,
                jwks_endpoint: text(&config, "jwks_endpoint")?,
                client_secret: Some(secret),
            },
            backchannel,
        })
    }
}
