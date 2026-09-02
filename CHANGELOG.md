# Changelog

Covers the eight published crates as a set: `openehr`, `openehr-store`,
`openehr-sqlite`, `openehr-postgresql`, `openehr-mysql`, `openehr-mariadb`,
`openehr-mssql`, `openehr-oracle`. They are versioned in lockstep and released
together.

## Unreleased

- `am::validate` now actually evaluates a `C_ATTRIBUTE_TUPLE` constraint
  against instance data, rather than reporting every one of them `Unchecked`
  unconditionally — closes `A-50`'s own residual. See `A-58` in
  `openehr/spec/audit.md`. Not a breaking change: no public type or function
  signature changes, only what `validate_against_archetype` reports for
  archetypes using the tuple form (AOM2's replacement for ADL 1.4's
  `C_DV_QUANTITY`/`C_DV_ORDINAL`).
- `am::ArchetypeSlot` gains `is_closed`/`closed()`/`is_closed()`/
  `any_allowed()`, AOM2's `ARCHETYPE_SLOT.is_closed` and its function, which
  had no counterpart at all before now. See `A-59` in
  `openehr/spec/audit.md`. Not a breaking change: an additive field
  defaulting to AOM2's own stated default, `#[serde(default)]` on the wire.
  Carried, not enforced — `am::validate` cannot check it yet, for the same
  reason it cannot check a slot's filler at all.

## 0.9.0 — 2026-09-02

**A minor bump, not a patch, over two breaking API changes.** Cargo treats
`0.8.x` as compatible with `0.8.0`, and both changes below break a build
that resolves against the local path today: `CObject::occurrences()`'s
return type changes for every caller, and `CPrimitive::TerminologyCode`
loses a field a `match` naming it exhaustively would no longer compile
against. Both are decided rather than deferred again — see `A-54`/`A-55` in
`openehr/spec/audit.md` for the residuals they close and why now rather than
earlier — and both are additive in spirit: neither removes anything AOM2
itself expresses, only the shape this crate used to say it in.

Everything else below is additive: the whole `openehr::am` archetype-model
surface this release adds — `C_ATTRIBUTE_TUPLE`/`C_PRIMITIVE_TUPLE`,
`constraint_status`, `RM_OVERLAY`, `C_COMPLEX_OBJECT_PROXY`, a bounded cADL
`definition` parser, `Inv_valid_assumed_value` checking — plus the four
temporal primitive kinds, `C_PRIMITIVE_OBJECT`'s `node_id`, and
`base::TerminologyCode`/`TerminologyTerm`, all landed in this release too
and are documented below in the order they were built.

**New: `openehr::am::parse_adl2_header` — read an ADL 2 archetype's
`archetype` and `specialize` lines.** Returns the archetype's own
identifier and its parent's identifier if it specialises one — checked
against the real ADL 2 grammar (`openEHR/adl-antlr`, `adl2.g4`). ADL 2
drops the `concept` section ADL 1.4 has, so unlike
`parse_adl14_header` this returns no concept code; confirmed by a test
that a source shaped like an ADL 1.4 header (with a trailing `concept
[at0000]`) is refused rather than silently accepted.

**Scope, stated rather than implied.** This is not `K15.5`: it does not
parse `language`, `description`, `definition`, `rules`, `terminology`, or
`annotations`, cannot build an `Archetype`, and refuses by name the
moment it reaches anything past the header, per `K15.6`/`K15.7`. See
`openehr::am::adl2`'s own module documentation, and `K15.5` in
[`openehr/spec/15-archetypes.md`](openehr/spec/15-archetypes.md).

**New: `openehr::am::parse_adl14_header` — read an ADL 1.4 archetype's
`archetype` and `concept` lines.** Returns the archetype's own identifier,
its parent's identifier if the archetype specialises one, and the local
concept term code — checked against the real ADL 1.4 grammar
(`openEHR/adl-antlr`) and tested against a real published archetype's actual
header bytes (`openEHR-EHR-OBSERVATION.blood_pressure.v1`), not an invented
fixture.

**Scope, stated rather than implied.** This is not `K15.8`: it does not
parse `language`, `description`, `definition`, `invariant`, or `ontology`,
cannot build an `Archetype` (which needs a `definition` and a
`terminology`, neither read here), and refuses by name — never silently —
the moment it reaches anything past `concept [<code>]`, per `K15.6`/`K15.7`'s
discipline. It is useful for identifying and cataloguing ADL 1.4 source, and
is explicitly not a step toward `K15.8`'s full conversion, which
`spec/audit.md` **A-40**'s residual already scopes at several weeks of work.
See `openehr::am::adl14`'s own module documentation, and `K15.8` in
[`openehr/spec/15-archetypes.md`](openehr/spec/15-archetypes.md).

