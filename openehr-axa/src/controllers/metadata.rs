//! What the service is, and what it does not do.

use axa::{AppContext, routes::Routes};
use axum::{Json, extract::State, routing::get};

use crate::access::SharedAccessLog;
use crate::views::{Absence, Metadata};

/// `GET /openehr/v1/metadata`
///
/// **The one route that takes no [`crate::auth::Principal`].** A caller has to
/// be able to find out how to authenticate before it can, and everything here
/// is the shape of the service rather than anything in it: a version, a
/// schema number, and a list of things that are absent. No clinical content
/// passes through this handler, which is what makes leaving it open safe.
#[allow(clippy::unused_async)]
async fn metadata(State(ctx): State<AppContext>) -> Json<Metadata> {
    let records_reads = ctx
        .shared_store
        .get::<SharedAccessLog>()
        .is_some_and(|log| log.is_recording());
    Json(Metadata {
        version: env!("CARGO_PKG_VERSION"),
        rm_version: "1.1.0",
        engine: "SQLite",
        schema_version: openehr_store::SCHEMA_VERSION,
        token_scheme: "PASETO v4.public, as Authorization: Bearer",
        records_reads,
        // Named, because a caller should not have to find these out by trying
        // them. Each cites the requirement that records the exclusion.
        not_implemented: vec![
            // AQL is **not** listed. It was, for as long as the executor was
            // served (`db:D-21`): the capability statement said "absent" about
            // a route that answered. `db:S1.6` bounds what the executor does
            // (`P6.20`-`P6.24`); it is not an absence.
            Absence {
                capability: "archetype and template validation",
                spec_ref: "lib:S1.4",
            },
            Absence {
                capability: "GDPR Art. 17 erasure",
                spec_ref: "db:M3.18",
            },
            // Listed only when it is genuinely absent. A fixed list would keep
            // saying "not implemented" after a deployment turned it on, which
            // is the kind of stale claim this file exists to avoid.
            Absence {
                capability: "read auditing",
                spec_ref: "db:PR12.5",
            },
            // Not "authentication": this service verifies a token. It does not
            // establish who anyone is, and it makes no access decision once it
            // knows. Both absences are named, because "requires a token" is
            // routinely read as covering the second one too.
            Absence {
                capability: "authentication (no credential is checked here; \
                             an issuer signs the assertion)",
                spec_ref: "db:PR12.13",
            },
            // Authorization itself is enforced: every clinical route checks the
            // token's `openehr_grants` (`db:PR12.18`, amended 2026-11-26). What
            // is absent is the policy behind the grants. That is the issuer's.
            Absence {
                capability: "care-relationship policy (access is exactly the \
                             issuer's openehr_grants; no care team, consent or \
                             break-glass model is held here)",
                spec_ref: "db:PR12.18",
            },
        ]
        .into_iter()
        .filter(|absence| !(records_reads && absence.capability == "read auditing"))
        .collect(),
    })
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1")
        .add("/metadata", get(metadata))
}
