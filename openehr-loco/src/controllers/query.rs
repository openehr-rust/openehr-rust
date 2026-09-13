//! `AQL` endpoints.
//!
//! One endpoint, `POST /openehr/v1/query/aql`, over the bounded executor
//! `openehr_store::execute_aql` implements (`db:P6.20`–`P6.24`): one `EHR`,
//! one archetype-predicated containment level, `WHERE`/`SELECT`/`ORDER BY`
//! over scalar leaves. A query outside that is refused with `400`, naming
//! the construct — never partially executed, and never the `501` this
//! service uses for a capability it does not have anywhere
//! ([`StoreError::Unsupported`](openehr_store::StoreError::Unsupported)):
//! the capability exists, and this specific query does not fit it.

use axum::{Json, extract::State, http::StatusCode, routing::post};
use loco_rs::{app::AppContext, controller::Routes};
use openehr::aql::{AqlQuery, Literal};
use openehr_store::Cell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::auth::Principal;
use crate::controllers::composition::lock;
use crate::controllers::{status_for, store};

/// The request body ITS-REST's own `POST /query/aql` specifies: `q` is the
/// query text, `query_parameters` binds its `$name` references.
#[derive(Debug, Deserialize)]
pub struct AqlRequest {
    q: String,
    #[serde(default)]
    query_parameters: HashMap<String, serde_json::Value>,
}

/// One column header in the response.
#[derive(Debug, Serialize)]
pub struct AqlColumn {
    name: String,
}

/// The response body, matching ITS-REST's own shape for the fields this
/// executor fills in.
#[derive(Debug, Serialize)]
pub struct AqlResponse {
    q: String,
    columns: Vec<AqlColumn>,
    rows: Vec<Vec<serde_json::Value>>,
}

/// Converts one `query_parameters` value into the `Literal` `AqlQuery`
/// operands compare against.
///
/// `400`, not `422`: this is a malformed request, the same footing
/// `AqlUnsupported` already stands on for the query text itself, not a
/// well-formed openEHR document this service cannot persist.
fn json_to_literal(name: &str, value: &serde_json::Value) -> Result<Literal, (StatusCode, String)> {
    match value {
        serde_json::Value::String(s) => Ok(Literal::String(s.clone())),
        serde_json::Value::Bool(b) => Ok(Literal::Boolean(*b)),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(Literal::Integer)
            .or_else(|| n.as_f64().map(Literal::Number))
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("query_parameters.{name} is not a representable number"),
                )
            }),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!(
                "query_parameters.{name} must be a string, number, or boolean, not {other}"
            ),
        )),
    }
}

/// The inverse of [`json_to_literal`], for a result row's own cells.
fn cell_to_json(cell: &Cell) -> serde_json::Value {
    match cell {
        Cell::Str(s) => serde_json::Value::String(s.clone()),
        Cell::Integer(i) => serde_json::Value::from(*i),
        Cell::Number(n) => serde_json::Number::from_f64(*n)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        Cell::Boolean(b) => serde_json::Value::Bool(*b),
        Cell::Null => serde_json::Value::Null,
    }
}

/// `POST /openehr/v1/query/aql`
///
/// Verified, then discarded, the same as every other route here — this
/// layer records commits and reads of clinical content, not which queries
/// were asked (`db:PR12.5`'s own reasoning one level up: a query's *result*
/// is content, but the query text is not, since `query_parameters` values
/// are exactly the archetype-agnostic scalars `db:P6.10` already permits
/// this layer to see without reading a composition).
async fn query(
    State(ctx): State<AppContext>,
    _principal: Principal,
    Json(body): Json<AqlRequest>,
) -> Result<(StatusCode, Json<AqlResponse>), (StatusCode, String)> {
    let parsed: AqlQuery = body
        .q
        .parse()
        .map_err(|e: openehr::aql::AqlError| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut params = HashMap::with_capacity(body.query_parameters.len());
    for (name, value) in &body.query_parameters {
        params.insert(name.clone(), json_to_literal(name, value)?);
    }

    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let result =
        openehr_store::execute_aql(&*guard, &parsed, &params).map_err(|e| status_for(&e))?;

    Ok((
        StatusCode::OK,
        Json(AqlResponse {
            q: body.q,
            columns: result
                .columns
                .into_iter()
                .map(|name| AqlColumn { name })
                .collect(),
            rows: result
                .rows
                .iter()
                .map(|row| row.iter().map(cell_to_json).collect())
                .collect(),
        }),
    ))
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/query")
        .add("/aql", post(query))
}
