# Ambiguities register

**What this is.** SEC asked (thread #18, #19) that spec silences and
contradictions not be discarded once a project has worked around them —
this repository had been adjudicating them privately, inside its three audit
registers ([`audit.md`](audit.md), [`spec/audit.md`](../../spec/audit.md),
[`spec/databases/audit.md`](../../spec/databases/audit.md)), and reporting
none of it upstream. This file is the first half of closing that gap: one
place collecting the cases where the specification is silent, or disagrees
with itself, or disagrees with another openEHR repository claiming the same
authority — with the disposition this project gave each one and a citation
back to the fuller account.

**What this is not.** Not a new adjudication process, and not the second
half of the task that named it — filing one issue per entry against an
openEHR tracker is a maintainer action taken on the project's behalf
(`GOVERNANCE.md` §Machines do not decide, the same footing as posting to the
FerroEHR thread itself), not something run here. Each entry below says
**Filed upstream: not yet** until that happens; when it does, the entry
gains the issue link and this line changes, rather than either being
deleted or silently going stale.

**Scope.** Five entries, the ones named when this register was proposed —
not an attempt at the full count SEC's own framing implied. More belong
here as they turn up; a partial register that says so is the honest
starting point, not a reason to wait for a complete one that would take
much longer to assemble and be stale before it was finished (`W0.4`).

## The `TERM` repository disagrees with the computable one

**The specification, or what stands in for it, is inconsistent with
itself.** Two openEHR repositories both present themselves as *the*
openEHR terminology: `openEHR/terminology` (the older one, still online)
and `openEHR/specifications-TERM` (the current, machine-readable one). They
do not agree. The older copy gives code `435` for `episodic` where the
current one gives `451`, and omits `815|report|`, `816|restoration|`, and
`817|format conversion|` from the change types entirely.

**Disposition.** [`openehr::terminology`](../src/terminology.rs) transcribes
from `specifications-TERM`'s `computable/XML/en/openehr_terminology.xml`
(read 2026-07-31), named as the normative machine-readable artifact, and
says explicitly that it is *not* derived from the older repository, with
the specific codes that disagree recorded in the module's own
documentation — a crate seeded from the wrong copy would reject valid
instances and mint invalid ones. `openehr/spec/releases.md` carries the
provenance date; a re-vendor check would re-read `specifications-TERM`,
never the older repository.

**Filed upstream:** not yet.

## `ARCHETYPE_HRID` vs `ARCHETYPE_REF` vs this crate's own `ArchetypeId`

**The specification uses three different tokens for what looks like one
concept.** ADL's own grammars distinguish an archetype header's own
identifier (`ARCHETYPE_HRID` — `adl14.g4`'s `archetype` rule, `adl2.g4`'s
`authored_archetype` rule) from the identifier a *specialization* names for
its parent (`ARCHETYPE_REF` in ADL 1.4's `specialization_section`; ADL 2's
own `specialize_section` accepts *either* token,
`archetype_ref: ARCHETYPE_HRID | ARCHETYPE_REF`). The two are not
interchangeable: `ARCHETYPE_HRID` allows an optional `namespace::` prefix
and a bounded, three-part version with prerelease suffixes;
`ARCHETYPE_REF` allows the same namespace prefix but an *unbounded* chain
of `.DIGIT+` version segments and no prerelease suffix. This project's own
pre-existing `base::ArchetypeId` type is close to neither — it lacks the
namespace prefix both grammars allow, and caps the version at three parts
where `ARCHETYPE_REF` does not — found while implementing `ARCHETYPE_HRID`
as its own type and checking the claim against `openEHR/adl-antlr`'s real
grammar files rather than assuming the type this crate already had was what
the header actually names (`lib:A-49`).

**Disposition.** `openehr::am::ArchetypeHrid` was added as its own type,
parsed against `ARCHETYPE_HRID`'s exact grammar, and both `Adl14Header` and
`Adl2Header` now hold one rather than an `ArchetypeId`. Reconciling
`ArchetypeId` itself against `ARCHETYPE_REF`, or building a true union type
for ADL 2's `archetype_ref` rule, was explicitly **not** undertaken in the
same pass: `ArchetypeId` is a `base` type used pervasively across Reference
Model data (`ARCHETYPED.archetype_id`), so widening it is a larger,
separate, more consequential change than adding a narrowly-scoped new type
was. `specializes` on both header types is therefore unchanged —
`Option<ArchetypeId>` — and does not yet accept the wider `ARCHETYPE_REF`
form. The residual is documented rather than silently left, in `lib:A-49`
itself.

**Filed upstream:** not yet.

## `C_STRING`'s regex-in-list shape had no counterpart in this crate

**A field this crate invented reads as spec, when it is not.**
`org.openehr.am.aom2.c_string.adoc` gives `C_STRING` exactly one
constraint attribute: a single `List<String>`, "a list of literal strings
and/or regular expression strings delimited by the `/` character." This
crate's own `CPrimitive::String` instead carried two separate fields —
`list: Vec<String>` and `pattern: Option<String>` — inherited by
resemblance from the four `C_TEMPORAL` variants, where a *separate*
`pattern_constraint` genuinely is its own AOM2 attribute with its own
`Pattern_validity` invariant. `C_STRING`'s own class definition was never
checked separately, so the fifth type read as right because the other four
were (`lib:A-63`).

**Disposition.** `pattern` was removed from `CPrimitive::String`; `list`
now carries literal and regex elements together, exactly as AOM2 states.
`am::constraint::is_c_string_pattern` recognises the delimiter convention
(`/…/` or `^…^`, matching `CONTAINED_REGEXP`'s own grammar in
`openEHR/adl-antlr`'s `base_lexer.g4`) and is the *only* place that does.
One interpretive decision beyond the type change, not mandated by AOM2
either way: when a literal element in the list matches a value but a
pattern element also sits in the same list, the node stays `Unchecked`
rather than being promoted to `Conforms` — the same choice this crate made
when the two lived in separate fields, kept rather than silently changed
now that they share one. Actually compiling and applying a regex element
is unaffected by this and remains open (`K15.18`).

**Filed upstream:** not yet.

## `is_modifiable`'s evaluation time against a reactivating commit

**The specification says an `EHR` may be deactivated and that a
deactivated `EHR`'s content becomes read-only, but is silent on the
ordering question a real commit sequence raises**: if a single
`CONTRIBUTION` both reactivates a deactivated `EHR` (setting
`EHR_STATUS.is_modifiable` back to `true`) and commits new content in the
same change set, is the content refused because the `EHR` *was* deactivated
at the start of the operation, or admitted because it *is not* deactivated
by the time the rule is checked? Thread #5 found the sequencing bug in
implementations that check the flag's value from before the same
contribution's own reactivation; thread #6 published the adjudication with
citations, and reported the distinct `versions` typing defect it surfaced
upstream to FerroEHR's own tracker as their `#2674` (this project's own
tracking of the sequencing question itself is FerroEHR's `#2673`).

**Disposition.** Refuse content members only when the `EHR` is deactivated
**and** the contribution being committed does not itself reactivate it —
order-independent, evaluated against the state the contribution leaves the
`EHR` in rather than the state it found it in. **Not yet implemented**:
`tasks.md`'s own backlog carries this as an open item, re-scoped 2026-09-06
once the prerequisite it did not originally name — versioned `EhrStatus`
persistence, since `Ehr.ehr_status` is only an `ObjectRef` today and
`openehr-store`'s `Store` trait has no method to create, commit, or read an
`EhrStatus` version at all — turned out close in size to the composition-
versioning work already done, not a small addition to it. No `db:H5.x`
identifier has been allocated yet; one is reserved for the commit rule once
it is written down as a requirement rather than only as a backlog
adjudication.

**Filed upstream:** not this project's own tracking (`#2673`) — filed by
thread #6 already, hence a number to cite rather than a gap to fill.

## `VERSIONED_OBJECT`'s `versions` is typed a List but described as a Set

**The Reference Model types a version container's contents as an ordered
collection while its own prose describes set semantics** — no two versions
share an identifier, and nothing in the model's intent depends on list
position beyond commit order, which a `List` can express but does not by
itself guarantee against a duplicate. FerroEHR reported this upstream
as `#2674`, surfaced while adjudicating the `is_modifiable` sequencing
question above (thread #6).

**Disposition.** This crate follows the Reference Model's literal typing
rather than taking an independent position: `VersionedObject<T>::versions`
is `Vec<Version<T>>`, and order is treated as significant — `all_versions`
returns oldest first (`db:H5.12`, already tested against
`openehr-sqlite`), which only makes sense of a `List`, not a `Set`. No
independent de-duplication guarantee beyond what the commit rules already
enforce (a duplicate or mis-parented version is refused at commit,
`db:H5.8`) has been added on the strength of this ambiguity; the type
itself is not widened or narrowed pending FerroEHR's own upstream report
being resolved.

**Filed upstream:** not this project's own tracking — FerroEHR's `#2674`
already exists; this entry cites it rather than duplicating it.

---

Part of the [openEHR library specification](index.md).