**New: `openehr::am::validate` — validate a Reference Model instance against
an archetype (`K15.18`–`K15.23`).** Existence, cardinality, occurrences, RM
class and node identity, and primitive value constraints (`C_BOOLEAN`,
`C_STRING` list, `C_INTEGER`, `C_REAL`, `C_TERMINOLOGY_CODE` against the
archetype's own internal codes, and `C_DATE`/`C_TIME`/`C_DATE_TIME`/
`C_DURATION` ranges) are checked by walking the instance and the archetype's
`definition` in parallel. The verdict is a new type, `ArchetypeReport`, kept
separate from Reference-Model validation's own verdict (`K15.19`) rather than
folded into it.

**The four temporal primitive kinds were added after the rest**, once
`base::Interval<T>` could be bounded on `base::Date`/`Time`/`DateTime`/
`Duration` — none of the four implemented `SemanticOrd` before this, so
`Interval<Date>` was a compile error, not a missing check. Each is a list of
ranges rather than `C_INTEGER`/`C_REAL`'s discrete-list-plus-one-range shape,
matching AOM2's own `C_DATE` etc. (`List<Interval<Iso8601_date>>`); a
`pattern` (e.g. `"YYYY-??-??"`) is carried and never evaluated, the same
choice already made for `C_STRING`'s own pattern field. This closes a gap
that mattered in practice: `path.rs` already exposed
`DV_DATE`/`DV_TIME`/`DV_DATE_TIME`/`DV_DURATION` as their ISO 8601 text, so
the missing piece was purely the constraint side, and most realistic
archetypes constrain at least one date or time field.

**Scope, stated rather than implied.** This validates against an `Archetype`
already held in memory, not against a flattened operational template: this
crate still does not parse ADL (`K15.5`) or flatten a specialised archetype's
inherited constraints (`K15.11`–`K15.13`, `K15.14`–`K15.17`). A construct it
cannot check — a bare `ARCHETYPE_SLOT`, an unmodelled primitive constraint, a
`C_STRING` pattern (carried but not compiled or applied) — is reported
**unchecked**, and `ArchetypeReport::is_conformant` is `false` whenever
anything is unchecked, never a silent pass (`K15.20`). See
`openehr::am::validate`'s own module documentation, and `A-40` in
[`spec/audit.md`](spec/audit.md) for what in §15 is still open.

**New: `openehr::am::repository` — resolve a `C_ARCHETYPE_ROOT` filler through
a repository you supply (`K15.24`–`K15.27`).** `ArchetypeRepository` is one
method, `resolve`; `openehr` itself performs no network or filesystem I/O
(`K15.25`), so no implementation of the trait lives in this crate. New
`validate_with_repository` resolves a `C_ARCHETYPE_ROOT` filler through one and
validates the same subtree against the filler's own definition and
terminology, attributing any violation to the *filler's* archetype id, not
the outer one. Verifies the repository answered the identifier actually
requested, requires `RepositoryOptions::allow_unestablished_provenance` before
validating against a `Resolved` with no `Provenance` (recording every such use
in `ArchetypeReport::unverified_provenance` regardless, `K15.26`), and reports
a retrieval failure as unchecked, never as a pass (`K15.27`). A bare
`ARCHETYPE_SLOT` is unaffected: which archetype fills it lives on the
instance's `ARCHETYPED.archetype_id`, which `crate::path::Node` does not
expose, so nothing here can name what to resolve — a stated gap, not a silent
one.

**New: `CPrimitiveObject::with_node_id`/`node_id()`, and
`CPrimitiveObject::PRIMITIVE_NODE_ID`.** Every `C_OBJECT` has a `node_id`
(`org.openehr.am.aom2.c_object.adoc`: `1..1`), and `CObject::node_id`'s
dispatcher already read `CPrimitiveObject`'s field, but nothing could ever
set it — it stayed `None` unconditionally. `with_node_id` mirrors
`CArchetypeRoot`'s own pair, and accepts one value `NodeIdSyntax::of` alone
would reject: `PRIMITIVE_NODE_ID`, the literal string
`"Primitive_node_id"`, AOM2's own sentinel for a `C_PRIMITIVE_OBJECT`
written inline in ADL with no node id of its own. See `openehr/spec/audit.md`
**A-46**.

**New: `openehr::base::TerminologyCode`/`TerminologyTerm`.** BASE Foundation
Types, not `CODE_PHRASE`: `Terminology_code.terminology_id` is a bare
namespace string rather than a structured `TerminologyId`, and it carries an
optional `terminology_version` and `uri` besides. This is the declared type
of `AUTHORED_RESOURCE.original_language`, `RESOURCE_DESCRIPTION_ITEM.language`,
and `TRANSLATION_DETAILS.language` — none of which this crate models, and
`S1.1` does not commit it to the `resource` package they belong to. Added on
its own because it is independently well-formed, not because those three
classes are now in progress. See `openehr/spec/audit.md` **A-47**.

