//! Fixture-only authorized progress transport; no product HTTP route is added.
mod authority;
mod domain;
mod gate;
mod provider;
mod server;
mod state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server::serve().await
}
