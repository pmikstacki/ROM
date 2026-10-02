use notification_core::{Channels, Delivery, Dispatcher, Outcome};
use notification_sqlite_probe::SqliteOutbox;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct AccountNotice {
    account: String,
}

// An ordinary application function. Replace its body with a real integration later.
// This example records acceptance only in its local database; it sends nothing.
async fn custom_send(delivery: Delivery<AccountNotice>) -> Outcome {
    Outcome::Accepted(format!(
        "local:{}:{}",
        delivery.id.0, delivery.payload.account
    ))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    let mut app = Channels::default();
    app.register("account-updates", "account-notice/v1", custom_send)?;
    let intent = app.intent(
        "example-delivery",
        "account-updates",
        "account-notice/v1",
        &AccountNotice {
            account: "account-1".into(),
        },
    )?;
    let mut scratch = SqliteOutbox::open(":memory:")?;
    scratch.transition("account-1", "updated", &[intent])?;
    let worker = Dispatcher {
        channels: app,
        max_attempts: 3,
        base_backoff: 10,
        lease: 10,
        timeout: Duration::from_secs(1),
    };
    worker.step(&mut scratch, 0).await?;
    let status = scratch
        .status("example-delivery")?
        .ok_or("missing delivery")?;
    assert_eq!(status.state, "accepted");
    println!("Local custom function accepted the committed notification; no external send.");
    Ok(())
}
