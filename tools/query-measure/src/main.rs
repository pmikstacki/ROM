#[cfg(feature = "heap")]
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), rom_query_measure::Failure> {
    rom_query_measure::run::main().await
}