**New: `openehr::am::PrimitiveValue`, and `CPrimitiveObject::with_assumed_value`
/`assumed_value()`.** `C_PRIMITIVE_OBJECT.assumed_value: Any` had no field at
all. `PrimitiveValue` covers `Boolean`, `Integer`, `Real` (`base::Real`, not
`f64`), and one `Text` variant standing in for `C_STRING`, `C_DATE`,
`C_TIME`, `C_DATE_TIME`, `C_DURATION`, and `C_TERMINOLOGY_CODE` alike —
the same collapsing `crate::path::Scalar::Str` already makes for the
corresponding `DataValue`s. **`Inv_valid_assumed_value` — that the value
conforms to the node's own `constraint` — is not checked**, the same choice
already made for `C_STRING`'s `pattern`: a `Boolean` assumed value attached
to a `C_INTEGER` constraint is accepted exactly as given. See
`openehr/spec/audit.md` **A-48**.

**Fixed: `parse_adl14_header`/`parse_adl2_header` used the wrong
archetype-identifier grammar for the header's own line.** Both grammars name
it `ARCHETYPE_HRID`, not `ArchetypeId` — a richer token allowing an optional
`namespace::` prefix and a prerelease version suffix (`-rc.4`, `-alpha`,
`-beta`), which `ArchetypeId` accepts neither of. **New:
`openehr::am::ArchetypeHrid`/`VersionStatus`**, modelling `ARCHETYPE_HRID`
faithfully and checked against `openEHR/adl-antlr`'s real lexer grammar;
`Adl14Header.archetype_id` and `Adl2Header.archetype_id` now hold one. The
`specialize` line's identifier is unchanged (`Option<ArchetypeId>`) and
remains narrower than its own grammar (`ARCHETYPE_REF`, or for ADL 2 either
`ARCHETYPE_HRID` or `ARCHETYPE_REF`) allows — declared, not fixed, in this
pass; see `openehr/spec/audit.md` **A-49**.

**New: `openehr::am::CAttributeTuple`/`CPrimitiveTuple`, and
`CComplexObject::with_attribute_tuples`/`attribute_tuples()`.** AOM2's
mechanism for a co-varying constraint — `{units, magnitude}` on a
`DV_QUANTITY`, `{value, symbol}` on a `DV_ORDINAL` — "replaces all
domain-specific constraint types defined in ADL/AOM 1.4, including
`C_DV_QUANTITY` and `C_DV_ORDINAL`", and had no counterpart here at all:
`CComplexObject` had nowhere to put one, so it vanished silently on JSON
read. A structural invariant is checked at construction — every tuple row's
arity must match the number of co-varying attributes. `am::validate` reports
a node governed by one as `Unchecked`, naming the attributes it covers,
rather than evaluating it. See `openehr/spec/audit.md` **A-50**.

**New: `CPrimitive::TerminologyCode::constraint_status`, and
`openehr::am::ConstraintStatus`.** Fixes a false violation on conformant
data: AOM2 states that an `extensible`/`preferred`/`example` terminology
constraint is satisfied by *any* terminology code, and with no field to
carry that, `am::validate` checked every `C_TERMINOLOGY_CODE` as though it
were `required` — a real archetype using the specification's own
recommended `extensible` pattern would have every conformant instance whose
code is not already in the value set reported as a violation. See
`openehr/spec/audit.md` **A-51**.

**New: `openehr::am::RmOverlay`/`RmAttributeVisibility`/`VisibilityType`, and
`Archetype::with_rm_overlay`/`rm_overlay()`.** `ARCHETYPE.rm_overlay` had no
counterpart at all — visibility and aliasing statements for RM attributes
outside the constrained structure vanished silently on JSON read.
`Inv_alias_validity` is checked at construction. Authoring-tool metadata
only; `am::validate` does not read it. See `openehr/spec/audit.md` **A-52**.

**New: `openehr::am::CComplexObjectProxy`, and `CObject::Proxy`.**
`C_COMPLEX_OBJECT_PROXY` — a node that references another node's constraint
by path instead of repeating it — had no counterpart under any name.
`am::validate` reports a node governed by one as `Unchecked`, naming
`target_path`, rather than resolving it. See `openehr/spec/audit.md`
**A-53**.

**BREAKING: `CObject::occurrences()` now returns
`Option<&MultiplicityInterval>`, not `&MultiplicityInterval`.** Closes
`A-53`'s own residual: only this widening lets `CComplexObjectProxy`
represent AOM2's `use_target_occurrences()` — `None` meaning "defer to the
target node's own occurrences", which this crate does not resolve. The four
other `C_OBJECT` variants are unaffected in every other respect and always
return `Some`; `CAttribute::single`/`container`'s own construction-time
checks treat a deferred child per AOM2's own stated default (assume a lower
bound of `0`; the upper-bound checks do not apply to it, since its effective
upper bound depends on a target this crate does not resolve). See
`openehr/spec/audit.md` **A-54**, and see there too for why this is an
ordinary source change rather than a version bump: this repository's own
history already treats a breaking `0.x` API change as a normal commit,
version-bumped only at the next actual `cargo publish`
(`agents/publishing.md`).

