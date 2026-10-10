mod configuration;
#[path = "../../tests/support/credentials.rs"]
mod credentials;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server::run().await
}
