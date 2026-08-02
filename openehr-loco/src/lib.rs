//! A `RESTful` openEHR service, on Axum and Loco.
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
pub mod initializers;
pub mod tasks;
pub mod views;