**BREAKING: `CPrimitive::TerminologyCode` no longer has a `code_list`
field.** Closes `A-51`'s own residual: `code_list` had no counterpart in
AOM2's actual single-valued `constraint: String` attribute. `constraint:
Option<String>` now carries either an `at`-code (an exact required value) or
an `ac`-code (a value set), distinguished by AOM2's own `"ac"` leader
convention rather than by which field a caller populated; multiple
alternative codes are now expressed as sibling `C_OBJECT`s, the same
alternative-matching shape every other node kind already uses. See
`openehr/spec/audit.md` **A-55**.

**New: `openehr::am::parse_definition` — a bounded cADL parser for
`definition`'s own grammar rule.** Reads `c_complex_object`
(`openEHR/adl-antlr`, `cadl2.g4`) — the constraint tree itself — for a real
subset of node kinds (`C_COMPLEX_OBJECT`, `C_PRIMITIVE_OBJECT`, both the
wrapped and unwrapped primitive forms) and primitive constraint kinds
(`Boolean`/`String`/`Integer`/`Real`/`Terminology_code` wrapped or
unwrapped; the four temporal kinds wrapped only), refusing everything else
by name: `C_ATTRIBUTE_TUPLE`, `ARCHETYPE_SLOT`, `C_ARCHETYPE_ROOT`,
`C_COMPLEX_OBJECT_PROXY`, `SIBLING_ORDER`, `default_value`, string/date
patterns, a terminology assumed-value, more than one disjoint range on one
node, and the relop/`+/-` interval forms. Tested against a real archetype's
own bytes (`openEHR/adl-archetypes`,
`openEHR-EHR-CLUSTER.device.v1.0.0.adls`), confirming the honest outcome
`K15.6`/`K15.7` require on real text this parser cannot fully consume: a
named refusal, never a silent partial tree. **Not `K15.5`** — see
`openehr/spec/15-archetypes.md`'s own `K15.5` entry and
`am::cadl`'s module documentation for the exact boundary and why it is
drawn where it is; this does not build an `Archetype`, and does not
compose with `am::adl2::parse_header` into more of `K15.5` than either
addition is alone.

**Fixed: `Inv_valid_assumed_value` — a `C_PRIMITIVE_OBJECT`'s
`assumed_value` conforming to its own `constraint` — was never checked.**
Closes `A-48`'s own residual. A kind-mismatched assumed value (`Boolean` on
a `C_INTEGER` constraint) or an out-of-range one was accepted silently all
the way through to a caller who never suspected either. Checked in
`Archetype::check`, not at `CPrimitiveObject::with_assumed_value` — which
stays exactly as permissive as documented, since it builds one node in
isolation, before the terminology a `Terminology_code` `ac`-code needs is in
scope. `C_UNSUPPORTED` is excluded from the check rather than guessed at,
the same reasoning `VASID`/`VACSD` already state for what this crate cannot
establish at all. See `openehr/spec/audit.md` **A-56**.

**Fixed: ADL 2 and ADL 1.4 header parsing (`am::adl2::parse_header`,
`am::adl14::parse_header`) silently failed on a `meta_data` clause in any
release-profile build.** `adl_lexer::Lexer::skip_parenthesised` put a
side-effecting token read inside `debug_assert!`, whose argument is not
evaluated at all when `debug-assertions` is off — the default for
`cargo build --release` and for any downstream consumer's own release
build. A well-formed `(adl_version=2.4.0; ...)` block was refused as
`"unterminated (...) metadata"`. Invisible to `cargo test`, which always
builds in the `dev` profile; caught by `cargo bench --benches -- --test`
while preparing this release, and reproduced before being touched. See
`openehr/spec/audit.md` **A-57**.

## 0.8.0 — 2026-08-29

**BREAKING: the MSRV floor moved from N−3 to N−2 — 1.95 to 1.96.** `RV1`
([`spec/rust-msrv-n-minus-2/index.md`](spec/rust-msrv-n-minus-2/index.md)) now
tracks stable two minor releases back instead of three; `RV6` requires this to
be stated here and to not go out as a patch, because raising the floor breaks
a consumer building below it — Cargo refuses the build with a clear message
rather than miscompiling, but "your dependency silently stopped supporting
your toolchain" is a thing a user is entitled to read before it happens.

`rust-version = "1.96"` in all eighteen manifests, the `msrv` CI job's
derivation changed from `N − 3` to `N − 2`, and every prose statement of the
floor updated to match — `python3 scripts/check-docs.py` and the `msrv` job's
own manifest/prose check both pass against the new number. Nothing else about
the public API changed.

## 0.7.4 — 2026-08-27

**The notice becomes the Foundation's own prescribed attribution.** openEHR
granted this project permission to use their trademarks (owner-reported,
2026-08-27; `TRADEMARKS.md` §Permission is the record), and at the owner's
direction every notice site — the crate descriptions and rustdoc that ship
to crates.io and docs.rs, the crate READMEs, and every root and help
document — now carries the attribution openehr.org/logos/ prescribes,
verbatim: "openEHR® is the registered trademark of the openEHR Foundation
and is used with the permission of openEHR International. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation." The descriptions keep the three-part
shape; `scripts/check-trademarks.py`'s enforced constant moved with the
wording and was plant-tested against a broken rustdoc section and a broken
description. `TRADEMARKS.md` is new at the root: the mark-by-mark table
with the verified registrations, what is and is not claimed, and the dated
permission record.

The 0.7.1–0.7.3 crates on crates.io carry the previous wording, which a
published version keeps forever; this release exists so the pages people
read state the permission in the mark owner's own words. No code changed.

## 0.7.3 — 2026-08-26

**Every crate `description` gets the owner-specified three-part shape, and
the checker enforces it.** The shape is `<short description>. <notice> This
project is an independent work.` — 0.7.2's descriptions carry the notice but
not the closing independent-work sentence, and `openehr-mysql`'s runs "DDL"
straight into "openEHR®" with no full stop. Both are immutable in 0.7.2;
this release is the remedy. `openehr-loco` carries the same shape by the
lockstep convention even though it is never published.

**`scripts/check-trademarks.py` now verifies the descriptions**: every
publishable manifest's `description` must end with the notice verbatim
followed by the independent-work sentence, with a full stop between the
short description and the notice. Both failure modes were plant-tested
against a deliberately broken manifest and reported on exactly that
manifest.

No code changed.

## 0.7.2 — 2026-08-26

**The trademark notice reaches the crate `description` and gets prominent in
the crate READMEs.** Every publishable crate's `Cargo.toml` `description` —
what crates.io shows in search results and at the top of the crate page — now
carries the notice verbatim, the crate READMEs open with it as a blockquote,
and the short descriptions write the mark as `openEHR®`.

Released ahead of the process in `agents/publishing.md`: the description
shape was not yet final (see 0.7.3), and the inter-crate pins, staged state,
and this changelog moved afterwards. No code changed.

## 0.7.1 — 2026-08-26

**Trademark notice reworded to the owner-specified text.** Every page that
carries the notice now reads "openEHR® is the registered trademark of the
openEHR Foundation. Use of the trademark does not constitute endorsement of
this product by openEHR International or openEHR Foundation.", and
`scripts/check-trademarks.py` enforces that wording. The 0.7.0 crates on
crates.io carry the previous wording, which a published version keeps forever;
this release exists so the crate pages and rustdoc show the owner-specified
text. No code changed — the release is the notice, plus the install snippets
in the eight crate READMEs moving off a stale `"0.2"` to `"0.7"`, since those
READMEs are what crates.io renders.

## 0.7.0 — 2026-08-26

**`#![forbid(unsafe_code)]` at every crate root and every fuzz target** — 32
files, added 2026-08-26. The ten buildable crates already forbade it through
`[lints.rust]` in their manifests; the attribute states the same guarantee in
the source, where removing it is a visible edit to the file it protects rather
than a line in a manifest.

