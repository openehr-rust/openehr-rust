//! What the service is, and what it does not do.

use axum::{Json, extract::State, routing::get};
use loco_rs::{app::AppContext, controller::Routes};

use crate::views::{Absence, Metadata};

/// `GET /openehr/v1/metadata`
#[allow(clippy::unused_async)]
async fn metadata(State(_ctx): State<AppContext>) -> Json<Metadata> {
    Json(Metadata {
        version: env!("CARGO_PKG_VERSION"),
        rm_version: "1.1.0",
        engine: "SQLite",
        schema_version: openehr_store::SCHEMA_VERSION,
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
            Absence {
                capability: "read auditing",
                spec_ref: "db:PR12.5",
            },
            Absence {
                capability: "authentication",
                spec_ref: "db:S1.8",
            },
        ],
    })
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new().prefix("/openehr/v1").add("/metadata", get(metadata))
}
