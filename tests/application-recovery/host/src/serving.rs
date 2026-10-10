use crate::{application, configuration::Configuration, database::Database, drill};
use rom_studio_host::StudioHost;
use std::time::{Duration, Instant};
pub async fn run(c: Configuration) -> Result<(), Box<dyn std::error::Error>> {
    if std::fs::read(c.source_directory.join("source-only-canary")).is_ok() {
        return Err("source directory available to recovered Host".into());
    }
    let db = Database::open(&c.adapter, &c.directory.join("database"))?;
    let runtime = application::runtime(db.storage())?;
    let blobs = drill::blobs(runtime.clone(), &c.directory.join("objects"))?;
    let config = crate::host_configuration::configured(
        &c.issuer,
        &c.client_id,
        &c.client_secret,
        application::actor(),
        blobs,
    );
    let host = StudioHost::new(runtime, config)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:44391").await?;
    println!("{{\"status\":\"ready\",\"source_canary_unavailable\":true}}");
    host.serve(listener, async move {
        let end = Instant::now() + Duration::from_secs(90);
        while Instant::now() < end && !c.stop_file.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await?;
    Ok(())
}
