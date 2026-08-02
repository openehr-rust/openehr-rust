//! Verifying the caller's assertion, with PASETO v4 public tokens.
//!
//! # This is not authentication, and the distinction is the whole design
//!
//! Authenticating means establishing an identity from a credential — a
//! password, a client certificate, a passkey — which needs a
//! credential store, a registration path, and a recovery path. This service has
//! none of those and MUST NOT acquire them (`db:S1.8`).
//!
//! What happens here is narrower and worth naming precisely: an issuer that
//! *did* authenticate somebody signs a statement saying who they are, and this
//! service checks the signature. The service is a **relying party**, never an
//! identity provider (`db:PR12.13`).
//!
//! The practical consequence is that `db:PR12.8` still holds. A deployment that
//! points [`KEYS_VAR`] at a key belonging to a careless issuer gets careless
//! attributions, and nothing here can tell.
//!
//! # Why PASETO rather than JWT
//!
//! JWT negotiates its algorithm in the token, and the token is supplied by the
//! attacker. That single design choice is the root of `alg: none`, of RS256
//! verified as HS256 against the public key as an HMAC secret, and of a decade
//! of library advisories that are all the same advisory.
//!
//! PASETO removes the negotiation: the version *is* the algorithm. `v4.public`
//! means Ed25519, and there is no field in which a caller can propose
//! otherwise. A verifier built for `v4.public` cannot be talked into doing
//! something else, because there is nothing to talk to (`db:PR12.14`).
//!
//! # Verify-only, which matters more here than in an ordinary service
//!
//! This crate holds a **public** key and no secret key, and there is no code
//! path in it that signs. That is not tidiness.
//!
//! A verified subject is destined for `AUDIT_DETAILS.committer`, which lands in
//! an append-only, hash-chained history whose entire purpose is to be evidence
//! later (`db:M3.16`, `db:M3.17`). Symmetric verification — PASETO `v4.local`,
//! or JWT's HS256 — would mean this service holds the key that *mints* tokens,
//! and an attacker who reached it could fabricate an attribution for a
//! clinician who never touched the system. That forgery would then be chained,
//! append-only, and indistinguishable from evidence.
//!
//! With `v4.public`, the worst a compromised instance can do is misuse the
//! tokens presented to it. Bad, and bounded (`db:PR12.15`).
//!
//! # PASETO replaces the header
//!
//! The principal comes from the token and from nothing else. No header names a
//! caller here: not `X-Principal`, not `X-Forwarded-User`, not
//! `X-On-Behalf-Of`, not `Remote-User`, not `X-Provenance`. There is no
//! trusted-proxy mode and no allow-listed-peer mode (`db:PR12.21`).
//!
//! The difference between the two designs is *where the check lives*. A
//! trusted header is believed because of where it arrived from, which puts the
//! check in the network diagram — and network diagrams are edited by people who
//! are not reading this. A header that is safe behind one ingress becomes
//! attacker-controlled the day a second route to the service exists, and
//! nothing in the code changes to mark that day. A signature is checked here,
//! on every request, and does not depend on any claim about topology still
//! being true.
//!
//! [`principal_from_headers`] reads exactly one header, and
//! `a_spoofed_identity_header_neither_authenticates_nor_overrides` holds it to
//! that. The prohibition is otherwise satisfied by nobody having written the
//! feature yet, which is a different thing.
//!
//! # What it deliberately does not do
//!
//! **Verification is not authorization.** Knowing that a caller is
//! `clinician-4417` says nothing about which records they may open; that needs
//! the patient–clinician relationship, the care team, the consent directives,
//! and the break-glass rules — a model this repository does not have. A service
//! that checked a `roles` claim and felt finished would be enforcing an access
//! policy nobody wrote (`db:PR12.18`).
//!
//! So every route below `/openehr/v1` demands a valid token and **none** of
//! them consult who it names.

use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use loco_rs::app::AppContext;
use pasetors::{
    Public,
    claims::ClaimsValidationRules,
    errors::{ClaimValidationError, Error as PasetoError},
    keys::AsymmetricPublicKey,
    public,
    token::UntrustedToken,
    version4::V4,
};

