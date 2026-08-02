//! Recording who read what.
//!
//! # The gap this closes
//!
//! A version history records what **changed**. An access investigation asks who
//! **looked**, and those are different questions — a clinician who opened a
//! colleague's record and closed it again leaves no trace in a history of
//! commits (`db:PR12.5`, `db:D-04`).
//!
//! Verification made the omission worse rather than better. This service
//! establishes who is calling, on every request, and until now discarded it
//! (`db:PR12.20`). The information existed and was thrown away.
//!
//! # Where this belongs, and why it is not in the store
//!
//! `db:PR12.5` says a deployment needing read auditing must provide it **above**
//! the storage layer. This crate is that layer, and it is also the only one that
//! could: a principal exists here and nowhere below. A store used directly has
//! no caller to record, so an access log inside it would be a column that is
//! always null for every embedded user.
//!
//! # Durability: recorded *before* the data is returned
//!
//! `db:PR12.6` required this to be stated rather than left to be discovered, so:
//! **the access record is written and flushed before any clinical content
//! reaches the caller, and a read whose record cannot be written is refused.**
//!
//! The alternative — return first, record after — never blocks a read, and
//! loses exactly the records an attacker most wants lost: a crash, a full disk,
//! or a revoked file handle between the response and the write leaves the access
//! unlogged and the data delivered. For an EHR the guarantee worth having is
//! *no unaudited access*, and that costs a synchronous append per read.
//!
//! It is a real cost, and a deployment that cannot pay it turns the log off
//! rather than getting a quiet best-effort version.
//!
//! # What a record may contain
//!
//! Identifiers, never content (`db:M3.38`, `lib:X11.7a`). An access log is
//! shipped to a collector, indexed, retained on a schedule nobody chose for PHI,
//! and read by people who are not treating the patient. A record naming *which*
//! record was read is what an investigation needs; a record quoting what it said
//! is a second copy of the data with weaker protection than the first.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::auth::Principal;

/// Environment variable naming the access log.
///
/// Unset means no read auditing, which the service reports in its metadata
/// rather than leaving to be discovered.
pub const LOG_VAR: &str = "OPENEHR_ACCESS_LOG";

/// The recorder, shared across requests.
pub type SharedAccessLog = Arc<AccessLog>;

/// One access, as written.
///
/// Field order is the order a reader wants it: when, who, what, and how it
/// ended.
#[derive(Debug, Serialize)]
pub struct Access<'a> {
    /// When the read was served, RFC 3339.
    pub at: String,
    /// The verified subject.
    pub subject: &'a str,
    /// The issuer that vouched for them, if the token named one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<&'a str>,
    /// The token identifier, if it carried one. What ties several accesses to
    /// one session without naming the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<&'a str>,
    /// Which endpoint.
    pub action: &'a str,
    /// The record.
    pub ehr: &'a str,
    /// What within it — a container, a version, or a search filter.
    pub target: &'a str,
    /// How it ended: `ok`, `not_found`, `gone`, `refused`.
    ///
    /// A failed lookup is recorded too, and deliberately. Someone probing for
    /// records they cannot see is exactly what an investigation is looking for,
    /// and a log of successes only would omit it.
    pub outcome: &'a str,
}

/// Appends access records, or refuses.
#[derive(Debug)]
pub struct AccessLog {
    /// `None` when auditing is off.
    file: Option<Mutex<File>>,
    path: Option<PathBuf>,
}

impl AccessLog {
    /// Builds a recorder from the environment.
    ///
    /// # Errors
    ///
    /// Returns the reason the log could not be opened. Fatal at startup: a
    /// service configured to audit reads and unable to must not serve them.
    pub fn from_env() -> Result<Self, String> {
        match std::env::var(LOG_VAR) {
            Err(_) => Ok(Self {
                file: None,
                path: None,
            }),
            Ok(path) => Self::at(Path::new(&path)),
        }
    }

    /// A recorder that records nothing.
    ///
    /// The honest name for "read auditing is off". Constructed explicitly
    /// rather than reached by leaving an environment variable unset, so that a
    /// caller — a test, an embedded use — has to state the choice rather than
    /// arrive at it.
    #[must_use]
    pub const fn off() -> Self {
        Self {
            file: None,
            path: None,
        }
    }

    /// Opens an access log at a path.
    ///
    /// Append-only by open mode. That is not tamper evidence and must not be
    /// described as such (`db:PR12.11`) — it stops this process overwriting its
    /// own history and says nothing about someone with file access. A
    /// deployment that needs more ships the lines somewhere append-only.
    ///
    /// # Errors
    ///
    /// Returns the reason the file could not be opened for appending.
    pub fn at(path: &Path) -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("{LOG_VAR} at {} cannot be appended to: {e}", path.display()))?;
        Ok(Self {
            file: Some(Mutex::new(file)),
            path: Some(path.to_path_buf()),
        })
    }

    /// Whether reads are being recorded.
    #[must_use]
    pub const fn is_recording(&self) -> bool {
        self.file.is_some()
    }

    /// The path being written, for the metadata endpoint to *not* disclose.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Records one access, flushing before it returns.
    ///
    /// # Errors
    ///
    /// Returns the reason the record could not be written. A caller **must**
    /// treat that as fatal to the request: the whole guarantee is that no
    /// clinical content is returned for an access that was not recorded.
    pub fn record(&self, access: &Access<'_>) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let mut line = serde_json::to_string(access).map_err(|e| e.to_string())?;
        line.push('\n');

        let mut handle = file
            .lock()
            .map_err(|_| "the access log lock is poisoned".to_owned())?;
        handle
            .write_all(line.as_bytes())
            .map_err(|e| format!("the access record could not be written: {e}"))?;
        // Flushed, not buffered. A record still in a buffer when the process is
        // killed is a record that did not survive the event most likely to be
        // under investigation.
        handle
            .flush()
            .map_err(|e| format!("the access record could not be flushed: {e}"))?;
        Ok(())
    }
}

/// Builds a record for one access, stamped now.
///
/// # Panics
///
/// Never: `Rfc3339` formatting of a well-formed `OffsetDateTime` cannot fail,
/// and the fallback is an empty string rather than a panic in a code path that
/// exists to make an audit record.
#[must_use]
pub fn access<'a>(
    principal: &'a Principal,
    action: &'a str,
    ehr: &'a str,
    target: &'a str,
    outcome: &'a str,
) -> Access<'a> {
    Access {
        at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default(),
        subject: &principal.subject,
        issuer: principal.issuer.as_deref(),
        token_id: principal.token_id.as_deref(),
        action,
        ehr,
        target,
        outcome,
    }
}
