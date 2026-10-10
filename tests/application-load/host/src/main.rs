#[tokio::main(worker_threads = 2)]
async fn main() {
    if let Err(error) = rom_application_load_authoring::runner::run().await {
        eprintln!("load fixture failed: {error}");
        std::process::exit(1);
    }
}
