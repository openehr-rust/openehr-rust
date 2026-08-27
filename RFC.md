# Request for comments

This project would rather be corrected than complimented. What follows is what
it does not know, stated specifically enough that someone can disagree with it —
because "feedback welcome" attached to nothing is a way of receiving none.

**The standing invitation: tell this project something it claims that is not
true.** A correction that survives checking becomes a numbered finding in
[`spec/audit.md`](spec/audit.md) or
[`openehr/spec/audit.md`](openehr/spec/audit.md), with your report cited as its
evidence, and every finding there so far was found by running something rather
than by reading it.

## What this project wants to learn

### 1. Is the storage model right for a real deployment?

Records are stored as byte-preserving canonical JSON with an index projected
alongside (`db:M3.43`), and **not** shredded into per-field tables (`db:M3.4`,
withdrawn: shredding). That decision was made for digest stability and against
query convenience.

- Does it fall down at a volume or query shape this project has not tried?
- Is the projected index the right set of columns for the queries you actually
  run, or is it the set that was easy to project?
- Times are two columns — an authoritative exact text and a derived nullable UTC
  — because `2024-05` is a date known to the month. Does that survive contact
  with your reporting stack, or does everyone immediately collapse it?

### 2. Do the dialects match how these engines are actually administered?

Five of six are below **Store** level, and the DDL has been reviewed by its
author and, for three engines, executed by a server. That is a low bar for
something a DBA has to live with.

- Partitioning, tablespaces, collation, and character-set choices: what is
  missing that your organisation would require before running this?
- Nothing indexes a `LongText` or `Json` column, deliberately, because no engine
  here can search one without the adjuncts in
  [`spec/databases/search-adjuncts.md`](spec/databases/search-adjuncts.md) —
  and none is emitted (`db:P6.18`). What would you need emitted?
- SQL Server and Oracle DDL has never been parsed by a server. If you have one,
  the output of `cargo run --example ddl` piped into it is the single most
  useful thing anyone could send.

### 3. Is the conformance ladder legible, or just unusual?

**Dialect / Schema / Store / Verified** exists so that "supports six databases"
cannot be said. It costs every document a qualification.

- Reading [`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md)
  cold: can you tell what you would be adopting?
- Does the ladder help you make a decision, or does it read as excuse-making?
- Is there a rung missing between **Schema** and **Store** — for instance,
  "a `Store` exists and the conformance suite passes, but not in CI"?

### 4. Was `Real` instead of `f64` the right call, and does it break you?

Since 0.6.0 the Reference Model's reals preserve the digits they were written
with, so `1.50 mg` and `1.5 mg` are different records that hash differently
(`lib:D3.18d`). The reason is `db:D-08`: MySQL's `JSON` type rewrote a stored
`1.10` as `1.1`.

- Does a downstream system of yours now see digests it did not expect?
- Is the `f64` accessor pair (`magnitude()` and `magnitude_real()`) the right
  ergonomics, or does it invite the wrong one being used?
- Is there a case where preserving the text is actively wrong?

### 5. Is removing `PartialOrd` from every `DV_ORDERED` painful in practice?

`a < b` does not compile on `DvQuantity`; `DvOrdered::semantic_cmp` is the
route, and `Interval<T>` is bounded on `SemanticOrd` with **no blanket impl**
(`lib:A-35`, `lib:D3.18b`, `lib:D3.18c`). Equality is record identity, ordering
is by magnitude, and Rust does not permit both through the standard traits.

- How much does this cost you in ordinary code?
- Did you hit a type that needs `SemanticOrd` and could not get one?
- Is there a formulation that keeps the guarantee and reads better?

### 6. The Archetype Model: what should exist first?

`lib:S1.4` excluded it; the exclusion was withdrawn on 2026-08-26 and
[§15](openehr/spec/15-archetypes.md) now specifies it. `openehr::am` is the AOM2
object model; 28 requirements have no code (`lib:A-40`).

- **Which comes first for you: validating data against an operational template
  you already have, or parsing ADL to produce one?** Deployed tooling emits
  OPT 1.4; ADL 2 is the authored form. The order this gets built in should be
  decided by what implementers actually need, and that is a question this
  project cannot answer from the inside.
- `K15.20` refuses to pass a node whose constraint uses something unimplemented.
  Is "unchecked, therefore not conformant" workable in your pipeline, or does it
  make partial validation useless to you?
- Retrieval (`K15.24`–`K15.27`) is specified to live outside `openehr`, so the
  library performs no I/O. Does that split match how you would use it?

### 7. What would make this adoptable where you work?

Not features — assurances. Signed commits and releases (there are none), an
SBOM (there is none), a
second maintainer, a licence answer for your legal team, an MSRV that is not
N−3. (GitHub private vulnerability reporting, listed here as disabled when
this file was written, was enabled 2026-08-26.) [`SECURITY.md`](SECURITY.md) now states the disclosure window and lists
this project's own gaps.
**Which of those is the blocker, and which is a nice-to-have?** The answers
determine what gets built next as much as any feature request.

### 8. The open findings, if any of them affect you

Six are open in the library register, and each is open by a stated reason rather
than by omission. If one of them is a real problem for you rather than a
theoretical one, say so — that changes its priority:

| Finding | What is open |
| --- | --- |
| `lib:A-02` | ISO 8601 basic format (`20240501`) is refused, by decision |
| `lib:A-05` | AQL re-rendering differs cosmetically from its input |
| `lib:A-08` | The `property` and `extract_*` terminology groups are not carried |
| `lib:A-10` | `X11.24`'s fail-closed default has no provokable error path to test |
| `lib:A-30` | AQL has no node-id predicate shorthand: `c[at0001]` is refused |
| `lib:A-40` | The Archetype Model is specified and 28 of 32 requirements have no code |

### 9. Naming: this project uses the openEHR mark, and is not openEHR

The organisation, the repository, and the crate names all carry "openehr", and
the project is not affiliated with, endorsed by, or certified by openEHR
International. Non-affiliation notices are being added across the root
documents and crate rustdoc, but a notice is the floor, not the answer.

- Is the naming itself misleading to you, notice or no notice? A procurement
  reader who sees `openehr = "…"` in a manifest may not read a README.
- Should this project contact openEHR International about the name before any
  public announcement, rather than after? If you have been through this with
  another standards body's mark, what did the conversation look like?

**Resolved 2026-08-27.** openEHR granted this project permission to use their
trademarks — owner-reported, with the correspondence held by the maintainer;
[`TRADEMARKS.md`](TRADEMARKS.md) §Permission is the record. The second
question answered itself in the right order: the contact happened before any
public announcement. The notices stay, unchanged — permission is not
affiliation — and adopting any permission-referencing wording awaits the
grant's own terms. The section is kept because the first question, whether
the naming reads clearly to an outside evaluator, remains worth an outside
answer.

### 10. Funding: is "no funding vehicle" a signal or a smell?

[`CONTRIBUTING.md`](CONTRIBUTING.md) §Money says it plainly: no Sponsors, no
Open Collective, no entity, no account — and argues that saying so beats an
unmaintained sponsor button. `.github/FUNDING.yml` would therefore be a
decision to create a vehicle, not a missing file.

- Does the absence read to you as honesty, or as a project that will be gone
  in a year?
- If your organisation wanted to pay for an engine to reach **Store** level,
  does the commercial-engagement route in CONTRIBUTING.md actually work for
  your procurement, or does it need a shape (invoiceable entity, support
  contract) that does not exist here?

### 11. Does the project need a website, or is the repository enough?

There is no landing site; sibling projects use an in-repo static site, and the
outreach plan currently presumes one exists. Standing one up is maintenance
surface forever.

- Did you, evaluating this, need anything a README could not give you?
- Is docs.rs plus the repository sufficient for the audience you sit in, or
  does "no website" fail a filter before a human ever looks?

### 12. One conformance matrix is machine-derived; the other is hand-assessed

The library matrix is re-derived mechanically and its totals are CI-checked —
that machinery exists because the totals went stale twice (`lib:A-41`). The
databases matrix, [`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md),
is hand-assessed apart from one CI check, and it is the document adoption
decisions are supposed to rest on.

