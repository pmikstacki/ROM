use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use rom::{Error, Result};
use rom_auth::{jwt::DecodingKey, valid_key_id};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeySet {
    keys: Vec<Jwk>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    usage: Option<String>,
    key_ops: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present")]
    x5c: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present")]
    x5t: Option<String>,
    #[serde(rename = "x5t#S256", default, deserialize_with = "present")]
    x5t_s256: Option<String>,
}
// Missing metadata is permitted; an explicit null is not a typed metadata value.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

impl Jwk {
    // This profile uses n/e exclusively. These fields never establish certificate trust,
    // key equality, certificate validity, or thumbprint equality.
    fn unused_metadata_is_bounded(&self) -> bool {
        if let Some(chain) = &self.x5c {
            if chain.is_empty()
                || chain.len() > 4
                || chain.iter().map(String::len).sum::<usize>() > 16_384
            {
                return false;
            }
            if chain.iter().any(|value| {
                value.is_empty() || value.len() > 8192 || STANDARD.decode(value).is_err()
            }) {
                return false;
            }
        }
        [(&self.x5t, 20), (&self.x5t_s256, 32)]
            .into_iter()
            .all(|(value, size)| {
                value.as_ref().is_none_or(|value| {
                    value.len() <= 43
                        && URL_SAFE_NO_PAD
                            .decode(value)
                            .is_ok_and(|bytes| bytes.len() == size)
                })
            })
    }
}

pub(crate) fn parse_jwks(bytes: &[u8]) -> Result<BTreeMap<String, DecodingKey>> {
    if bytes.len() > 65_536 {
        return Err(Error::TooLarge);
    }
    let set: KeySet = serde_json::from_slice(bytes).map_err(|_| Error::Denied)?;
    if set.keys.is_empty() || set.keys.len() > 8 {
        return Err(Error::Denied);
    }
    let mut keys = BTreeMap::new();
    for jwk in set.keys {
        if !valid_key_id(&jwk.kid)
            || jwk.kty != "RSA"
            || jwk.alg.as_deref().is_some_and(|alg| alg != "RS256")
            || jwk.usage.as_deref().is_some_and(|usage| usage != "sig")
            || jwk
                .key_ops
                .as_ref()
                .is_some_and(|ops| ops.as_slice() != ["verify"])
            || !jwk.unused_metadata_is_bounded()
            || keys.contains_key(&jwk.kid)
        {
            return Err(Error::Denied);
        }
        let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|_| Error::Denied)?;
        keys.insert(jwk.kid, key);
    }
    Ok(keys)
}
