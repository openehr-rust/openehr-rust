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
//! perimeter (`db:S1.8`), which is why Loco's `auth` feature is switched off
//! rather than merely unused — a JWT layer here would imply this service
//! establishes who is calling, and it does not.
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
//! (`db:PR12.5`): this service records no record of who read what, and a
//! deployment needing that must provide it above this layer.

#![forbid(unsafe_code)]

pub mod app;
pub mod controllers;
pub mod initializers;
pub mod tasks;
pub mod views;