**The eight fuzz crates were not covered before this.** They carried no
`[lints]` table, so `unsafe_code` was not forbidden in any of the 21 fuzz
targets, while this repository's documentation said the tree forbids it. No
`unsafe` was present — `grep -rn '\bunsafe\b' --include='*.rs'` finds one hit,
in a comment explaining why a test cannot drive `App::before_run` — so the claim
was true of the code and false of the configuration.

Both halves are now in place in all eighteen crates: `unsafe_code = "forbid"` in
every manifest, covering files not yet written, and the attribute in every
existing root and target, surviving a manifest edit. Verified with
`cargo fuzz build` across all eight fuzz crates and `cargo test` plus
`RUSTFLAGS="-D warnings" cargo clippy --all-targets` across the ten buildable
ones.

**Scope change, specification only — no code changed.** The Archetype Model is
now in scope. `S1.4` — *the crate MUST NOT implement the Archetype Model* — was
withdrawn on 2026-08-26 under the new `C0.19`, and `S1.21` plus
[`openehr/spec/15-archetypes.md`](openehr/spec/15-archetypes.md) require AOM2,
ADL 2 parsing, ADL 1.4 ingestion, specialisation and flattening, template
expansion, operational templates, validation of Reference Model data against an
operational template, and a repository abstraction for retrieval.

**Added: `openehr::am`, the AOM2 object model** (`K15.1`–`K15.4`). `Archetype`,
`CComplexObject`, `CAttribute`, `CObject`, `CPrimitive`, `ArchetypeSlot`,
`CArchetypeRoot`, `MultiplicityInterval`, `Cardinality`,
`ArchetypeTerminology`, and `TermDefinition`, with construction-time checking of
the AOM2 validity conditions that one artefact decides on its own — `VARDT`,
`VATDF`, `VACDF`, `VATCD`, `VOKU` — and `Archetype::check` to re-run them on
anything that arrived as JSON, because `Deserialize` writes fields straight in
(`L10.1a`). `am::AM_RELEASE` names the targeted release, 2.3.0.

A primitive constraint this crate cannot model becomes
`CPrimitive::Unsupported` and survives a round trip rather than being dropped:
a dropped constraint silently widens an archetype, which is the failure the
withdrawn `S1.4` predicted.

**Twenty-eight of the thirty-two requirements remain unimplemented**, and the
practically important one is among them: **this crate still cannot tell you
whether a `COMPOSITION` conforms to its archetype.** No ADL parser, no
flattening, no template expansion, no operational template, no retrieval. The
conformance matrix marks each `spec` and `A-40` tracks them. `validate()` is
unchanged and remains Reference-Model-level.