/// Environment variable holding the verification keys.
///
/// One or more PASERK `k4.public.…` strings, comma separated. More than one is
/// the rotation story: publish the incoming key alongside the outgoing one,
/// wait for every issued token to expire, then drop the old entry.
pub const KEYS_VAR: &str = "OPENEHR_PASETO_PUBLIC_KEYS";

/// Environment variable naming the expected `iss` claim. Optional.
pub const ISSUER_VAR: &str = "OPENEHR_PASETO_ISSUER";

/// Environment variable naming the expected `aud` claim. Optional, and see
/// [`PasetoVerifier::new`] for why leaving it unset is a real exposure.
pub const AUDIENCE_VAR: &str = "OPENEHR_PASETO_AUDIENCE";

/// Environment variable holding the implicit assertion. Optional.
///
/// PASETO v4 binds this into the signature without carrying it in the token, so
/// issuer and verifier must agree on it out of band. A deployment can use it to
/// tie tokens to one environment, which stops a staging token from being
/// replayed at production even when both trust the same issuer.
pub const IMPLICIT_VAR: &str = "OPENEHR_PASETO_IMPLICIT_ASSERTION";

/// The verifier, shared across requests.
pub type SharedVerifier = Arc<PasetoVerifier>;

/// The service could not be configured to verify anything.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// No verification key was supplied.
    ///
    /// Fatal rather than a warning, and fatal rather than falling back to
    /// serving unauthenticated traffic (`db:PR12.16`). A service that starts
    /// wide open when its configuration is incomplete fails in the direction
    /// where nothing looks wrong: the health check is green, requests succeed,
    /// and the only symptom is that anybody on the network can read the
    /// records.
    #[error(
        "{KEYS_VAR} is not set; this service verifies every request and refuses \
         to start without a verification key (see spec/databases/12-trust-principal-and-audit.md PR12.16)"
    )]
    NoKeys,

    /// One entry was not a PASERK `k4.public.…` key.
    ///
    /// Names the position and not the value: a misconfigured variable is
    /// exactly the kind of thing that gets pasted into an issue.
    #[error("{KEYS_VAR} entry {index} is not a PASERK k4.public key")]
    BadKey {
        /// Zero-based position within the comma-separated list.
        index: usize,
    },
}

/// Why a request was refused.
///
/// Deliberately coarse. The variants a caller can act on are distinguished, and
/// the rest collapse into one answer so that trying tokens against this service
/// teaches an attacker nothing about which part failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Denial {
    /// No `Authorization` header.
    Missing,
    /// The header was not `Bearer <token>`, or the token was not `v4.public`.
    Malformed,
    /// The signature verified and the token has expired.
    ///
    /// Reported distinctly because it is the one failure with an obvious remedy
    /// — refresh and retry — and because it leaks nothing: pasetors checks the
    /// signature *before* the claims, so only the holder of an already-valid
    /// token can ever see this.
    Expired,
    /// The token did not verify, or its claims were refused.
    Invalid,
    /// The token verified and names nobody.
    ///
    /// A `sub` claim is required. pasetors validates registered claims but not
    /// their presence beyond `iat`/`nbf`/`exp`, so this is checked here. A
    /// token naming nobody cannot become an `AUDIT_DETAILS.committer`, and
    /// accepting one would mean the service knew, at the moment it wrote the
    /// row, that the attribution was empty.
    NoSubject,
    /// The verifier is not installed — the service has not finished starting.
    NotReady,
}

impl Denial {
    /// The `error_description` shown to the caller, and nothing more.
    const fn describe(self) -> &'static str {
        match self {
            Self::Missing => "an Authorization: Bearer <PASETO v4.public token> header is required",
            Self::Malformed => "expected Authorization: Bearer <PASETO v4.public token>",
            Self::Expired => "the token has expired",
            Self::Invalid | Self::NoSubject => "the token could not be verified",
            Self::NotReady => "the service is still starting",
        }
    }
}

