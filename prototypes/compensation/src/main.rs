#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("crash-child") {
        assert_eq!(args.len(), 3);
        rom_compensation_probe::crash_child(std::path::Path::new(&args[1]), &args[2]).await;
        return;
    }
    assert!(args.is_empty(), "usage: rom-compensation-probe");
    let mut total = 0;
    for backend in ["sqlite", "redb"] {
        for result in rom_compensation_probe::run_backend(backend).await {
            println!("{result}");
            total += 1;
        }
    }
    for result in rom_compensation_probe::run_crash_cases(&std::env::current_exe().unwrap()).await {
        println!("{result}");
        total += 1;
    }
    eprintln!("{total} compensation cases passed (external outcomes simulated)");
}