`L10.2` is amended: validation now has two levels, and a verdict must say which
one produced it. `S1.5` is unchanged — AQL is still parsed and not executed
(`K15.29`).

## 0.6.0 — 2026-08-22

**A representation change, and the reason it is not 0.5.1.** The source API is
additive — `magnitude()` still returns `f64`, `Real` and the `_real` accessors
are new — but **serialization changes**: a document carrying `1.50` now
round-trips as `1.50` rather than `1.5`, and its canonical digest differs from
what 0.5.0 produces for the same input. Cargo treats `0.5.x` as compatible, so a
patch would reach dependents on `cargo update` and silently change their
digests.

**Stored data is unaffected.** `db:M3.43` keeps canonical JSON byte-preserving
and `verify_versions` hashes the bytes that were stored, so records written by
0.5.0 keep their bytes and still verify. What changes is the bytes produced for
new commits from input that carried digits an `f64` discards.


- **BREAKING (representation, not signature): the Reference Model's real
  numbers preserve the digits they were written with** (`lib:D3.18d`–`D3.18f`).
  `1.50 mg` and `1.5 mg` are now different records and hash differently.

  `DV_QUANTITY.magnitude` and `.accuracy`, `DV_SCALE.value`,
  `DV_PROPORTION.numerator` and `.denominator`, and `DV_COUNT.accuracy` are
  `openehr::base::Real` instead of `f64`. `serde_json`'s `arbitrary_precision`
  feature is enabled, which is what makes the literal text reachable.

  **The `f64` accessors are unchanged.** `magnitude()` still returns `f64`;
  `magnitude_real()` is new and returns the text. Same for `value`, `accuracy`,
  `numerator`, `denominator`. Code that reads magnitudes compiles untouched.

  What does change for a caller: a struct-literal construction of these types
  (there is none in the public API — all go through constructors), and any code
  matching on the field types. Serialized output changes only where the input
  carried digits an `f64` discards, which is the point.

  Every digit survives, including trailing zeros and significant digits beyond
  what an `f64` can hold. One measured exception: exponent notation normalises,
  `1e5` and `1E5` both to `1e+5`, with no digit lost and the value unchanged.

  `db:D-08` is this same loss one layer out — MySQL rewrote a stored `1.10` as
  `1.1`, changing bytes a content digest covered, and `db:M3.43` moved canonical
  JSON onto a byte-preserving column for it. The crate had been discarding the
  digits before storage ever saw them, and `security::canonical`'s own test
  recorded that as the limit of the guarantee. It no longer is.

- **`serde_json`'s `float_roundtrip` feature is enabled**, closing `lib:A-38`.
  A `DV_QUANTITY` magnitude no longer drifts across repeated canonical round
  trips: `serde_json`'s parser was one ULP off `core::str::parse` for some
  inputs, so it was not the inverse of its own serializer.

  Recorded here because the effect is visible to a dependent: a value read back
  is now bit-identical to the value written, where before it could move. The
  digest over the *stored* bytes was never affected (`db:M3.43`).

  `arbitrary_precision` is deliberately **not** enabled — it is incompatible
  with this crate's `#[serde(tag)]` and `#[serde(flatten)]` layout, and its
  benefit applies to `serde_json::Number` rather than to the `f64` fields the
  Reference Model uses. See `spec/serde-json-float-roundtrip-arbitrary-precision/`.


- **`conformance::check_projection` and `check_verify_versions` now return what
  they checked** — `bool` for whether the composition projected, `usize` for how
  many versions had their tamper detection provoked (`db:D-10`). Not breaking:
  a caller ignoring the result still compiles.

  They return anything because otherwise they could not fail. Both were
  replaceable with `()` and nothing in the repository noticed — they are called
  only from `openehr-store-fuzz`, `cargo test` does not run fuzz targets, and a
  property that asserts nothing never crashes. `check_verify_versions` also now
  **provokes** what it is about: for a history that verifies, editing each
  version's content must make the chain report `ContentAltered`.

- **No behaviour change in `openehr`**, but two matches in `DataValue` gained
  the tests that make them non-deletable (`lib:A-39`), and
  `INTERVAL<T>::contains` treating "not comparable" as *not contained* is now
  stated as a requirement rather than left implicit (`lib:D3.14a`).

## 0.5.0 — 2026-08-21

**A feature and a behaviour change, neither an API break.** 0.5.0 rather than
0.4.1 because cargo treats `0.4.x` as compatible: a dependent on `openehr =
"0.4"` picks up a patch on `cargo update`, and the rendering change below is
visible to anyone asserting on the text of a rendered query.


