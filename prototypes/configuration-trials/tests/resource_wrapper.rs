//! Every guarantee here is ROM-owned prototype logic, not a crate feature.
use rom_disposable_configuration_trials::*;
use serde_json::json;
fn each(mut test: impl FnMut(Runtime)) {
    for loader in [Loader::Config, Loader::Figment] {
        test(fixture(loader));
    }
}
fn boot(mut r: Runtime) -> Runtime {
    r.reload("bootstrap", Ok(seed())).unwrap();
    r
}
fn app() -> Id {
    Id::new("AppSettings", "app")
}
#[test]
fn three_kinds_use_same_registry_and_reads_preserve_literal_ids() {
    each(|r| {
        let r = boot(r);
        assert_eq!(r.resources.len(), 3);
        assert_eq!(
            r.read(&Id::new("User", "alice.example[0]"), 0)
                .unwrap()
                .fields["name"],
            "Alice"
        );
        assert_eq!(r.read(&app(), 0).unwrap().revision, 1);
    });
}
#[test]
fn invalid_startup_does_not_start_service_and_bootstrap_is_one_shot() {
    each(|mut r| {
        assert_eq!(r.read(&app(), 0), Err("not-ready"));
        assert_eq!(
            r.begin("unauthenticated").unwrap_err(),
            "unauthorized-source"
        );
        assert_eq!(
            r.reload("bootstrap", Ok(Snapshot::default())),
            Err("missing-trust-resource")
        );
        assert_eq!(r.generation, 0);
        r.reload("bootstrap", Ok(seed())).unwrap();
        assert_eq!(r.reload("bootstrap", Ok(seed())), Err("bootstrap-ended"));
    });
}
#[test]
fn overlay_removal_reveals_inherited_value_without_materializing_defaults() {
    each(|r| {
        let mut r = boot(r);
        r.reload(
            "deployment",
            Ok(Snapshot::one(app(), json!({"title":"file"}))),
        )
        .unwrap();
        r.reload("studio", Ok(Snapshot::one(app(), json!({"title":"edit"}))))
            .unwrap();
        assert_eq!(r.provenance(&app(), "title").unwrap().0, "studio");
        assert_eq!(r.provenance(&app(), "note").unwrap().0, "bootstrap");
        r.reload(
            "deployment",
            Ok(Snapshot::one(app(), json!({"title":"new-file"}))),
        )
        .unwrap();
        assert_eq!(r.resources[&app()].fields["title"], "edit");
        r.reload("studio", Ok(Snapshot::default())).unwrap();
        assert_eq!(r.resources[&app()].fields["title"], "new-file");
    });
}
#[test]
fn missing_null_array_and_tombstone_are_different_operations() {
    each(|r| {
        let mut r = boot(r);
        r.reload("deployment", Ok(Snapshot::one(app(), json!({"note":null}))))
            .unwrap();
        assert!(r.resources[&app()].fields["note"].is_null());
        assert_eq!(r.resources[&app()].fields["title"], "base");
        r.reload(
            "deployment",
            Ok(Snapshot::one(
                Id::new("IdentityProvider", "auth"),
                json!({"audiences":["new"]}),
            )),
        )
        .unwrap();
        assert_eq!(
            r.resources[&Id::new("IdentityProvider", "auth")].fields["audiences"],
            json!(["new"])
        );
        let owned = Id::new("User", "external");
        r.reload(
            "deployment",
            Ok(Snapshot::one(
                owned.clone(),
                json!({"name":"External","enabled":true}),
            )),
        )
        .unwrap();
        let tombstone = Snapshot(std::collections::BTreeMap::from([(
            owned.clone(),
            Input::Delete,
        )]));
        r.reload("deployment", Ok(tombstone)).unwrap();
        assert!(!r.resources.contains_key(&owned));
        assert!(
            r.resources
                .contains_key(&Id::new("User", "alice.example[0]"))
        );
    });
}
#[test]
fn deletion_cannot_erase_other_owners_and_unavailable_source_is_not_empty() {
    each(|r| {
        let mut r = boot(r);
        r.reload(
            "studio",
            Ok(Snapshot::one(
                Id::new("User", "local"),
                json!({"name":"Local","enabled":true}),
            )),
        )
        .unwrap();
        let before = r.resources.clone();
        let generation = r.generation;
        assert_eq!(
            r.reload(
                "deployment",
                Ok(Snapshot(std::collections::BTreeMap::from([(
                    Id::new("User", "local"),
                    Input::Delete
                )])))
            ),
            Err("ownership-conflict")
        );
        assert_eq!(
            r.reload("deployment", Err("fetch-unavailable")),
            Err("fetch-unavailable")
        );
        assert_eq!(r.resources, before);
        assert_eq!(r.generation, generation);
    });
}
#[test]
fn authorization_and_resource_validation_cannot_be_overridden_by_priority() {
    each(|r| {
        let mut r = boot(r);
        assert_eq!(
            r.reload(
                "project",
                Ok(Snapshot::one(
                    Id::new("IdentityProvider", "evil"),
                    json!({"issuer":"https://evil","audiences":["rom"]})
                ))
            ),
            Err("forbidden-kind")
        );
        assert_eq!(
            r.reload(
                "project",
                Ok(Snapshot::one(app(), json!({"title":"overwrite"})))
            ),
            Err("ownership-conflict")
        );
        for (id, fields, expected) in [
            (app(), json!({"title":null}), "invalid-field"),
            (app(), json!({"unknown":true}), "unknown-field"),
            (Id::new("InjectedSchema", "x"), json!({}), "unknown-kind"),
            (
                Id::new("User", "x"),
                json!({"name":"X"}),
                "missing-required-field",
            ),
            (
                Id::new("IdentityProvider", "x"),
                json!({"issuer":"http://bad","audiences":["rom"]}),
                "invalid-issuer",
            ),
            (
                Id::new("IdentityProvider", "x"),
                json!({"issuer":"https://ok","audiences":[]}),
                "empty-audiences",
            ),
            (
                Id::new("IdentityProvider", "x"),
                json!({"secret_ref":"plaintext-sensitive-marker"}),
                "secret-reference-required",
            ),
        ] {
            let before = r.resources.clone();
            assert_eq!(
                r.reload("studio", Ok(Snapshot::one(id, fields))),
                Err(expected)
            );
            assert_eq!(r.resources, before);
        }
    });
}
#[test]
fn invalid_shadowed_source_is_rejected_before_overlay() {
    each(|r| {
        let mut r = boot(r);
        r.reload(
            "studio",
            Ok(Snapshot::one(app(), json!({"title":"valid-winner"}))),
        )
        .unwrap();
        assert_eq!(
            r.reload("deployment", Ok(Snapshot::one(app(), json!({"title":123})))),
            Err("invalid-field")
        );
        assert_eq!(r.resources[&app()].fields["title"], "valid-winner");
    });
}
#[test]
fn failed_reload_is_atomic_and_last_good_state_expires() {
    each(|r| {
        let mut r = boot(r);
        let before = r.resources.clone();
        let generation = r.generation;
        let mut bad = seed();
        bad.0
            .insert(app(), Input::Values(json!({"title":"changed"})));
        bad.0.insert(
            Id::new("User", "broken"),
            Input::Values(json!({"enabled":"sensitive-marker"})),
        );
        assert_eq!(r.reload("deployment", Ok(bad)), Err("invalid-field"));
        assert_eq!(r.status, "rejected");
        assert_eq!(r.generation, generation);
        assert_eq!(r.resources, before);
        assert!(r.read(&app(), 99).is_ok());
        assert_eq!(r.read(&app(), 100), Err("expired-active-state"));
        assert_eq!(
            r.reload("deployment", Snapshot::decode("invalid-sensitive-marker")),
            Err("parse")
        );
    });
}
#[test]
fn no_op_replay_retains_resource_revision_and_generation() {
    each(|r| {
        let mut r = boot(r);
        let snapshot = Snapshot::one(app(), json!({"title":"new"}));
        r.reload("deployment", Ok(snapshot.clone())).unwrap();
        let generation = r.generation;
        let revision = r.resources[&app()].revision;
        r.reload("deployment", Ok(snapshot)).unwrap();
        assert_eq!(r.generation, generation);
        assert_eq!(r.resources[&app()].revision, revision);
        assert_eq!(r.provenance(&app(), "title"), Some(("deployment", 2)));
    });
}
#[test]
fn out_of_order_actual_remote_threads_cannot_publish_old_completion() {
    each(|r| {
        let mut r = boot(r);
        let old = r.begin("deployment").unwrap();
        let new = r.begin("deployment").unwrap();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (tx, rx) = std::sync::mpsc::channel();
        let old_tx = tx.clone();
        let old_thread = std::thread::spawn(move || {
            release_rx.recv().unwrap();
            old_tx
                .send((old, Snapshot::one(app(), json!({"title":"old-remote"}))))
                .unwrap();
        });
        let new_thread = std::thread::spawn(move || {
            tx.send((new, Snapshot::one(app(), json!({"title":"new-remote"}))))
                .unwrap();
        });
        let (ticket, snapshot) = rx.recv().unwrap();
        let candidate = r.prepare(ticket, snapshot).unwrap();
        r.publish(candidate).unwrap();
        release_tx.send(()).unwrap();
        let (ticket, snapshot) = rx.recv().unwrap();
        let candidate = r.prepare(ticket, snapshot).unwrap();
        assert_eq!(r.publish(candidate), Err("late-completion"));
        assert_eq!(r.resources[&app()].fields["title"], "new-remote");
        old_thread.join().unwrap();
        new_thread.join().unwrap();
    });
}
#[test]
fn new_failed_request_still_invalidates_older_completion() {
    each(|r| {
        let mut r = boot(r);
        let old = r.begin("deployment").unwrap();
        let old = r
            .prepare(old, Snapshot::one(app(), json!({"title":"old"})))
            .unwrap();
        assert_eq!(
            r.reload("deployment", Err("fetch-unavailable")),
            Err("fetch-unavailable")
        );
        assert_eq!(r.publish(old), Err("late-completion"));
        assert_eq!(r.resources[&app()].fields["title"], "base");
    });
}
#[test]
fn concurrent_studio_edit_conflicts_with_prepared_reload_then_rebase_preserves_overlay() {
    each(|r| {
        let mut r = boot(r);
        let ticket = r.begin("deployment").unwrap();
        let snapshot = Snapshot::one(app(), json!({"title":"remote"}));
        let candidate = r.prepare(ticket, snapshot.clone()).unwrap();
        r.reload(
            "studio",
            Ok(Snapshot::one(app(), json!({"title":"studio"}))),
        )
        .unwrap();
        assert_eq!(r.publish(candidate), Err("revision-conflict"));
        r.reload("deployment", Ok(snapshot)).unwrap();
        assert_eq!(r.resources[&app()].fields["title"], "studio");
    });
}
#[test]
fn import_envelope_rejects_duplicate_identity_unknown_schema_and_ambiguous_operations() {
    for (input, error) in [
        (
            r#"[{"kind":"User","id":"x","values":{},"delete":true}]"#,
            "ambiguous-operation",
        ),
        (
            r#"[{"kind":"User","id":"x","schema":{}}]"#,
            "unknown-envelope-field",
        ),
        (
            r#"[{"kind":"User","id":"x","values":{}},{"kind":"User","id":"x","values":{}}]"#,
            "duplicate-id",
        ),
        (
            r#"[{"kind":"User","id":"x","delete":null}]"#,
            "ambiguous-operation",
        ),
    ] {
        assert_eq!(Snapshot::decode(input), Err(error));
    }
}
#[test]
fn equal_priority_sources_are_rejected_instead_of_map_order_tiebreak() {
    let grant = Grant {
        kinds: vec!["User".into()],
        priority: 1,
        trust_admin: false,
        overlay: false,
    };
    assert_eq!(
        Runtime::new(
            Loader::Config,
            std::collections::BTreeMap::from([("a".into(), grant.clone()), ("b".into(), grant)])
        )
        .unwrap_err(),
        "duplicate-priority"
    );
}

#[test]
fn environment_is_allowlisted_collision_checked_and_uses_declared_string_codec() {
    assert_eq!(
        environment_snapshot(&[("ROM_APP_TITLE", "1"), ("rom_app_title", "2")]),
        Err("environment-alias-collision")
    );
    assert_eq!(
        environment_snapshot(&[("ROM_APP_ISSUER", "https://evil")]),
        Err("environment-not-allowlisted")
    );
    each(|r| {
        let mut r = boot(r);
        let captured = environment_snapshot(&[
            ("OTHER_SECRET", "never-imported"),
            ("ROM_APP_TITLE", "false"),
            ("ROM_APP_NOTE", ""),
        ])
        .unwrap();
        r.reload("deployment", Ok(captured)).unwrap();
        assert_eq!(
            r.resources[&app()].fields,
            json!({"title":"false", "note":""})
        );
    });
}