impl IntoResponse for Denial {
    fn into_response(self) -> Response {
        if self == Self::NotReady {
            // Not the caller's fault and not fixable by re-presenting anything,
            // so not a 401. Same reasoning as the missing store in
            // `crate::controllers::store`.
            return (StatusCode::SERVICE_UNAVAILABLE, Self::NotReady.describe()).into_response();
        }
        // RFC 6750 §3: a request carrying no credentials gets a bare challenge;
        // one carrying bad credentials gets `error="invalid_token"`.
        let challenge = if self == Self::Missing {
            "Bearer".to_owned()
        } else {
            format!(
                "Bearer error=\"invalid_token\", error_description=\"{}\"",
                self.describe()
            )
        };
        let mut headers = HeaderMap::new();
        if let Ok(value) = HeaderValue::from_str(&challenge) {
            headers.insert(header::WWW_AUTHENTICATE, value);
        }
        (StatusCode::UNAUTHORIZED, headers, self.describe()).into_response()
    }
}

/// Who the issuer says is calling.
///
/// Everything here is an assertion by the issuer, restated. This service
/// established none of it and MUST NOT present it as though it had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// The `sub` claim. Required.
    pub subject: String,
    /// An optional `name` claim, which maps onto `PARTY_IDENTIFIED.name`.
    pub name: Option<String>,
    /// The `iss` claim, if the token carried one.
    pub issuer: Option<String>,
    /// The `jti` claim, if the token carried one.
    pub token_id: Option<String>,
}

/// Checks PASETO v4 public tokens against a set of configured public keys.
#[derive(Debug)]
pub struct PasetoVerifier {
    keys: Vec<AsymmetricPublicKey<V4>>,
    rules: ClaimsValidationRules,
    implicit: Option<Vec<u8>>,
}

impl PasetoVerifier {
    /// Builds a verifier from the environment.
    ///
    /// # Errors
    ///
    /// [`ConfigError::NoKeys`] if [`KEYS_VAR`] is unset or empty, and
    /// [`ConfigError::BadKey`] if an entry does not parse.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::new(
            &std::env::var(KEYS_VAR).map_err(|_| ConfigError::NoKeys)?,
            std::env::var(ISSUER_VAR).ok().as_deref(),
            std::env::var(AUDIENCE_VAR).ok().as_deref(),
            std::env::var(IMPLICIT_VAR).ok().as_deref(),
        )
    }

    /// Builds a verifier from explicit configuration.
    ///
    /// `keys` is a comma-separated list of PASERK `k4.public.…` strings.
    ///
    /// # On leaving `audience` unset
    ///
    /// Without it, any token this issuer minted for *any* service is accepted
    /// here. Where one issuer serves several services that is a working
    /// cross-service replay: a token handed to a low-value service can be
    /// presented to this one. It is left optional because a single-service
    /// deployment genuinely does not need it, and made loud here because the
    /// failure is invisible until somebody looks for it.
    ///
    /// # Errors
    ///
    /// [`ConfigError::NoKeys`] if no usable key is present, and
    /// [`ConfigError::BadKey`] naming the first entry that does not parse.
    pub fn new(
        keys: &str,
        issuer: Option<&str>,
        audience: Option<&str>,
        implicit: Option<&str>,
    ) -> Result<Self, ConfigError> {
        let mut parsed = Vec::new();
        for (index, entry) in keys
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .enumerate()
        {
            parsed.push(
                AsymmetricPublicKey::<V4>::try_from(entry)
                    .map_err(|_| ConfigError::BadKey { index })?,
            );
        }
        if parsed.is_empty() {
            return Err(ConfigError::NoKeys);
        }

        // `ClaimsValidationRules::new()` requires `iat`, `nbf`, and `exp`, and
        // refuses a token whose `exp` has passed.
        //
        // `allow_non_expiring()` is never called, here or anywhere. A
        // non-expiring token is a credential that cannot be withdrawn without
        // rotating the issuer's key for everybody, and this service has no
        // revocation list to make up the difference.
        let mut rules = ClaimsValidationRules::new();
        if let Some(issuer) = issuer {
            rules.validate_issuer_with(issuer);
        }
        if let Some(audience) = audience {
            rules.validate_audience_with(audience);
        }

        Ok(Self {
            keys: parsed,
            rules,
            implicit: implicit.map(|value| value.as_bytes().to_vec()),
        })
    }

    /// Verifies a token and reports who it names.
    ///
    /// Every configured key is tried. Ed25519 verification is cheap and the set
    /// is small, so this costs nothing worth optimising — and it avoids
    /// selecting a key from the token's own footer, which would mean an
    /// attacker-supplied field steering key selection. That is the shape of
    /// mistake this crate chose PASETO to avoid, and reintroducing it one layer
    /// up would be a poor trade.
    ///
    /// # Errors
    ///
    /// A [`Denial`] describing what the caller may act on, and no more.
    pub fn verify(&self, token: &str) -> Result<Principal, Denial> {
        let untrusted =
            UntrustedToken::<Public, V4>::try_from(token).map_err(|_| Denial::Malformed)?;

        // Upgraded only by a signature-verified expiry, so a plain signature
        // failure on a later key cannot mask it.
        let mut denial = Denial::Invalid;
        for key in &self.keys {
            match public::verify(key, &untrusted, &self.rules, None, self.implicit.as_deref()) {
                Ok(trusted) => return Principal::from_trusted(&trusted),
                Err(PasetoError::ClaimValidation(ClaimValidationError::Exp)) => {
                    denial = Denial::Expired;
                }
                Err(_) => {}
            }
        }
        Err(denial)
    }
}

