//! What the service is, and what it does not do.

use axum::{Json, extract::State, routing::get};
use loco_rs::{app::AppContext, controller::Routes};

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
            Absence {
                capability: "AQL execution",
                spec_ref: "db:S1.6",
            },
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
            Absence {
                capability: "authorization (a verified caller is not checked \
                             against the records it asks for)",
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
