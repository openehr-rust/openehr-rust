//! The **Archetype Model**: AOM2 as Rust types.
//!
//! # What this is, and what it is not yet
//!
//! openEHR is a two-level model. The [Reference Model](crate::rm) is the small,
//! stable structure — `COMPOSITION`, `OBSERVATION`, `ELEMENT`, `DV_QUANTITY` —
//! and an **archetype** is a constraint on it that says which of those may
//! appear where, how many times, and with what values. A **template** combines
//! archetypes for a local purpose, and an **operational template** is the
//! flattened result a runtime validates against.
//!
//! This module started as the *object model* for those artefacts and has
//! grown the machinery around it that does not need a parser to exist:
//!
//! | Capability | Requirement | State |
//! | --- | --- | --- |
//! | AOM2 as types, with construction-time checking | `K15.1`–`K15.4` | **here** |
//! | ADL 2 parsing | `K15.5`–`K15.7` | not implemented |
//! | ADL 1.4 ingestion and conversion | `K15.8`–`K15.10` | not implemented — [`adl14::parse_header`](self::parse_adl14_header) reads the header and concept line only, and is not a step toward `K15.8`'s conversion (see its own module docs) |
//! | Specialisation and flattening | `K15.11`–`K15.13` | not implemented |
//! | Template expansion, operational templates | `K15.14`–`K15.17` | not implemented |
//! | Validating data against an archetype | `K15.18`–`K15.23` | **here** (`validate`) |
//! | Retrieval, CKM included | `K15.24`–`K15.27` | **here** (`repository`, `validate`) |
//!
//! **So this crate can tell you whether a `COMPOSITION` conforms to an
//! archetype you already have in memory, or to a `C_ARCHETYPE_ROOT` filler
//! resolved through a repository you supply — not yet whether it conforms to
//! one read from ADL, and not to one specialised from a parent it has not
//! merged in.** [`validate::validate_against_archetype`] and
//! [`validate::validate_with_repository`] validate the definition as given,
//! without flattening (`K15.11`, not implemented) or template expansion
//! (`K15.14`, not implemented) first; a construct they cannot check — a bare
//! slot, an unresolved filler, an unmodelled primitive kind — is reported
//! *unchecked*, never as a silent pass (`K15.20`). [`crate::validation`]
//! remains Reference-Model-level and separate (`K15.19`); `K15.30` requires
//! that said wherever validation is offered rather than left for a reader to
//! infer from what is missing. The remaining gap is registered as `A-40` in
//! `spec/audit.md`, and every requirement still unimplemented above appears
//! in the conformance matrix as `spec` — in force, unimplemented — rather
//! than as a plan.
//!
//! # Why an object model first
//!
//! Because everything else needs it and it needs nothing else. A parser has to
//! parse *into* something, flattening merges two of these, validation walks one
//! against data, and retrieval returns one. Building it first also means the
//! refusal discipline the rest of §15 depends on can be tested now: an
//! unrepresentable primitive constraint becomes
//! [`CPrimitive::Unsupported`] and survives a round trip, rather than being
//! dropped into an archetype that then permits anything.
//!
//! # Which openEHR release
//!
//! [`AM_RELEASE`] names it (`K15.2`). An artefact's own declared versions are
//! carried and not enforced, on the same terms `S1.16` sets for
//! `ARCHETYPED.rm_version`: an older artefact is readable, and what it declares
//! is preserved so a caller can decide.
//!
//! Note that within that release AOM 2 and ADL 2 are STABLE while **OPT 2 is at
//! DEVELOPMENT status**. `K15.15` makes OPT2 this crate's internal form
//! regardless, and `K15.16` requires reading the legacy OPT 1.4 that deployed
//! systems actually emit — a development-status specification is a real risk to
//! take deliberately rather than discover.
//!
//! # Example
//!
//! ```
//! use openehr::am::{
//!     Archetype, ArchetypeTerminology, CAttribute, CComplexObject, CObject,
//!     MultiplicityInterval, TermDefinition,
//! };
//! use std::collections::BTreeMap;
//!
//! // An archetype is built in memory, without a parser (`K15.4`).
//! let mut terms = BTreeMap::new();
//! terms.insert("id1".to_owned(), TermDefinition::new("Body weight", None).unwrap());
//! terms.insert("at0004".to_owned(), TermDefinition::new("Weight", None).unwrap());
//!
//! let weight = CObject::Complex(CComplexObject::new(
//!     "ELEMENT", Some("at0004".to_owned()), MultiplicityInterval::MANDATORY, Vec::new(),
//! ).unwrap());
//!
//! let archetype = Archetype::new(
//!     "openEHR-EHR-OBSERVATION.body_weight.v2".parse().unwrap(),
//!     CComplexObject::new(
//!         "OBSERVATION", Some("id1".to_owned()), MultiplicityInterval::MANDATORY,
//!         vec![CAttribute::single("data", MultiplicityInterval::MANDATORY, vec![weight]).unwrap()],
//!     ).unwrap(),
//!     ArchetypeTerminology::new("en", terms).unwrap(),
//! ).unwrap();
//!
//! assert_eq!(archetype.node_ids(), ["id1", "at0004"]);
//! ```

mod adl14;
mod archetype;
mod constraint;
mod multiplicity;
mod repository;
mod terminology;
mod validate;

pub use adl14::{Adl14Error, Adl14Header, parse_header as parse_adl14_header};
pub use archetype::{Archetype, ROOT_OCCURRENCES};
pub use constraint::{
    ArchetypeSlot, CArchetypeRoot, CAttribute, CComplexObject, CObject, CPrimitive,
    CPrimitiveObject, NodeIdSyntax,
};
pub use multiplicity::{Cardinality, MultiplicityInterval};
pub use repository::{ArchetypeRepository, Provenance, RepositoryError, Resolved};
pub use terminology::{ArchetypeTerminology, TermDefinition};
pub use validate::{
    ArchetypeReport, ArchetypeViolation, RepositoryOptions, Unchecked, validate_against_archetype,
    validate_with_repository,
};

/// The openEHR Archetype Model release these types are modelled against
/// (`K15.2`).
///
/// Within it, AOM 2 and ADL 2 are STABLE and OPT 2 is at DEVELOPMENT status.
pub const AM_RELEASE: &str = "2.3.0";
