mod control;
#[cfg(test)]
mod control_tests;
mod host;
mod mailbox;
mod protected;
mod provisioning;
#[tokio::main(worker_threads = 2)]
async fn main() {
    if host::run().await.is_err() {
        eprintln!("isolated identity host fixture failed");
        std::process::exit(1);
    }
}
