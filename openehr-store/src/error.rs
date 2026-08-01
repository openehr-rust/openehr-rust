//! Errors.
//!
//! The rule from the `openehr` crate applies here unchanged and matters more:
//! **an error must not echo stored content** (`X11.7`). A store error is the
//! one that reaches a connection-pool log, an APM trace, and a paging alert at
//! once, so these messages name identifiers, tables, and rules — never a
//! patient's data.

/// What went wrong.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The commit would produce a history that reads back and does not connect.
    ///
    /// Wraps the `openehr` crate's own rules rather than restating them, so the
    /// database and the library cannot disagree about what a valid history is.
    #[error("commit refused: {0}")]
    Commit(#[from] openehr::rm::common::CommitError),

    /// The record, container, or version named does not exist.
    #[error("no such {kind}: {id}")]
    NotFound {
        /// What was looked for — `ehr`, `versioned_object`, `version`.
        kind: &'static str,
        /// Its identifier. Identifiers are design-time or system-minted, not
        /// clinical content, so naming one is safe and is the only way a caller
        /// can act (`X11.7a`).
        id: String,
    },

    /// The record already exists.
    #[error("{kind} already exists: {id}")]
    Conflict {
        /// What was being created.
        kind: &'static str,
        /// Its identifier.
        id: String,
    },

    /// A stored document failed Reference Model validation on the way in.
    ///
    /// The store validates before it writes. A store that accepted an invalid
    /// composition would make every later reader's `validate()` fail on data it
    /// cannot fix.
    #[error("{0}")]
    Invalid(#[from] openehr::ValidationReport),

    /// A value could not be parsed or built.
    #[error(transparent)]
    Parse(#[from] openehr::ParseError),

    /// Canonical JSON could not be written or read.
    #[error("canonical JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// The engine reported an error.
    ///
    /// A string, because the five drivers have five unrelated error types and
    /// this crate does not depend on any of them. The engine crate is expected
    /// to have logged the typed error already.
    #[error("{engine}: {message}")]
    Engine {
        /// Which engine.
        engine: &'static str,
        /// What it said. The engine crate MUST NOT put row data in here.
        message: String,
    },

    /// The operation is defined by this crate and not implemented by this
    /// engine.
    ///
    /// Never a silent no-op. An engine crate at a conformance level below
    /// `Store` returns this rather than pretending.
    #[error("unsupported by {engine}: {what} (see {spec_ref})")]
    Unsupported {
        /// Which engine.
        engine: &'static str,
        /// What was asked for.
        what: &'static str,
        /// Where the exclusion is recorded.
        spec_ref: &'static str,
    },
}

/// This crate's result alias.
pub type Result<T, E = StoreError> = core::result::Result<T, E>;
