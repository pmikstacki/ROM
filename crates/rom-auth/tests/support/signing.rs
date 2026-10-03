//! Shared real RSA fixtures; private material is synthetic and never printed.
use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode};
use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::OnceLock,
};

pub struct Keys {
    pub private: Vec<u8>,
    pub public: Vec<u8>,
}
fn generate() -> Keys {
    let output = Command::new("openssl")
        .args([
            "genpkey",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:2048",
        ])
        .stderr(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    let private = output.stdout;
    let mut child = Command::new("openssl")
        .args(["pkey", "-pubout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&private).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    Keys {
        private,
        public: output.stdout,
    }
}
pub fn keys() -> &'static [Keys; 2] {
    static KEYS: OnceLock<[Keys; 2]> = OnceLock::new();
    KEYS.get_or_init(|| [generate(), generate()])
}
pub fn public(index: usize) -> DecodingKey {
    DecodingKey::from_rsa_pem(&keys()[index].public).unwrap()
}
pub fn token(claims: &Value, header: Header, index: usize) -> String {
    encode(
        &header,
        claims,
        &EncodingKey::from_rsa_pem(&keys()[index].private).unwrap(),
    )
    .unwrap()
}
