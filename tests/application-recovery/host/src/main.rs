mod application;
mod configuration;
mod database;
mod drill;
mod fixture_identity;
mod host_configuration;
mod serving;
#[tokio::main(worker_threads = 2)]
async fn main() {
    if let Err(error) = drill::run().await {
        eprintln!("application recovery fixture failed: {error}");
        std::process::exit(1);
    }
}
