use rom_auth::{
    AuthError, OidcIdTokenAdapter,
    jwt::{DecodingKey, TrustedKeys},
    oidc::OidcTokenBindings,
};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    time::{SystemTime, UNIX_EPOCH},
};
struct OriginalKeys(BTreeMap<String, DecodingKey>);
impl TrustedKeys for OriginalKeys {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        Ok(self.0.clone())
    }
}
pub fn run() -> i32 {
    let arguments: Vec<_> = std::env::args().collect();
    let historical = arguments
        .get(3)
        .is_some_and(|value| value == "--historical-callback-time");
    if arguments.len() != 3 && !(arguments.len() == 4 && historical) {
        eprintln!("private flow and public JWKS paths required");
        return 2;
    }
    let verify = || -> Result<(), Box<dyn std::error::Error>> {
        let bounded_read = |path: &str| -> std::io::Result<Vec<u8>> {
            let mut bytes = Vec::new();
            fs::File::open(path)?.take(65537).read_to_end(&mut bytes)?;
            Ok(bytes)
        };
        let flow_bytes = bounded_read(&arguments[1])?;
        let key_bytes = bounded_read(&arguments[2])?;
        if flow_bytes.len() > 65536 || key_bytes.len() > 65536 {
            return Err("bounded input exceeded".into());
        }
        let flow: serde_json::Value = serde_json::from_slice(&flow_bytes)?;
        let jwks: serde_json::Value = serde_json::from_slice(&key_bytes)?;
        let string = |name: &str| flow[name].as_str().ok_or("missing private flow field");
        let mut keys = BTreeMap::new();
        for key in jwks["keys"].as_array().ok_or("missing public key set")? {
            let kid = key["kid"].as_str().ok_or("missing key ID")?;
            let decoding = DecodingKey::from_rsa_components(
                key["n"].as_str().ok_or("missing modulus")?,
                key["e"].as_str().ok_or("missing exponent")?,
            )?;
            if keys.insert(kid.to_owned(), decoding).is_some() || keys.len() > 8 {
                return Err("ambiguous public key set".into());
            }
        }
        let mut adapter = OidcIdTokenAdapter::configured(
            "synthetic-authentik",
            string("issuer")?,
            string("client_id")?,
            OriginalKeys(keys),
        )?;
        // Explicit historical cryptographic reproduction only. This retained host
        // callback-file timestamp is independent of every unverified token claim.
        let trusted_time = if historical {
            fs::metadata(&arguments[1])?.modified()?
        } else {
            SystemTime::now()
        };
        let now = trusted_time.duration_since(UNIX_EPOCH)?.as_secs();
        let result = adapter.authenticate(
            string("id_token")?,
            string("nonce")?,
            OidcTokenBindings {
                access_token: Some(string("access_token")?),
                authorization_code: Some(string("code")?),
            },
            now,
        );
        // Never serialize token, nonce, code, account subject, or private source bodies.
        match result {
            Ok(_) => {
                println!(
                    "{{\"profile\":\"original-authentik-sdk-authoring\",\"historical_replay\":{historical},\"accepted\":true}}"
                );
                Ok(())
            }
            Err(error) => {
                println!(
                    "{{\"profile\":\"original-authentik-sdk-authoring\",\"historical_replay\":{historical},\"accepted\":false,\"error\":\"{error:?}\"}}"
                );
                Err("original provider-issued ID token rejected".into())
            }
        }
    };
    match verify() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}
