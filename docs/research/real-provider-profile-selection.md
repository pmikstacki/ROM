# Select a local provider for the service profile

Date: 2026-10-03.
Status: source-based provider selection. The [separate probe report](real-provider-profile-probe.md) records execution results.

The initial source study proposed version 9.5.1. Its later dependency audit found a router advisory and rejected that pin.
The revised selection uses version 9.12.2. The probe report preserves the rejected candidate and its audit result.

Select **oidc-provider 9.12.2** for the first disposable service introspection fixture.
It is an actual OAuth authorization server library with client credentials and introspection support.
The upstream documentation lists version 9.x as maintained. The selected version is an exact fixture pin, not a claim about the latest release. [Pinned upstream README](https://github.com/panva/node-oidc-provider/blob/v9.12.2/README.md), [9.12.2 release](https://github.com/panva/node-oidc-provider/releases/tag/v9.12.2).

The library has an issuance hook for extra claims. Its token model stores those claims before introspection.
This supports a configured service subject without a response rewrite or a change to ROM's verifier.
This paragraph describes source-based feasibility. The separate probe report establishes the measured compatibility boundary. [Pinned opaque token implementation](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/models/formats/opaque.js), [pinned introspection implementation](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/actions/introspection.js).

This note supports [release task 4.3](../../openspec/changes/prepare-framework-release/tasks.md) and the [release design](../../openspec/changes/prepare-framework-release/design.md).
It selects two candidates only. It changes no product code, dependencies, services or release task state.

## Compare the two candidates

| Item | oidc-provider 9.12.2 | Keycloak 26.4.0 |
| --- | --- | --- |
| Provider form | OAuth server library in a small Node application | Independent server distribution |
| License | MIT | Apache License 2.0 |
| Local prerequisite | Node 22.x LTS or a later LTS release; npm package and lockfile | OpenJDK 21; downloaded distribution; realm configuration |
| Service identity | Issue `sub=client_id` and `principal_kind=service` through `extraTokenClaims` | Replace the default service user's subject with the issuing client ID through a protocol mapper |
| Audience | Configure a resource indicator with audience `rom-api` | Configure an Audience mapper with audience `rom-api` |
| Disposable storage | Default process memory; provider restart loses issued tokens | Development server database in its isolated distribution directory |
| Additional setup here | Download pinned npm dependencies in a separate fixture directory | Obtain a JDK; download the server; import a private realm |
| Decision | Select for the first local profile | Retain as a second option; no compatibility claim |

The Node runtime requirement comes from the pinned entrypoint. The license comes from the pinned package and license text.
Resource audience and opaque format are explicit configuration choices. [Node runtime gate](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/index.js), [package metadata](https://github.com/panva/node-oidc-provider/blob/v9.12.2/package.json), [MIT license](https://github.com/panva/node-oidc-provider/blob/v9.12.2/LICENSE.md), [resource configuration](https://github.com/panva/node-oidc-provider/blob/v9.12.2/docs/README.md#featuresresourceindicators).

Keycloak's pinned guide specifies OpenJDK 21. Its license is Apache License 2.0.
Its normal subject mapper uses the user's internal ID. Its claim helper permits an explicit `sub` value.
Its introspection implementation returns `client_id` from the issuing client, plus the token's type and time claims. [Pinned startup guide](https://github.com/keycloak/keycloak/blob/26.4.0/docs/guides/getting-started/getting-started-zip.adoc), [license](https://github.com/keycloak/keycloak/blob/26.4.0/LICENSE.txt), [subject mapper](https://github.com/keycloak/keycloak/blob/26.4.0/services/src/main/java/org/keycloak/protocol/oidc/mappers/SubMapper.java), [claim helper](https://github.com/keycloak/keycloak/blob/26.4.0/services/src/main/java/org/keycloak/protocol/oidc/mappers/OIDCAttributeMapperHelper.java), [introspection](https://github.com/keycloak/keycloak/blob/26.4.0/services/src/main/java/org/keycloak/protocol/oidc/AccessTokenIntrospectionProvider.java).

The local prerequisite inspection ran inside `rom-dev` through `nixos-container run`.
Node reports `v22.16.0`. `npm`, `nix` and `openssl` exist. `java` is absent.
This observation supports the setup-cost decision. It is not a provider startup or package-install result.
The plan assumes no Docker.

## ROM's unchanged acceptance contract

The [current introspection verifier](../../crates/rom-auth/src/introspection.rs) accepts the following service profile:

| Claim or condition | Exact fixture value or rule |
| --- | --- |
| `active` | `true` |
| `iss` | Exact local issuer `http://127.0.0.1:<allocated-port>` |
| `aud` | String or string array that contains `rom-api` |
| `exp` | Provider Unix expiry greater than trusted host time |
| `iat`, `nbf` | If present, no value greater than trusted host time |
| `principal_kind` | `service` |
| `sub`, `client_id` | Both equal the issuing client ID `rom-service` |
| `token_type` | `Bearer`, with this exact capitalization |
| `cnf` | Absent |

Configure `EndpointPolicy::LoopbackTestOnly` for this numeric loopback HTTP fixture.
Production configuration uses `HttpsOnly`; the local result does not prove TLS deployment.
ROM authenticates its introspection POST with separate client credentials.
It limits the response to 16 KiB and the request to 500 milliseconds.
Its proof and cache validity are at most five seconds, bounded further by token expiry.
These facts come from the local verifier, not from a general OAuth compatibility assumption.

Use provider Resource ID and authority `local-oidc` for the proposed host activation.
Set profile `OAuthIntrospectionService`, encoded as `oauth-introspection-service`.
Set its endpoint to `<issuer>/token/introspection` and its audience to `rom-api`.
Use an approved opaque reference such as `local-oidc-introspection-v1` for `credential_ref`.
The host maps that reference to private material for `rom-introspector`.
The separate issuance secret belongs to the token-acquisition fixture. [Current provider Resource](../../crates/rom-identity/src/resources.rs).

The JWT profile is a separate human profile with an access-token header and RS256 requirements.
This service selection makes no JWT or human login claim. [Current JWT verifier](../../crates/rom-auth/src/jwt.rs).

## Proposed exact provider configuration

Use two static confidential clients. Only `rom-service` can issue the fixture's access tokens.
Only `rom-introspector` can introspect those tokens.
Each client has `response_types: []`, `redirect_uris: []` and `token_endpoint_auth_method: 'client_secret_basic'`.
The issuing client has `grant_types: ['client_credentials']` and scope `rom`.
The introspection client has `grant_types: []`.
The pinned metadata gate admits these declarations without browser redirects. [Client schema](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/helpers/client_schema.js).

The following fragment describes the selected configuration. Execution evidence belongs to the separate probe report.
`serviceSecret` and `introspectionSecret` are separate bounded values from private host files.
`port` is a free numeric loopback port that the fixture allocates.
`fixturePrivateJwk` is a fresh private RSA JWK with `kid`, `use: 'sig'` and `alg: 'RS256'`.
Generate it at fixture startup with Node's `generateKeyPairSync` and JWK export. Do not record the private key. [Node 22.16.0 crypto API](https://nodejs.org/download/release/v22.16.0/docs/api/crypto.html).

```javascript
import { Provider, errors } from 'oidc-provider';

const issuer = `http://127.0.0.1:${port}`;
const resource = 'https://rom.fixture.invalid/api';
const client = (id, secret, grants) => ({
  client_id: id, client_secret: secret, grant_types: grants,
  response_types: [], redirect_uris: [],
  token_endpoint_auth_method: 'client_secret_basic',
});
const provider = new Provider(issuer, {
  scopes: ['rom'],
  ttl: { ClientCredentials: 60 },
  jwks: { keys: [fixturePrivateJwk] },
  clients: [
    { ...client('rom-service', serviceSecret, ['client_credentials']), scope: 'rom' },
    client('rom-introspector', introspectionSecret, []),
  ],
  features: {
    devInteractions: { enabled: false },
    clientCredentials: { enabled: true },
    introspection: {
      enabled: true,
      allowedPolicy: (_ctx, caller, token) =>
        caller.clientId === 'rom-introspector' && token.clientId === 'rom-service',
    },
    resourceIndicators: {
      enabled: true,
      getResourceServerInfo: (_ctx, requested, caller) => {
        if (requested !== resource || caller.clientId !== 'rom-service')
          throw new errors.InvalidTarget();
        return { scope: 'rom', audience: 'rom-api',
          accessTokenTTL: 60, accessTokenFormat: 'opaque' };
      },
    },
  },
  extraTokenClaims: (_ctx, token) => {
    if (token.kind !== 'ClientCredentials' || token.clientId !== 'rom-service')
      throw new errors.InvalidGrant();
    return { sub: token.clientId, principal_kind: 'service' };
  },
});
const server = provider.listen(port, '127.0.0.1');
```

The feature declarations use the pinned configuration contract. Dynamic registration remains disabled.
No middleware changes the token endpoint or introspection response. [Configuration defaults](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/helpers/defaults.js).

The client credentials grant constructs the provider's actual `ClientCredentials` token and applies its resource policy.
The opaque formatter invokes `extraTokenClaims` before it stores the token.
Introspection returns stored extra claims and provider-owned issuer, audience, client ID and time claims.
The token model returns `Bearer` when the token has no DPoP binding. [Grant implementation](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/actions/grants/client_credentials.js), [opaque formatter](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/models/formats/opaque.js), [introspection](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/actions/introspection.js), [token type](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/models/client_credentials.js).

These issuance hooks define provider claims through supported configuration.
They do not repair a failing response at ROM's boundary.
The later probe must pass the original response directly to the unchanged verifier.

## Startup and probe prerequisites

These steps define the isolated probe. The separate report records their executed subset and result.

1. Create a private fixture directory separate from the repository and existing services.
2. Pin `oidc-provider` to `9.12.2` in a fixture package manifest.
3. Generate and retain a lockfile with exact transitive versions and package integrity values.
4. Record the Node version, package license inventory and dependency review.
5. Load each client secret from a separate bounded private file.
6. Allocate a free loopback port.
7. Start the provider with the configuration above.
8. Wait for bounded discovery readiness at `/.well-known/openid-configuration`.
9. Obtain a token with an authenticated POST to `/token`.
10. Send `grant_type=client_credentials`, `scope=rom` and `resource=https://rom.fixture.invalid/api`.
11. Introspect through `/token/introspection` with the separate introspection client.
12. Authenticate that same token with ROM's unmodified introspection adapter.
13. Record claim equality and acceptance results without credential bodies.
14. Stop and join only the provider process that the fixture owns.

The endpoint paths come from the pinned provider defaults. The grant performs real authenticated token issuance. [Route defaults](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/helpers/defaults.js), [client credentials grant](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/actions/grants/client_credentials.js).

The source study initially left lockfile and integrity values pending. The separate probe report now records those inputs.
Do not describe the fixture as reproducible acceptance evidence until those inputs and the actual probe are retained.
If any claim fails ROM's contract, stop the compatibility claim.
Do not rewrite responses or relax the verifier.

## Keycloak alternative

A future Keycloak fixture can use the pinned 26.4.0 archive with OpenJDK 21.
Create an isolated `rom-fixture` realm and a confidential `rom-service` client with service accounts enabled.
Disable browser and password grants for that client.
Add hardcoded string claims `sub=rom-service` and `principal_kind=service`.
Enable both `access.token.claim` and `introspection.token.claim` for those mappers.
Remove the ordinary `oidc-sub-mapper` from the fixture client's effective scopes to prevent subject overwrite.
Add audience `rom-api` through `oidc-audience-mapper` with `included.custom.audience`.
These are source-supported configuration candidates, not tested realm-export instructions. [Hardcoded claim mapper](https://github.com/keycloak/keycloak/blob/26.4.0/services/src/main/java/org/keycloak/protocol/oidc/mappers/HardcodedClaim.java), [claim flags and subject setter](https://github.com/keycloak/keycloak/blob/26.4.0/services/src/main/java/org/keycloak/protocol/oidc/mappers/OIDCAttributeMapperHelper.java), [audience mapper](https://github.com/keycloak/keycloak/blob/26.4.0/services/src/main/java/org/keycloak/protocol/oidc/mappers/AudienceProtocolMapper.java).

A retained realm JSON file can enter through `data/import` with `--import-realm`.
Startup skips an existing realm, so rotation tests need an explicit configuration update.
This alternative needs a separate compatibility probe and introspection-client authorization review. [Pinned realm import guide](https://github.com/keycloak/keycloak/blob/26.4.0/docs/guides/server/importExport.adoc).

## Limits and next acceptance boundary

The default Node storage adapter keeps tokens in process memory. Provider restart makes old opaque tokens unavailable.
This is suitable for a disposable interoperability fixture, not a persistent provider deployment. [Pinned memory adapter](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/adapters/memory_adapter.js).

ROM's native reopen tests can keep the provider process active while the ROM host restarts.
Provider restart is a separate unavailable-token case.
The host must retain approved secret references and rebuild the current activation from Resource state.
Release task 4.3 still needs bootstrap, rotation, redaction, overload and both-store CLI acceptance tests.
The host must retain current User, provider and link checks through those paths.

This selection does not establish provider uptime, human login, SSO, hosted tenant behavior, TLS trust or immediate external revocation.
It does not grant operator authorization to an authenticated service.
The later host integration must retain current User, provider and link checks.
This source-selection report does not claim an npm dependency audit, provider probe or completed integration plan.

## Research evidence

Source inspection used ROM baseline `62285bd40b7890000f89ee8fed2cba371053ada8` with concurrent operator changes.
The inspected introspection and JWT sources are unchanged from that baseline.
The introspection source SHA-256 is `71e9b3b095cb8a9e331c44d1e8598848e3ba16ff0b6f50055435cb6f57a3d8b2`.
The JWT source SHA-256 is `5c9e7681ce3afbef2de5f94561132f17a03119d974fe11c3249172a2f9afbb9a`.
The Cargo lockfile SHA-256 is `13659b8f2f4114437bb754d7b6f1a69ae25fb53e163ae52878bd6f2bcf664cf7`.
Official references use exact upstream tags where available. The Node API reference uses version 22.16.0.

The read-only prerequisite command used `nixos-container run rom-dev -- bash -lc` with `command -v`, `node --version` and `java -version`.
The JavaScript fragment passed `node --input-type=module --check` inside `rom-dev`.
That command tests syntax only. It does not import the library, validate registration or issue a token.

Technical terms retain their protocol meanings: OAuth provider, introspection, claim, subject, audience, issuer, token, verifier and secret reference.
Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary.
Full compliance requires verification against the official standard. This document does not certify compliance.
