use rom::{Resource, Shape, json};
use rom_maintenance_portal::{Inspection, PortalSettings};

#[test]
fn inspection_reference_exposes_the_registered_target_kind() {
    let descriptor = Inspection::descriptor();
    let equipment = descriptor
        .fields
        .iter()
        .find(|field| field.name == "equipment")
        .unwrap();
    assert_eq!(
        equipment.shape,
        Shape::Reference {
            kind: "equipment".into()
        }
    );
}

#[test]
fn settings_accept_full_layout_but_reject_corrupt_layouts() {
    let settings = |layout: &str| {
        json!({
            "owner":"alice", "show_history":true,"columns":4,"layout":layout,
        })
    };
    let layout = r#"[{"id":"history","x":0,"y":0,"width":2,"height":1,"visible":true}]"#;
    let accepted = PortalSettings::decode(settings(layout));
    assert!(accepted.is_ok(), "full layout must be accepted");
    assert_eq!(accepted.unwrap().encode()["layout"], layout);
    for invalid in [
        r#"[{"id":"unknown","x":0,"y":0,"width":2,"height":1,"visible":true}]"#,
        r#"[{"id":"history","x":4,"y":0,"width":1,"height":1,"visible":true}]"#,
        r#"[{"id":"history","x":0,"y":0,"width":2,"height":1,"visible":1}]"#,
        r#"[{"id":"history","x":0,"y":0,"width":2,"height":1,"visible":true,"extra":1}]"#,
        r#"[{"id":"history","x":0,"y":0,"width":2,"height":1,"visible":true},{"id":"history","x":0,"y":1,"width":2,"height":1,"visible":true}]"#,
    ] {
        assert!(
            PortalSettings::decode(settings(invalid)).is_err(),
            "accepted {invalid}"
        );
    }
}

#[test]
fn inspection_date_rejects_invalid_input_and_canonicalizes_offsets() {
    let input = |date: &str| {
        json!({
            "owner": "alice", "equipment": "pump-1", "notes": "Checked",
            "inspected_at": date,
        })
    };
    assert!(Inspection::decode(input("yesterday")).is_err());
    let inspection = Inspection::decode(input("2026-10-08T14:00:00+02:00")).unwrap();
    assert_eq!(
        inspection.encode()["inspected_at"],
        json!("2026-10-08T12:00:00.000000000Z")
    );
    let codec = Inspection::field_codecs()
        .into_iter()
        .find(|codec| codec.name == "inspected_at")
        .unwrap();
    assert_eq!(codec.codec.name, "rom.datetime");
    assert_eq!(codec.codec.version, 1);
}