- Does the asymmetry bother you? Would you trust a hand-maintained cell that
  says **Verified**?
- Machine-deriving it is a real cost — evidence per engine is a live server
  run, not a `grep`. Is there a cheaper check that would still catch a stale
  cell, short of full derivation?

## What kind of feedback helps

**These land well**, in rough order of value:

- A reproduction: input, expected, observed, and the version.
- A server transcript. What PostgreSQL 18 or Oracle actually said.
- A specification citation showing this project has read openEHR wrong — file
  and section, so it can be checked rather than debated.
- A test that fails on `main`.
- "We tried to adopt this and stopped because X." X is the finding.
- A measurement taken on hardware you name.

**These are harder to act on**, and it is fairer to say so than to let them sit:

- A feature request with no use case behind it. The scope exclusions in
  [`openehr/spec/01-scope.md`](openehr/spec/01-scope.md) are decisions with
  reasons; the way to move one is to show the reason is wrong, as happened to
  `S1.4`.
- A benchmark on unspecified hardware, or a comparison against another
  implementation run by someone with an interest in the result. See
  [`BENCHMARKS.md`](BENCHMARKS.md) for what this project will and will not say
  about performance.
- Style preferences. The house style is heavy comments that explain *why*, and
  it is deliberate.
- "Support X too" where X is a standard this project has explicitly not
  implemented. Say what you need X *for*.

## How to send it

| Route | Best for |
| --- | --- |
| A GitHub issue on [`openehr-rust/openehr-rust`](https://github.com/openehr-rust/openehr-rust/issues) | anything checkable — cite the requirement id if you have it |
| The openEHR Discourse [Implementation category](https://discourse.openehr.org/c/implem/39) | anything the wider openEHR community should see rather than one maintainer |
| joel@joelparkerhenderson.com | security-relevant reports under [`SECURITY.md`](SECURITY.md) — synthetic reproductions only, never patient data; and commercial questions |

## What happens to it

- A report that is right and fixable becomes a commit, with the requirement
  written first (`W0.19`).
- A report that is right and **not** fixable now becomes a numbered finding with
  evidence, because a gap that is not written down reads as a pass (`W0.4`).
- A report that is wrong gets an answer saying why, with the citation.
- A report that shows a *requirement* is wrong is the most valuable of the four,
  and the specification gets amended — in place, keeping its identifier, saying
  it was amended and why (`C0.15`, `C0.16`). `S1.4` is the worked example: an
  exclusion that stood since the first release, withdrawn with its reasoning
  kept, because the reasoning is what the replacement had to answer.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
