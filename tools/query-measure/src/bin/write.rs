#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), rom_query_measure::Failure> {
    rom_query_measure::write_run::main().await
}
