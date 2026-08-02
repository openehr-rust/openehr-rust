//! A `RESTful` openEHR service, on Axum and Loco.
//!
//! # Evidence, in place of a conformance level
//!
//! This crate is **outside the ladder**: every rung there is defined by DDL, a
//! `Store` implementation, or a database server, and this is none of those, so
//! it states what has been shown rather than borrowing the nearest-looking
//! level (`W0.32`).
//!
//! **Shown.** `tests/http.rs` serves requests through Loco's own router:
//! `410` for a deleted composition against `404` for one that never existed,
//! the history readable behind that `410`, `401` on every clinical route
//! without a token with a body that does not reveal whether the record exists,
//! no identity header standing in for a token, the weak `ETag`, paging and its
//! cap, `501` on `DELETE`, and `503` rather than `404` when the store is
//! absent, and the write path: a committer who is not the caller, one who
//! cannot be identified, and `If-Match` required, stale, starred, and in both
//! spellings. The `410`, the auth gate, the header prohibition, the committer
//! check, and the `If-Match` comparison were each mutation-checked. [`auth`] is covered separately, and `tests/tasks.rs`
//! executes the built binary so that [`tasks`] cannot quietly become the empty
//! body it used to be.
//!
//! **Not shown.** No real deployment, no concurrency, no TLS, no engine but
//! `SQLite`. Not published.
//!
//! # What this crate is for
//!
//! The database crates deliberately ship no server, so that a program wanting
//! storage does not also acquire a web framework (`db:S1.7`). This is where
//! that surface lives instead.
//!
//! # Its job is narrow, and the narrowness is the design
//!
//! Translate HTTP to store calls, and get the status codes right. Everything it
//! *appears* to promise — versioned history, the tamper-evident audit chain,
//! search, decimal fidelity — is [`openehr_store`]'s work, reached through
//! [`openehr_sqlite`]. This crate adds no clinical behaviour and MUST NOT: a
//! rule enforced here and not in the store would be a rule that stops applying
//! the moment somebody uses the store directly.
//!
//! It also does not authenticate. Identity is established at the deployment
//! perimeter (`db:S1.8`); what this service does is **verify** the assertion
//! that perimeter signed — a PASETO `v4.public` token, checked against a public
//! key it holds and a secret key it does not (`db:PR12.13`–`db:PR12.15`). It is
//! a relying party, not an identity provider, and [`auth`] sets out the
//! difference and why the key direction matters.
//!
//! Nor does it authorize. Every route below `/openehr/v1` requires a valid
//! token and not one of them consults who it names, because deciding *which*
//! records a clinician may open needs the care relationship, the consent
//! directives, and the break-glass rules — none of which exist here
//! (`db:PR12.18`).
//!
//! # The one distinction it does own
//!
//! **A resource that was deleted answers `410 Gone`. One that never existed
//! answers `404 Not Found`.**
//!
//! Collapsing those would tell a caller that a record it once held never was.
//! openEHR deletion is a new version carrying a deleted lifecycle state
//! (`db:H5.2`) — the history is still there, and saying "not found" would
//! misdescribe it as absence.
//!
//! # What it does not claim
//!
//! GDPR erasure is not implemented anywhere in this repository (`db:M3.18`), so
//! no endpoint offers it. Read auditing is not implemented either
//! (`db:PR12.5`), and verification makes that worth restating rather than
//! quietly improving: this service now knows who is reading, verifies it on
//! every request, and **discards it**. A deployment needing an access log must
//! still provide one above this layer, and must not assume that requiring a
//! token produced one.

#![forbid(unsafe_code)]

pub mod app;
pub mod auth;
pub mod controllers;
pub mod tasks;
pub mod views;
