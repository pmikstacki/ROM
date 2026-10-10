//! Public fake token used exclusively by local wire fixtures.
use rom_ai::AiFuture;
use rom_openrouter::{CredentialSource, SecretToken};
pub struct Credentials;
impl CredentialSource for Credentials {
    fn resolve<'a>(&'a self, _: &'a str) -> AiFuture<'a, SecretToken> {
        Box::pin(async { SecretToken::new("fixture-secret") })
    }
}
