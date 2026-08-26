//! openEHR Reference Model types, validation, paths, AQL parsing, and
//! change-control security primitives — in Rust.
//!
//! [openEHR](https://specifications.openehr.org/) is a specification for
//! clinical information: a small, stable **Reference Model** of about ninety
//! classes, plus **archetypes** that constrain it into clinical content. This
//! crate implements the Reference Model and the machinery around it, so that a
//! Rust program can read, build, check, address, and safely disclose openEHR
//! data without inventing its own idea of what a health record is.
//!
//! ```
//! use openehr::path::Pathable;
//! use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
//! use openehr::rm::data_structures::{Element, History, ItemTree, PointEvent};
//! use openehr::rm::data_types::{CodePhrase, DataValue, DvDateTime, DvQuantity};
//! use openehr::rm::ehr::{Composition, EntryAttrs, Observation};
//! use openehr::terminology::composition_category;
//! use openehr::validation::Validate;
//!
//! let at = |name: &str, node: &str| LocatableAttrs::named(name, node).unwrap();
//! let quantity = |v: f64| DataValue::Quantity(DvQuantity::new(v, "mm[Hg]").unwrap());
//!
//! let readings = ItemTree::new(at("blood pressure", "at0003"), vec![
//!     Element::new(at("Systolic", "at0004"), quantity(184.0)).into(),
//!     Element::new(at("Diastolic", "at0005"), quantity(96.0)).into(),
//! ]);
//!
//! let observation = Observation::new(
//!     at("Blood pressure", "openEHR-EHR-OBSERVATION.blood_pressure.v2")
//!         .with_archetype_details(
//!             Archetyped::new("openEHR-EHR-OBSERVATION.blood_pressure.v2", "1.1.0").unwrap(),
//!         ),
//!     EntryAttrs::about_subject(
//!         CodePhrase::new("ISO_639-1", "en").unwrap(),
//!         CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
//!     ),
//!     History::new(
//!         at("Event Series", "at0001"),
//!         DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
//!         vec![PointEvent::new(
//!             at("any event", "at0006"),
//!             DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
//!             readings.into(),
//!         ).into()],
//!         None,
//!     ).unwrap(),
//! );
//!
//! let composition = Composition::new(
//!     at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1")
//!         .with_archetype_details(
//!             Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
//!         ),
//!     composition_category::EVENT,
//!     PartyIdentified::named("Dr A Nurse").unwrap().into(),
//!     CodePhrase::new("ISO_639-1", "en").unwrap(),
//!     CodePhrase::new("ISO_3166-1", "GB").unwrap(),
//! ).unwrap().with_content(observation.into());
//!
//! // It satisfies the Reference Model's invariants…
//! assert!(composition.validate().is_empty());
//!
//! // …it is addressable by openEHR path…
//! let systolic = composition.item_at_path(
//!     "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]\
//!      /data/events[at0006]/data/items[at0004]/value/magnitude",
//! ).unwrap();
//! assert_eq!(systolic.type_name(), "primitive");
//!
//! // …and it round-trips through openEHR canonical JSON.
//! let json = serde_json::to_string(&composition).unwrap();
//! let back: Composition = serde_json::from_str(&json).unwrap();
//! assert_eq!(back, composition);
//! ```
//!
//! # Trademarks
//!
//! openEHR® is a registered trademark of openEHR International (the openEHR
//! Foundation). This project is an independent implementation: it is not
//! affiliated with, endorsed by, or certified by openEHR International.
//!
//! # What is here
//!
//! | Module | openEHR component |
//! | --- | --- |
//! | [`base`] | BASE: identifiers, references, intervals, ISO 8601 |
//! | [`am`] | AM: the AOM2 object model — archetypes, constraints, terminology |
//! | [`rm::data_types`] | RM: Data Types (`DV_*`) |
//! | [`rm::data_structures`] | RM: Data Structures (`ITEM_*`, `CLUSTER`, `ELEMENT`, `HISTORY`) |
//! | [`rm::common`] | RM: Common (archetyping, parties, audit, change control) |
//! | [`rm::ehr`] | RM: EHR (`COMPOSITION`, entries, `EHR_STATUS`, `FOLDER`) |
//! | [`rm::demographic`] | RM: Demographic (`PERSON`, `ROLE`, `ORGANISATION`) |
//! | [`terminology`] | TERM: the openEHR support terminology |
//! | [`path`] | openEHR path parsing and navigation |
//! | [`aql`] | QUERY: AQL lexing, parsing, and static checking |
//! | [`validation`] | RM invariant checking |
//! | [`security`] | `EHR_ACCESS`, audit chaining, redaction |
//!
//! # What is not here, and why
//!
//! Saying this plainly is part of the design. A clinical library that implies
//! coverage it does not have is worse than a small one.
//!
//! | Not implemented | Why |
//! | --- | --- |
//! | AQL **execution** | needs a repository; [`aql`] parses and checks, and returns no rows |
//! | Terminology lookup beyond openEHR's own | needs a terminology server; external codes are carried opaquely |
//! | UCUM unit conversion | a wrong conversion is a thousand-fold dosing error |
//! | REST service, persistence, EHR Extract | out of scope; see `spec/01-scope.md` |
//! | HL7 `GTS` / `PIVL` timing evaluation | returns [`Error::Unsupported`] rather than a guess |
//!
//! **Archetypes are a special case, and the honest statement is longer.** They
//! were excluded outright until 2026-08-26; `S1.4` is now withdrawn and §15
//! requires them. [`am`] is the AOM2 object model, and it is all that exists:
//! **no ADL parser, no flattening, no template expansion, and no way to check
//! that a `COMPOSITION` conforms to its archetype.** [`validation`] is
//! Reference-Model-level and stays that way until the conformance matrix says
//! otherwise (`K15.30`). See `spec/15-archetypes.md` and finding `A-40`.
//!
//! Where an openEHR operation is defined and not implemented, this crate
//! returns [`Error::Unsupported`] naming the spec section that records the
//! exclusion. It never returns a plausible default.
//!
//! # Three design commitments
//!
//! **Refuse rather than guess.** Comparison is partial throughout: a
//! month-precision date is not ordered against a day inside that month
//! ([`base::iso8601`]), `5 mg` is not comparable with `5 mL`
//! ([`rm::data_types::quantity`]), and a path matching three elements fails
//! rather than returning the first ([`path`]). Each of those has a plausible
//! wrong answer that no downstream reader could detect.
//!
//! **Absence is structured.** openEHR's four null flavours — nobody looked,
//! somebody looked and could not find out, the value is withheld, the question
//! does not arise — are four different clinical facts, and this crate will not
//! let them collapse. See [`rm::data_structures`].
//!
//! **Nothing prints protected health information.** No `Display` implementation
//! renders an identifier or a media blob; no error echoes a submitted value
//! ([`error`]); redaction masks rather than deletes, and counts rather than
//! names what it withheld ([`security::redact`]).
//!
//! # Two gates, not one
//!
//! Constructors enforce invariants on data this program **builds**.
//! [`validation`] enforces them on data this program **receives** — serde
//! writes fields directly and never calls a constructor. A service that
//! deserializes and stores without validating has no invariant checking at all.
//!
//! # Specification
//!
//! This crate is developed specification-first. Every normative statement has a
//! stable identifier and lives in `spec/`, which also records the known gaps
//! (`spec/audit.md`) and what is verified per requirement
//! (`spec/conformance-matrix.md`). Requirement ids are cited inline in the code
//! and in these docs — `S1.4`, `Q12.9`, `X11.7` — so prose can be traced back
//! to a decision.

#![forbid(unsafe_code)]

pub mod am;
pub mod aql;
pub mod base;
pub mod error;
pub mod path;
pub mod rm;
pub mod security;
pub mod terminology;
pub mod validation;

pub use error::{Error, ParseError, PathError, Result, ValidationReport, Violation};
