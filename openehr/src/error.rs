//! Errors.
//!
//! # The rule that shapes every message in this module
//!
//! **An error must not echo the value that failed** (`X11.7`). openEHR
//! instances carry protected health information, and an error message is the
//! one place a value escapes into a log, an API response, and a support ticket
//! simultaneously — three retention policies, none of them the record's.
//!
//! So the messages here name the *path* and the *rule*, never the content:
//!
//! ```text
//! good:  invariant violated at /content[0]/data/events[0]/data/items[1]: Element.is_null
//! bad:   invariant violated: value "Systolic 184 mmHg" is not null but null_flavour is set
//! ```
//!
//! The one deliberate exception is [`ParseError`], which reports failures in
//! *identifiers and type codes* — `at0004`, `openEHR-EHR-OBSERVATION.x.v1`,
//! `SNOMED-CT`. Those are design-time vocabulary, not patient content, and an
//! identifier error that will not say which identifier is unactionable. Even
//! there the value is truncated ([`ParseError::MAX_ECHO`]) so that a caller who
//! passes a whole document into a parser by mistake does not log the document.

use core::fmt;

/// The crate-wide error type.
///
/// Callers who only need "did it work" can use [`Result`]; callers who need to
/// distinguish a malformed identifier from a violated invariant match on this.
///
/// ```
/// use openehr::{Error, base::ArchetypeId};
///
/// let err = "not-an-archetype-id".parse::<ArchetypeId>().unwrap_err();
/// assert!(matches!(err, openehr::ParseError { .. }));
/// let wrapped: Error = err.into();
/// assert!(wrapped.to_string().contains("ARCHETYPE_ID"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A lexical form did not match the grammar its type requires.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// One or more Reference Model invariants failed.
    #[error("{0}")]
    Invalid(#[from] ValidationReport),

    /// A path did not resolve, or resolved to more than one node where one was
    /// required.
    #[error(transparent)]
    Path(#[from] PathError),

    /// The operation is defined by openEHR but not implemented here.
    ///
    /// Never returned as a silent success. See `spec/01-scope.md` for the list
    /// of what is deliberately out of scope; this variant is what the code says
    /// at the boundary the spec draws.
    #[error("unsupported: {what} (see {spec_ref})")]
    Unsupported {
        /// What was asked for.
        what: &'static str,
        /// The requirement id or spec section that records the exclusion.
        spec_ref: &'static str,
    },
}

/// A lexical form did not match the grammar its type requires.
///
/// The `input` field carries at most [`ParseError::MAX_ECHO`] characters of the
/// offending text — enough to identify a malformed archetype id, not enough to
/// leak a clinical note that reached a parser by mistake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The openEHR type that rejected the input, spelled as the specification
    /// spells it: `ARCHETYPE_ID`, `OBJECT_VERSION_ID`, `TERMINOLOGY_ID`.
    pub kind: &'static str,
    /// Why the grammar rejected it.
    pub reason: &'static str,
    /// The rejected text, truncated to [`ParseError::MAX_ECHO`] characters.
    pub input: String,
}

impl ParseError {
    /// The maximum number of characters of rejected input an error will repeat.
    ///
    /// Sized to hold the longest identifier openEHR defines with room to spare
    /// — a fully specialised `ARCHETYPE_ID` runs to roughly 80 characters — and
    /// to be uselessly short for anything document-shaped.
    pub const MAX_ECHO: usize = 96;

    /// Builds a parse error, truncating the echoed input on a character
    /// boundary.
    ///
    /// ```
    /// use openehr::ParseError;
    ///
    /// let e = ParseError::new("TERMINOLOGY_ID", "empty name", "");
    /// assert_eq!(e.kind, "TERMINOLOGY_ID");
    /// ```
    #[must_use]
    pub fn new(kind: &'static str, reason: &'static str, input: &str) -> Self {
        let input = if input.chars().count() > Self::MAX_ECHO {
            let cut: String = input.chars().take(Self::MAX_ECHO).collect();
            format!("{cut}…")
        } else {
            input.to_owned()
        };
        Self {
            kind,
            reason,
            input,
        }
    }

