use rom::{Field, json};
use rom_conformance::CodecCase;
use rom_skill_native_example::Code;

#[test]
fn external_native_module_invokes_the_public_codec_profile() {
    rom_conformance::profile::require(1).unwrap();
    rom_conformance::field::codec(
        &[CodecCase {
            input: json!(" xy-2 "),
            canonical: json!("XY-2"),
            expected: Code::decode(json!("XY-2")).unwrap(),
        }],
        &[json!(null), json!(""), json!("invalid/code")],
    )
    .unwrap();
}