impl Principal {
    fn from_trusted(trusted: &pasetors::token::TrustedToken) -> Result<Self, Denial> {
        let claims = trusted.payload_claims().ok_or(Denial::Invalid)?;
        let string = |name: &str| {
            claims
                .get_claim(name)
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        };
        Ok(Self {
            subject: string("sub").ok_or(Denial::NoSubject)?,
            name: string("name"),
            issuer: string("iss"),
            token_id: string("jti"),
        })
    }
}

/// Pulls the token out of an `Authorization` header and verifies it.
///
/// **This function reads one header and no others**, which is the whole of
/// `db:PR12.21`. Anything that looks like an identity elsewhere in the request
/// — a header naming a user, a proxy, or an origin — is not consulted, cannot
/// authenticate, and cannot alter the subject. Adding a fallback here is the
/// single change that would turn this service back into a trusted-header one.
///
/// Split out from the extractor so it can be tested without standing up an
/// [`AppContext`]. What remains in the extractor is the shared-state lookup.
///
/// # Errors
///
/// A [`Denial`], as [`PasetoVerifier::verify`].
pub fn principal_from_headers(
    headers: &HeaderMap,
    verifier: &PasetoVerifier,
) -> Result<Principal, Denial> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or(Denial::Missing)?
        .to_str()
        .map_err(|_| Denial::Malformed)?;
    let (scheme, token) = value.split_once(' ').ok_or(Denial::Malformed)?;
    // RFC 7235 §2.1: the scheme is case-insensitive.
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return Err(Denial::Malformed);
    }
    verifier.verify(token.trim())
}

impl FromRequestParts<AppContext> for Principal {
    type Rejection = Denial;

    async fn from_request_parts(parts: &mut Parts, ctx: &AppContext) -> Result<Self, Denial> {
        let verifier = ctx
            .shared_store
            .get::<SharedVerifier>()
            .ok_or(Denial::NotReady)?;
        principal_from_headers(&parts.headers, &verifier)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConfigError, Denial, HeaderMap, HeaderValue, PasetoVerifier, StatusCode, header,
        principal_from_headers,
    };
    use pasetors::{
        claims::Claims,
        keys::{AsymmetricKeyPair, Generate},
        paserk::FormatAsPaserk,
        public,
        version4::V4,
    };

    fn paserk(pair: &AsymmetricKeyPair<V4>) -> String {
        let mut out = String::new();
        pair.public.fmt(&mut out).expect("PASERK formats");
        out
    }

