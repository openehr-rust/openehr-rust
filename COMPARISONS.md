# Comparisons

**Not normative** (`W0.2`), and deliberately unflattering where the facts are.
The purpose of this file is to help someone choose the right tool quickly, which
usually means telling them it is not this one.

Facts about other projects were checked on **2026-08-26** against the GitHub API
and each project's own published material. They will go stale; correct them on
the tracker and they will be fixed.

## The one-line version

**openehr-rust is a set of libraries, not a clinical data repository.** It gives
a Rust program the openEHR Reference Model and a way to put records in SQL. It
does not run as a server, it does not implement the Archetype Model, and it does
not serve the openEHR REST API as a product. If you need a CDR you can deploy
this afternoon, one of the projects below is the better answer.

## Where each project sits

| Project | Language | Licence | What it is | Activity (2026-08-26) |
| --- | --- | --- | --- | --- |
| **openehr-rust** (this) | Rust | MIT / Apache-2.0 / BSD-3-Clause / GPL-2.0 / GPL-3.0, your choice | RM library + SQL persistence layer + six dialects + embedded SQLite store | public history from 2026-08-01; eight crates on crates.io at 0.9.0 |
| [**EHRbase**](https://github.com/ehrbase/ehrbase) | Java | Apache-2.0 | the reference open-source **CDR**: REST API, AQL, templates, PostgreSQL | 380 stars, actively developed |
| [**FerroEHR**](https://github.com/rubentalstra/FerroEHR) | Rust | MIT | a **CDR** in Rust: ITS-REST 1.1.0, AQL 1.1, ADL 2.4 templates, PostgreSQL 18, with a published conformance catalogue of roughly 1,100 cases | announced 2026-08-24, actively developed |
| [**Archie**](https://github.com/openEHR/archie) | Java | Apache-2.0 | the reference **library** for archetypes: ADL and AOM parsing, plus RM classes | 67 stars, actively developed |
| **Atomik openEHR SDK** (CaboLabs) | Java / Groovy | open source | SDK for operational templates and related tooling; listed on openehr.org | see the project |
| [**CaboLabs EHRServer**](https://github.com/ppazos/cabolabs-ehrserver) | Groovy | Apache-2.0 | an open-source CDR with a long deployment history | 205 stars, last pushed 2023-03-13 |
| **Better Platform**, **Ocean Health Systems**, **EHR Craft** | — | proprietary | commercial openEHR platforms with support contracts and certification | commercial |

## Choose something else if…

- **…you need a running CDR.** EHRbase is the mature open-source answer;
  FerroEHR is the Rust one, if a young project is acceptable; the commercial
  platforms are the answer if you need someone to call at 3 a.m. This project
  publishes one embeddable store and five schema dialects — the server around
  them is yours to write.
- **…you need archetypes, templates, or ADL.** This crate has the AOM2 object
  model (`openehr::am`), validation of data against an archetype already held
  in memory (`openehr::am::validate`), and a reader for an archetype's
  `definition` section (`openehr::am::cadl`) — and nothing above those. Until
  2026-08-26 the Archetype Model was excluded by decision (`lib:S1.4`); it is
  now specified by `lib:S1.21` and `openehr/spec/15-archetypes.md`, and 16 of
  those 33 requirements have no code (`lib:A-40`): no parser for a whole ADL
  archetype, no ADL 1.4, no OPT, no flattening, no template expansion. The
  `definition` reader parses 969 of the 1,379 ADL 2 files in
  `openEHR/adl-archetypes` as of 2026-09-04 and refuses the rest by name
  (`openehr/spec/corpus.md`) — a parser, not archetype support. Archie
  is the library for that work today, and every CDR above validates a
  composition against an uploaded template, which this project does not.
- **…you need the openEHR REST API as a supported product.** `openehr-loco` is
  an HTTP service over the SQLite store, it is `publish = false`, and it sits
  outside the conformance ladder entirely (`W0.32`) because every rung on that
  ladder is defined by DDL, a `Store`, or a database server. It states evidence,
  not a level. FerroEHR states ITS-REST conformance and publishes the run
  records; that is a different kind of claim.
- **…you need proven scale.** Nobody has run this in production. That sentence
  is not modesty, it is the state of the tree, and `W0.3` forbids dressing it up.

## Choose this if…

- **…you are writing Rust and want the Reference Model as types.** Constructors
  validate, `Deserialize` does not and never pretends to (`lib:A-23`), paths
  resolve against a navigation table with a test per attribute (`lib:A-28`), and
  AQL parses.
- **…precision in stored measurements matters to you.** The RM's reals are
  `base::Real`, not `f64`, so `1.50 mg` and `1.5 mg` remain different records and
  hash differently (`lib:D3.18d`). That is not a general property of openEHR
  implementations: `db:D-08` records MySQL's `JSON` type rewriting a stored
  magnitude of `1.10` as `1.1`, which is a clinical precision loss independent of
  any digest, and `conformance::check_dialect` refuses the column types that do
  it.
- **…you want the record in a file, not a server.** `openehr-sqlite` is a
  complete store in process, with no daemon — which is what offline clinics,
  edge devices, and reproducible test suites actually need. It is the only crate
  here at **Verified**.
- **…you want to know exactly how far each claim has been checked.** The
  conformance ladder and its matrix exist because "supports six databases" is a
  sentence anyone can write. Two of the six have never been parsed by a server,
  and the matrix says so.
- **…you are auditing.** The requirement tree, the register of known defects, and
  the disclosure of how the code was written are all in the repository, and CI
  fails when a document disagrees with the tree.

## The comparison this file will not make

**No performance comparison against another implementation appears here**, and
none will be published on the strength of a run this project performed on
someone else's system. See [`BENCHMARKS.md`](BENCHMARKS.md) for what is measured
and what deliberately is not.

**No conformance comparison, either.** openEHR publishes a conformance
programme; this project has not been through it, and a table implying otherwise
would be exactly the failure `W0.3` exists to prevent.

## A note on FHIR

openEHR and HL7 FHIR solve overlapping problems differently — FHIR standardises
exchange resources, openEHR standardises the record and its archetypes — and
bridging them is its own field of work (Medblocks' openFHIR is one open-source
engine for it). Nothing in this repository is a FHIR implementation, and no
crate here should be evaluated against one.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