    /// Builds an error for a broken class invariant, echoing **nothing**.
    ///
    /// Constructors use this rather than [`ParseError::new`]. The distinction
    /// is not stylistic: a rejected identifier is design-time vocabulary and
    /// safe to repeat, whereas the value that broke `DV_TEXT`'s invariant is
    /// the clinical text itself. Same error type, two entry points, and the
    /// one that could leak takes no value to leak.
    ///
    /// ```
    /// use openehr::ParseError;
    ///
    /// let e = ParseError::invariant("DV_QUANTITY", "Units_valid");
    /// assert_eq!(e.input, "");
    /// ```
    #[must_use]
    pub fn invariant(class: &'static str, invariant: &'static str) -> Self {
        Self {
            kind: class,
            reason: invariant,
            input: String::new(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid {}: {} (got {:?})",
            self.kind, self.reason, self.input
        )
    }
}

impl core::error::Error for ParseError {}

/// A path expression failed to resolve.
///
/// Paths are archetype node ids and RM attribute names — design-time
/// vocabulary — so the path itself is safe to report, and reporting it is the
/// only way the caller can act. A *predicate value* inside the path may not be
/// safe (`[name/value='Mrs Patient']`), so [`PathError`] carries the path only
/// as the caller wrote it: this crate never reconstructs a path from instance
/// data to put in an error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PathError {
    /// The path text is not a well-formed openEHR path.
    #[error("malformed path at character {offset}: {reason}")]
    Malformed {
        /// Byte offset into the path where parsing stopped.
        offset: usize,
        /// What was expected.
        reason: &'static str,
    },

    /// The path is well formed but matches nothing in this instance.
    #[error("path matched no node: {path}")]
    NoMatch {
        /// The path as supplied.
        path: String,
    },

    /// The path matched more than one node where exactly one was required.
    ///
    /// This is a distinct outcome from [`PathError::NoMatch`] because openEHR
    /// draws the distinction itself: `path_exists` is true and `path_unique` is
    /// false, and a caller that conflates them will silently take the first of
    /// several repeated `ELEMENT`s.
    #[error("path matched {count} nodes, expected exactly one: {path}")]
    NotUnique {
        /// The path as supplied.
        path: String,
        /// How many nodes matched.
        count: usize,
    },

    /// The path names an attribute that the node's RM class does not have.
    #[error("no attribute `{attribute}` on {class} (at {path})")]
    UnknownAttribute {
        /// The RM class reached.
        class: &'static str,
        /// The attribute segment that failed.
        attribute: String,
        /// The prefix of the path that resolved successfully.
        path: String,
    },
}

/// A single failed invariant.
///
/// The `path` locates the offending node; the `invariant` names the rule using
/// the specification's own invariant name, so a reader can find it in the
/// openEHR class definition without a translation step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Path to the failing node, from the root of the validated object.
    pub path: String,
    /// The RM class that owns the invariant.
    pub class: &'static str,
    /// The invariant's name as the openEHR specification states it, for example
    /// `Value_null_flavour_valid` or `Denominator_valid`.
    pub invariant: &'static str,
    /// Why it failed, stated without reference to the offending value.
    pub detail: &'static str,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}.{} — {}",
            if self.path.is_empty() {
                "/"
            } else {
                &self.path
            },
            self.class,
            self.invariant,
            self.detail
        )
    }
}

/// Every invariant that failed, not just the first.
///
/// Validation collects rather than short-circuits. A caller fixing a rejected
/// COMPOSITION wants the whole list: returning one violation per round trip
/// turns a five-minute fix into five deployments, and in a clinical import
/// pipeline the round trip is a working day.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
    violations: Vec<Violation>,
}

impl ValidationReport {
    /// An empty report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a violation.
    pub fn push(&mut self, violation: Violation) {
        self.violations.push(violation);
    }

    /// Every violation, in the order the walk encountered them (depth-first,
    /// document order), which is stable across runs.
    #[must_use]
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Whether anything failed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.violations.is_empty()
    }

    /// How many invariants failed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.violations.len()
    }

    /// Turns the report into a `Result`, so validation composes with `?`.
    ///
    /// # Errors
    ///
    /// Returns the report itself if any invariant failed.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} invariant(s) violated", self.violations.len())?;
        for v in &self.violations {
            write!(f, "\n  {v}")?;
        }
        Ok(())
    }
}

impl core::error::Error for ValidationReport {}

/// The crate's result alias.
pub type Result<T, E = Error> = core::result::Result<T, E>;
