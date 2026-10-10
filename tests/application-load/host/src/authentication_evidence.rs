//! Bounded authentication attribution after the finite Host lifecycle drains.
pub(crate) fn finish(
    directory: &std::path::Path,
    host: &rom_studio_host::StudioHost,
    result: rom::Result<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let proof = (|| {
        let snapshot = host
            .authentication_diagnostics()?
            .ok_or("load authentication observer disabled")?;
        super::runner::write_bounded(
            directory,
            "load-final-authentication-proof.json",
            &serde_json::json!({
                "schema": "rom-application-load-authentication-v2",
                "scope": "fixed-category Host lifecycle attribution; not per-request evidence",
                "observer_enabled": true,
                "lifecycle_result": if result.is_ok() { "succeeded" } else { "failed" },
                "snapshot": snapshot,
            }),
            256 * 1024,
        )
    })();
    match (result, proof) {
        (Err(error), proof) => {
            if proof.is_err() {
                eprintln!("authentication evidence unavailable");
            }
            Err(error.into())
        }
        (Ok(()), proof) => proof,
    }
}
