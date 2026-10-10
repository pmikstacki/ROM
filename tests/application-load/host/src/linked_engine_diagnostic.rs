//! Offline additive CLI diagnostic. No configuration, persisted store or Host.
use serde_json::json;
use std::io::Write;

pub(crate) const FLAG: &str = "--sqlite-engine-diagnostic";
const EXPECT: &str = "--expect-native-3534";
const SOURCE_ID: &str =
    "2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc";
const REQUIRED: [&str; 3] = ["THREADSAFE=1", "ENABLE_COLUMN_METADATA", "USE_URI"];
const MAXIMUM_BYTES: usize = 128 * 1024;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn arguments(args: &[String]) -> Result<bool> {
    match args {
        [flag] if flag == FLAG => Ok(false),
        [flag, expected] if flag == FLAG && expected == EXPECT => Ok(true),
        _ => Err("closed offline SQLite diagnostic arguments".into()),
    }
}
fn encode(identity: rom_sqlite::LinkedEngineIdentity, expected: bool) -> Result<(Vec<u8>, bool)> {
    identity.validate()?;
    let matched = !expected
        || (identity.version == "3.53.4"
            && identity.source_id == SOURCE_ID
            && REQUIRED.iter().all(|required| {
                identity
                    .compile_options
                    .iter()
                    .any(|option| option.as_str() == *required)
            }));
    let value = json!({
        "schema":"rom-load-host-linked-sqlite-v1", "mode":"offline-linked-engine",
        "engine":{"version":identity.version,"source_id":identity.source_id,"compile_options":identity.compile_options},
        "expected_profile":if expected {json!({"version":"3.53.4","source_id":SOURCE_ID,"required_options":REQUIRED})} else {serde_json::Value::Null},
        "profile_matches":if expected {json!(matched)} else {serde_json::Value::Null},
        "diagnostic_only":true,"acceptance":false,"persisted_database":false,"remote_endpoint":false,
    });
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    if bytes.len() > MAXIMUM_BYTES {
        return Err("offline SQLite diagnostic output bound".into());
    }
    Ok((bytes, matched))
}
pub(crate) fn run(args: &[String]) -> Result<()> {
    let expected = arguments(args)?;
    let identity = rom_sqlite::Sqlite::linked_engine_identity()?;
    let (bytes, matched) = encode(identity, expected)?;
    std::io::stdout().lock().write_all(&bytes)?;
    if !matched {
        return Err("linked SQLite does not match selected native profile".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
