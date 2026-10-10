//! Run public Runtime commands against the independently verified accepted source.
mod model;
mod population;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    population::run().await;
}