    fn signed(pair: &AsymmetricKeyPair<V4>, claims: &Claims) -> String {
        public::sign(&pair.secret, claims, None, None).expect("signs")
    }

    fn subject_claims(subject: &str) -> Claims {
        let mut claims = Claims::new().expect("claims");
        claims.subject(subject).expect("subject");
        claims
    }

    #[test]
    fn a_token_from_the_configured_issuer_names_its_subject() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");

        let principal = verifier
            .verify(&signed(&pair, &subject_claims("clinician-4417")))
            .expect("verifies");

        assert_eq!(principal.subject, "clinician-4417");
    }

    #[test]
    fn a_token_signed_by_another_key_is_refused() {
        let ours = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let theirs = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&ours), None, None, None).expect("verifier");

        assert_eq!(
            verifier.verify(&signed(&theirs, &subject_claims("impostor"))),
            Err(Denial::Invalid)
        );
    }

    #[test]
    fn both_keys_verify_during_a_rotation() {
        let outgoing = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let incoming = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(
            &format!("{},{}", paserk(&outgoing), paserk(&incoming)),
            None,
            None,
            None,
        )
        .expect("verifier");

        for (pair, subject) in [(&outgoing, "before"), (&incoming, "after")] {
            assert_eq!(
                verifier
                    .verify(&signed(pair, &subject_claims(subject)))
                    .expect("verifies")
                    .subject,
                subject
            );
        }
    }

    #[test]
    fn an_expired_token_is_distinguishable_from_an_invalid_one() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");
        let mut claims = subject_claims("clinician-4417");
        claims.expiration("2020-01-01T00:00:00+00:00").expect("exp");

        assert_eq!(
            verifier.verify(&signed(&pair, &claims)),
            Err(Denial::Expired)
        );
    }

    #[test]
    fn a_non_expiring_token_is_refused() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");
        let mut claims = subject_claims("clinician-4417");
        claims.non_expiring();

        // Not `Expired`: there is no expiry to have passed. The rules refuse it
        // for being unwithdrawable, which is a different complaint.
        assert_eq!(
            verifier.verify(&signed(&pair, &claims)),
            Err(Denial::Invalid)
        );
    }

    #[test]
    fn a_token_naming_nobody_is_refused() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");

        assert_eq!(
            verifier.verify(&signed(&pair, &Claims::new().expect("claims"))),
            Err(Denial::NoSubject)
        );
    }

    #[test]
    fn a_token_for_another_audience_is_refused() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, Some("openehr-loco"), None)
            .expect("verifier");
        let mut claims = subject_claims("clinician-4417");
        claims.audience("some-other-service").expect("audience");

        assert_eq!(
            verifier.verify(&signed(&pair, &claims)),
            Err(Denial::Invalid)
        );
    }

    #[test]
    fn an_implicit_assertion_binds_the_token_to_one_environment() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier =
            PasetoVerifier::new(&paserk(&pair), None, None, Some("production")).expect("verifier");
        let claims = subject_claims("clinician-4417");

        let staging = public::sign(&pair.secret, &claims, None, Some(b"staging")).expect("signs");
        assert_eq!(verifier.verify(&staging), Err(Denial::Invalid));

        let production =
            public::sign(&pair.secret, &claims, None, Some(b"production")).expect("signs");
        assert!(verifier.verify(&production).is_ok());
    }

    #[test]
    fn a_local_token_is_not_accepted_as_a_public_one() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");

        // The header is the algorithm. There is no field a caller can set to
        // make a `v4.local` token be checked as `v4.public`.
        let signed = signed(&pair, &subject_claims("clinician-4417"));
        let swapped = signed.replacen("v4.public.", "v4.local.", 1);
        assert_eq!(verifier.verify(&swapped), Err(Denial::Malformed));
    }

    fn bearer(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(value).expect("header"),
        );
        headers
    }

    #[test]
    fn the_authorization_header_is_parsed_case_insensitively() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");
        let token = signed(&pair, &subject_claims("clinician-4417"));

        // RFC 7235 §2.1. A client sending `bearer` is a client that works
        // against every other server and would silently fail here.
        for scheme in ["Bearer", "bearer", "BEARER"] {
            let headers = bearer(&format!("{scheme} {token}"));
            assert_eq!(
                principal_from_headers(&headers, &verifier)
                    .expect("verifies")
                    .subject,
                "clinician-4417"
            );
        }
    }

    #[test]
    fn a_request_with_no_token_is_distinguished_from_one_with_a_bad_token() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");
        let token = signed(&pair, &subject_claims("clinician-4417"));

        assert_eq!(
            principal_from_headers(&HeaderMap::new(), &verifier),
            Err(Denial::Missing)
        );
        for value in [
            "Basic dXNlcjpwYXNz",
            &token,
            &format!("Bearer{token}"),
            "Bearer ",
        ] {
            assert_eq!(
                principal_from_headers(&bearer(value), &verifier),
                Err(Denial::Malformed),
                "{value}"
            );
        }
    }

    /// Headers a deployment might once have been asked to trust.
    const SPOOFED: [&str; 6] = [
        "x-principal",
        "x-forwarded-user",
        "x-on-behalf-of",
        "x-provenance",
        "remote-user",
        "x-authenticated-user",
    ];

    #[test]
    fn a_spoofed_identity_header_neither_authenticates_nor_overrides() {
        use axum::http::HeaderName;

        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let verifier = PasetoVerifier::new(&paserk(&pair), None, None, None).expect("verifier");
        let token = signed(&pair, &subject_claims("clinician-4417"));

        let spoof = |base: HeaderMap| {
            let mut headers = base;
            for name in SPOOFED {
                headers.insert(
                    HeaderName::from_static(name),
                    HeaderValue::from_static("chief-medical-officer"),
                );
            }
            headers
        };

        // Alone, none of them is a credential. A service that fell back to one
        // of these would be a trusted-header service again, and the fallback is
        // exactly one `else` away (PR12.21).
        assert_eq!(
            principal_from_headers(&spoof(HeaderMap::new()), &verifier),
            Err(Denial::Missing)
        );

        // Alongside a valid token, they do not win, tie, or contribute. The
        // subject is whoever the issuer signed for, unchanged.
        assert_eq!(
            principal_from_headers(&spoof(bearer(&format!("Bearer {token}"))), &verifier)
                .expect("verifies")
                .subject,
            "clinician-4417"
        );
    }

    #[test]
    fn a_refusal_carries_a_challenge_and_never_the_reason_it_failed() {
        use axum::response::IntoResponse as _;

        let missing = Denial::Missing.into_response();
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
        // RFC 6750 §3: no credentials, so a bare challenge with no error code.
        assert_eq!(
            missing.headers().get(header::WWW_AUTHENTICATE).unwrap(),
            "Bearer"
        );

        for denial in [Denial::Invalid, Denial::NoSubject] {
            let response = denial.into_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let challenge = response
                .headers()
                .get(header::WWW_AUTHENTICATE)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            assert!(challenge.contains("invalid_token"), "{challenge}");
            // A caller that presents an unsigned token and one that presents a
            // token naming nobody get the same sentence. Telling them apart
            // would turn this endpoint into an oracle for shaping a forgery.
            assert!(challenge.contains("could not be verified"), "{challenge}");
        }

        // Not a 401: the caller has nothing to fix.
        assert_eq!(
            Denial::NotReady.into_response().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn no_key_is_a_refusal_to_start_rather_than_a_default() {
        assert!(matches!(
            PasetoVerifier::new("", None, None, None),
            Err(ConfigError::NoKeys)
        ));
        assert!(matches!(
            PasetoVerifier::new("   ,  ", None, None, None),
            Err(ConfigError::NoKeys)
        ));
    }

    #[test]
    fn a_malformed_key_names_its_position_and_not_its_value() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let message = PasetoVerifier::new(
            &format!("{},k4.public.not-a-key", paserk(&pair)),
            None,
            None,
            None,
        )
        .expect_err("refuses")
        .to_string();

        assert!(message.contains("entry 1"), "{message}");
        assert!(!message.contains("not-a-key"), "{message}");
    }
}