- **AQL accepts negative numeric literals** (`lib:Q12.9b`, closing `lib:A-27`).
  `WHERE o/value/magnitude > -2.5` — a base excess, a temperature difference, a
  scale scored below zero — parses. So does `MATCHES {-1, 0, 1}`.

  The sign is resolved by the parser at operand position, never by the number
  scanner, so an archetype id is unaffected:
  `openEHR-EHR-COMPOSITION.encounter.v1` begins with a letter and is scanned as
  a word that absorbs its own hyphens. `> -openEHR-EHR-…` is an error, not a
  guess. `LIMIT`/`OFFSET` refuse a sign deliberately and say why (`Q12.9d`).

- **A real numeric literal renders with a decimal point** (`lib:Q12.9e`).
  `Number(0.0)` rendered as `0` and reparsed as `Integer(0)` — a literal
  changing type across a round trip. Pre-existing; found by fuzzing the widened
  grammar above.

## 0.4.0 — 2026-08-21

**Breaking.** Two of the three items below change an API; the third raises the
minimum toolchain. Every affected line in a dependent is a **compile error**,
never a silent change in behaviour — which is the property that made the
`PartialOrd` removal safe to do at all.


- **MSRV raised from 1.90 to 1.95, and it is now a rule rather than a number:
  N−3, three Rust releases behind stable**
  (`spec/rust-msrv-n-minus-3/index.md` (superseded 2026-08-29 by `spec/rust-msrv-n-minus-2/index.md`)).

  Raising a floor is breaking for a user below it (`RV6`), so it is recorded
  here rather than left to be discovered by a build error. Cargo refuses with a
  clear message rather than miscompiling, so the damage is bounded.

  1.90 was never verified: no job had ever compiled this repository with a 1.90
  toolchain, and the claim was **false** for `openehr-loco`, whose framework
  requires 1.94. CI now derives N−3 from the stable toolchain it installs and
  builds and tests every crate on it (`spec/audit.md` **W-09**).

- **A runnable tutorial for the persistence layer**:
  `openehr-sqlite/examples/01_store_a_record.rs`, run by CI on every push. The
  five existing tutorials build and check documents in memory; this is the other
  half — install, commit, amend, read the history, resolve a point-in-time read,
  query the archetype index, watch a stale predecessor be refused, print a
  tamper-evidence checkpoint, and watch the database's own trigger refuse a raw
  `UPDATE` that went around the `Store`.

- **Criterion benchmarks** in `openehr` and `openehr-store`. A number from them
  is not a conformance claim and nothing is gated on wall-clock (`W0.34`,
  `W0.35`); CI runs them with `--test`, one iteration, so they cannot rot
  (`W0.36`).

- **`scripts/check-docs.py`**, run by the `claims` job: derives the crate count,
  the published version, the fuzz-target and tutorial counts, the CI job list,
  and every crate's conformance level from the tree, and fails when a document
  disagrees. Duplicated passages are bound to one owner with
  `<!-- shared: NAME (owner) -->` markers and compared byte for byte (`W0.38`).
  Three findings were drift of exactly this kind (**W-10**, **W-11**, **W-16**).

