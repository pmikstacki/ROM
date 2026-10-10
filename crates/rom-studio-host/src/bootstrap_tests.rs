use crate::{BrowserStore, StudioBootstrap, assets::Assets};
use serde_json::{Value, json};

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../studio/tests/fixtures/studio-bootstrap.json"
    ))
    .unwrap()
}

#[test]
fn bootstrap_matches_the_frontend_wire_contract_without_numeric_epoch_loss() {
    let profile = StudioBootstrap::new(
        "deployment-authority",
        "application-one",
        u64::MAX,
        16384,
        BrowserStore::new("explicit-intents", 20480, 64, 2000).unwrap(),
        BrowserStore::new("explicit-editors", 20480, 64, 2000).unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&profile.json().unwrap()).unwrap(),
        golden()
    );
}

#[test]
fn bootstrap_denies_unknown_fields_and_invalid_store_or_authority_bounds() {
    for mutate in [
        |v: &mut Value| v["authority"] = json!(""),
        |v: &mut Value| v["recovery"]["retryEpoch"] = json!("01"),
        |v: &mut Value| v["recovery"]["retryEpoch"] = json!("18446744073709551616"),
        |v: &mut Value| v["recovery"]["intentStore"]["maxBytes"] = json!(16384),
        |v: &mut Value| v["recovery"]["editorStore"]["timeoutMs"] = json!(60001),
        |v: &mut Value| v["csrf"] = json!("not-host-configuration"),
    ] {
        let mut value = golden();
        mutate(&mut value);
        assert!(
            serde_json::from_value::<StudioBootstrap>(value)
                .and_then(|profile| { profile.json().map_err(serde::de::Error::custom) })
                .is_err()
        );
    }
}

#[test]
fn html_bootstrap_is_bounded_escaped_and_shared_by_navigation_fallback() {
    let directory = std::env::temp_dir().join(format!(
        "rom-bootstrap-{}-{}",
        std::process::id(),
        crate::session::secret().unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(
        directory.join("index.html"),
        "<html><body><div id=app></div></body></html>",
    )
    .unwrap();
    let mut assets = Assets::load(&directory, 16, 100000).unwrap();
    let mut value = golden();
    value["authority"] = json!("</script><script>alert(1)</script>");
    let profile: StudioBootstrap = serde_json::from_value(value).unwrap();
    assets.install_bootstrap(&profile, 100000).unwrap();
    let bytes = assets.get("index.html").unwrap().bytes;
    let html = std::str::from_utf8(&bytes).unwrap();
    assert!(html.contains("type=\"application/json\" id=\"rom-studio-auth-profile\""));
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("\\u003c/script\\u003e"));
    let script = html
        .split("id=\"rom-studio-auth-profile\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    let decoded: serde_json::Value = serde_json::from_str(script).unwrap();
    assert_eq!(
        decoded,
        serde_json::from_str::<serde_json::Value>(&profile.json().unwrap()).unwrap()
    );
    assert_eq!(
        bytes.as_ref(),
        assets.get("resources/tasks").unwrap().bytes.as_ref()
    );
    assert!(assets.install_bootstrap(&profile, 100000).is_err());
    let mut bounded = Assets::load(&directory, 16, 100000).unwrap();
    let original = bounded.get("index.html").unwrap().bytes;
    assert!(bounded.install_bootstrap(&profile, 50).is_err());
    assert_eq!(
        original.as_ref(),
        bounded.get("index.html").unwrap().bytes.as_ref()
    );
    std::fs::remove_dir_all(directory).unwrap();
}
