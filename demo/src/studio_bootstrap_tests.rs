//! Browser composition binds durable state to the demo deployment, not its operator actor.
use crate::studio_bootstrap;

#[test]
fn browser_profile_preserves_current_epoch_and_declares_the_human_authority() {
    for epoch in [0, 1, u64::MAX] {
        let value: serde_json::Value =
            serde_json::from_str(&studio_bootstrap::profile(epoch).unwrap().json().unwrap())
                .unwrap();
        assert_eq!(value["authority"], "local");
        assert_ne!(
            value["authority"],
            crate::studio_application::host_actor().authority
        );
        assert_eq!(value["recovery"]["retryEpoch"], epoch.to_string());
        assert_ne!(
            value["recovery"]["intentStore"]["name"],
            value["recovery"]["editorStore"]["name"]
        );
        assert!(value.get("csrf").is_none());
        assert!(value.get("client_secret").is_none());
    }
}