- **AQL string literals are no longer corrupted, and rendering no longer changes
  what a query asks** (`lib:A-37`, `lib:Q12.15`, `lib:Q12.15a`, `lib:Q12.15b`).

  The lexer copied a string literal one UTF-8 **byte** at a time, so `'Müller'`
  became `'MÃ¼ller'` and a `WHERE` against it matched nobody — the query parsed,
  checked clean, and was about a different string. Separately, the `FROM`
  renderer omitted parentheses its own grammar needs, so
  `(EHR e CONTAINS COMPOSITION c) OR EHR x` rendered as text that re-parsed to
  `EHR e CONTAINS (COMPOSITION c OR EHR x)` — a query over different records.
  Rendering also now escapes `'` and `\` in string literals.

  Not a breaking API change; a behaviour fix. Code that round-tripped a query
  through `to_string()` was getting a different query back, and now is not.

- **Known limitation, upstream: `serde_json` reads back a number it did not
  write** (`lib:A-38`). Its float parser is one ULP below `core::str::parse`
  for some inputs, so a magnitude **drifts** across repeated canonical-JSON
  round trips — three applications before it settled in the observed case, with
  no bound established. Reported upstream as
  [serde-rs/json#1336](https://github.com/serde-rs/json/issues/1336). **Stored bytes, and the
  content digest over them, are unaffected** — `db:M3.43` stores canonical JSON
  byte-preserving and the integrity check hashes the stored bytes rather than
  re-deriving them, so no false tamper alarm is reachable. Recorded rather than
  worked around; the fix is upstream.

- **The `agents/` directory is lowercase** (`AG1`, `spec/agents-directory-name-is-lowercase/index.md`).
  `AGENTS/` became `agents/`; the file `AGENTS.md` keeps its name, which is a
  cross-tool convention. Affects nobody depending on these crates.

- **BREAKING: no `DV_ORDERED` implements `PartialOrd` any more, and neither
  does `DATA_VALUE`** (`lib:D3.18b`, closing `lib:A-35`). Comparison is
  `DvOrdered::semantic_cmp`; `INTERVAL<T>` is bounded on the new
  `openehr::base::SemanticOrd` rather than on `PartialOrd` (`lib:D3.18c`).

  All ten types derived `PartialEq` over every field — including the
  `OrderedAttrs` each carries, and `DV_QUANTITY`'s `precision` and
  `units_display_name` — while comparing only the magnitude. So
  `5 mg precision 1` was `!=` to `5 mg precision 2` while `<=` and `>=` were
  both true of it, which is exactly what Rust's `PartialOrd` contract forbids.
  Invisible inside this crate; a wrong answer inside a caller's `binary_search`
  or `dedup_by`.

  **Migrating**: `a < b` becomes `a.semantic_cmp(&b) == Some(Ordering::Less)`,
  `a.partial_cmp(&b)` becomes `a.semantic_cmp(&b)`, and `DvOrdered` must be in
  scope. Every affected line is a compile error, never a silent change — the
  four crates in this repository that depend on `openehr` needed no edits at
  all. **No behaviour changed**: the comparison logic is the same logic, reached
  through a different name.

- **`DV_URI` and `DV_EHR_URI` are validated, and reading one no longer panics**
  (`lib:A-36`). A `DV_URI` deserialized from `{"value":"nocolon"}` panicked in
  `scheme()`; a `DV_EHR_URI` deserialized from `{"value":"https://…"}` reported
  scheme `https`, which is what that type exists to make impossible.

  `DvUri::scheme()` and `rest()` now return `""` where there is no scheme, and
  `validate()` reports `DV_URI.Value_valid`, `DV_URI.Uri_well_formed`, and
  `DV_EHR_URI.Scheme_valid` — including on `LINK.target`, on every `LOCATABLE`,
  which was validated nowhere. **Behaviour change for callers:** a document that
  previously validated clean and carried a malformed or foreign-scheme URI now
  reports violations, and code matching on `scheme()` for a value built by
  `Deserialize` gets `""` instead of a panic.

## 0.3.0

**Breaking.** No migration path exists or is planned before 1.0
(`db:O10.14`). A deployment on 0.2.0 exports its data, upgrades, recreates the
schema, and reloads.

- **`SCHEMA_VERSION` now exists, and is `4`.** A database written by 0.2.0
  records no schema version at all. `Store::install()` now refuses to open a
  *populated* database that has none, rather than guessing which schema it
  is (`db:O10.16`). A fresh, empty database installs normally.
- **`ColTy::Json` moved off `jsonb` (PostgreSQL) and `JSON` (MySQL) onto a
  byte-preserving text type** (`db:M3.43`, `db:D-08`). Both prior types
  normalise on the way in — reordering object keys, and on MySQL rewriting a
  magnitude of `1.10` as `1.1` — which changes the bytes a content digest was
  taken over. A database created under 0.2.0 has columns of the old type and
  already-normalised content; it cannot be upgraded in place.
- **Nine columns added to `openehr_version`**, carrying the tamper-evident
  hash chain (`db:D-07`). Absent from the 0.2.0 schema.
- **`ColTy::Digest` added.** `ColTy` is deliberately not `#[non_exhaustive]`,
  so any `Dialect` implementation outside this repository fails to compile
  against 0.3.0 until it handles the new variant — intentional, not an
  oversight.
- **`OriginalVersion::new` refuses input it previously accepted**
  (`lib:A-23`): a first version naming a preceding version, or a
  non-first version naming none, is now a construction error rather than a
  silently inconsistent value.
- **`Date`, `Time`, `DateTime`, and `Duration` no longer implement
  `PartialOrd`/`Ord`** (`openehr::base::iso8601`; `lib:A-32`). `Eq` on these
  types is lexical — two values are equal only when written the same way —
  while chronological (or, for `Duration`, length) order compares what the
  value denotes, and Rust requires `PartialOrd` to agree with `Eq` wherever
  it is implemented. It cannot, for either of these types, without giving up
  something load-bearing (record identity for `Eq`, or the query the ordering
  exists for), so the trait impl is gone rather than left contradicting
  itself. Callers using `<`, `<=`, `.partial_cmp()`, `.min()`, `.max()`, or
  `sort()` directly on these four types call the new inherent method
  `.semantic_cmp(&self, other: &Self) -> Option<core::cmp::Ordering>`
  instead. **This does not affect** `DvDate`, `DvTime`, `DvDateTime`, or
  `DvDuration` in `openehr::rm::data_types` — their own `PartialOrd` impls are
  unchanged and still work with `<` and friends; only the four bare ISO 8601
  types lost the trait.

**Fixed**, not breaking:

- A normal range on a `DV_DATE`, `DV_TIME`, `DV_DATE_TIME`, or `DV_DURATION`
  was silently unreachable by path and never contributed to `is_abnormal()`
  — the four temporal types were missing from the internal list of classes
  carrying `DV_ORDERED` attributes, despite implementing it (`lib:A-29`).
  They now behave as documented; no signature changed.

See [`spec/audit.md`](spec/audit.md) and [`openehr/spec/audit.md`](openehr/spec/audit.md)
for the full findings this release closes, and
[`agents/publishing.md`](agents/publishing.md) for the publishing process
itself.

## 0.2.0 and earlier

Not tracked here. See the git history and crates.io.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
