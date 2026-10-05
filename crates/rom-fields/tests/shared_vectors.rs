use rom::{Field, Value};
use rom_fields::*;

fn check<F: Field>(catalog: &Value) {
    let identity = F::codec_identity().unwrap();
    assert_eq!(catalog["name"], identity.name);
    assert_eq!(catalog["version"], identity.version);
    for pair in catalog["accepted"].as_array().unwrap() {
        let decoded = F::decode(pair[0].clone()).unwrap();
        assert_eq!(decoded.encode(), pair[1]);
        assert_eq!(F::decode(decoded.encode()).unwrap().encode(), pair[1]);
    }
    for invalid in catalog["rejected"].as_array().unwrap() {
        assert!(
            F::decode(invalid.clone()).is_err(),
            "{} accepted {invalid}",
            identity.name
        );
    }
}
#[test]
fn shared_studio_vectors_match_compiled_backend_codecs() {
    let catalog: Value =
        serde_json::from_str(include_str!("fixtures/semantic-codecs-v1.json")).unwrap();
    let entries = catalog.as_array().unwrap();
    assert_eq!(entries.len(), 10);
    check::<Date>(&entries[0]);
    check::<Time>(&entries[1]);
    check::<DateTime>(&entries[2]);
    check::<Color>(&entries[3]);
    check::<Email>(&entries[4]);
    check::<Url>(&entries[5]);
    check::<Multiline>(&entries[6]);
    check::<JsonDocument>(&entries[7]);
    check::<Decimal>(&entries[8]);
    check::<UnitValue>(&entries[9]);
}
