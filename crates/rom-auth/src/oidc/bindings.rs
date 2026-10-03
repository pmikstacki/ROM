/// Original token-endpoint inputs for validating present ID-token hash claims.
///
/// Supply the unmodified access token for `at_hash` and the redeemed authorization
/// code for `c_hash`. A present hash without its original input is rejected. These
/// borrowed credentials are never retained or included in diagnostics.
#[derive(Clone, Copy, Default)]
pub struct OidcTokenBindings<'a> {
    /// Original returned access token, when consumed by this flow.
    pub access_token: Option<&'a str>,
    /// Original redeemed authorization code.
    pub authorization_code: Option<&'a str>,
}
