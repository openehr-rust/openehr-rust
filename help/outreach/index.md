# Promoting openehr-rust to professionals

**Not normative** (`W0.2`). This document allocates no requirement identifiers
and constrains no code; it is a research note and a plan. Where it and a
specification disagree, the specification governs.

**Researched 2026-08-25.** Every channel, deadline, address, and submission rule
below was checked on that date, and each carries its source at the foot of the
document. Other people's conferences, editors, and forum categories change
without telling us, so **re-check before acting** — a promotion plan is the one
kind of document here whose facts rot on someone else's schedule, which is the
same property [`rust-msrv-n-minus-3/index.md`](../../spec/rust-msrv-n-minus-3/index.md)
writes down about the MSRV.

## 1. The one rule that governs every word of the copy

`W0.3` — never claim more than is verified — is a repository rule, and
promotional copy is where it is most tempting to break and least likely to be
caught. `python3 scripts/check-docs.py` reads the Markdown in this tree. It does
not read a forum post, a LinkedIn update, or a press release, and none of those
can be corrected once a reader has quoted them.

The conformance ladder is **the differentiator**, not a caveat to be minimised.
Anyone can write "supports six SQL databases". Almost nobody can say which of
those six a server has actually executed, twice, and observed refusing an
`UPDATE`. That sentence is worth more to a professional evaluator than the six.

| Do not write | Write instead |
| --- | --- |
| "Supports PostgreSQL, MySQL, MariaDB, SQL Server, Oracle, and SQLite." | "Emits DDL for six engines. SQLite is at **Verified** — the full store, re-checked in CI on every commit. PostgreSQL, MySQL, and MariaDB are at **Schema**: a real server executed the DDL. SQL Server and Oracle are at **Dialect**: no server has parsed it yet." |
| "Production-ready openEHR persistence." | "One embedded store you can run today; five dialects that need an implementor." |
| "Fast." | Nothing. No benchmark here asserts a wall-clock number, deliberately (`W0.35`, `W0.36`). |
| "Safe for patient data." | Never. That is a regulatory claim about a deployment, not a property of a crate. |

Evidence that **is** citable, because something ran:

- `openehr-sqlite` at **Verified**, per
  [`spec/databases/conformance-matrix.md`](../../spec/databases/conformance-matrix.md),
  which owns every level claim (`W0.40`).
- `db:D-08` — MySQL's `JSON` type rewrote a stored magnitude of `1.10` as `1.1`.
  That is a measured clinical precision loss in a shipping database, found here,
  and it is the most interesting thing this repository can tell a stranger.
- `lib:D3.18d` — the Reference Model's reals are `base::Real`, so `1.50 mg` and
  `1.5 mg` remain different records and hash differently.
- `lib:A-35` — no `DV_ORDERED` implements `PartialOrd`, on purpose, and the
  reasoning is written down.
- The audit register itself. A published list of your own defects
  ([`spec/audit.md`](../../spec/audit.md)) is unusual enough in this field to be
  persuasive on its own to the kind of reader worth having.

## 2. Readiness — what a professional checks in the first sixty seconds

Promotion converts curiosity into evaluation. Evaluation begins at the README
and ends at "who else uses this?", and each of the following was true of the
repository on 2026-08-25.

| Gap | Fix | Why it matters before, not after |
| --- | --- | --- |
| [`README.md`](../../README.md) line 70 says `openehr = "0.2"`; crates.io is at 0.6.0 | Update the install block | It is the first thing a reader copies, and it is wrong. Nothing catches it: `check-docs.py` checks fixed-form *sentences* about the published version, not a TOML snippet |
| ~~No `SECURITY.md`~~ — added 2026-08-26; ~~no `CODE_OF_CONDUCT.md`~~ — added 2026-08-26 | Done | A clinical-data library with no disclosure address fails the first question a hospital's security reviewer asks, and both files are what OpenSSF-style checklists look for. `SECURITY.md` also states the project's own posture gaps — unsigned commits, no SBOM; private reporting was disabled until 2026-08-26 and is now enabled — which a reviewer will find anyway |
| No repository topics on GitHub | `openehr`, `ehr`, `healthcare`, `interoperability`, `rust`, `aql`, `sqlite` | `github.com/topics/openehr` is how this community browses. A repository with no topics is not in the room |
| No project website; three stars, one contributor, public history beginning 2026-08-01 | Nothing to fix — know it | The bus-factor question is the first one a vendor asks. Answer it plainly rather than being surprised by it |
| Five-way licence disjunction (MIT, Apache-2.0, BSD-3-Clause, GPL-2.0, GPL-3.0) | One sentence in the README: "take whichever you need; most take MIT or Apache-2.0" | Permissive in effect, unusual in form. An unexplained licence list becomes a legal review, and a legal review becomes a delay |
| No statement of how the code was written | Add `AI_STATEMENT.md` | See below. This is not optional in this particular community, this particular month |

**On the AI statement.** On 2026-08-24, FerroEHR — a Rust openEHR CDR —
was announced on the openEHR forum, and a respondent specifically praised "the
transparency with the AI_STATEMENT" published in its repository. This repository
carries [`AGENTS.md`](../../AGENTS.md), [`CLAUDE.md`](../../CLAUDE.md), and an
[`agents/`](../../agents/index.md) directory: agent involvement is obvious to
anyone who clicks, and the only question is whether they learn it from us or
notice it themselves. Write the statement before the first announcement, say
what is machine-written and what is human-reviewed, and point at the audit
register as the evidence of review. *This Week in Rust* separately requires
disclosure of machine-generated articles, so the same file settles that channel
too.

## 3. The landscape you are announcing into

openEHR is a **small, high-context professional community**. The same dozen
names answer nearly every implementation thread — Thomas Beale, Ian McNicoll,
Pablo Pazos, Sebastian Iancu, Borut Jures, and others. Reputation compounds
across threads and years, and one over-claim is remembered by everyone at once.
This is the opposite of the Rust channels, where an audience is anonymous and
renewed weekly.

Three pieces of prior art shape how a Rust announcement will land:

- **The 2023 `ehrust` thread**
  ([discourse.openehr.org/t/3489](https://discourse.openehr.org/t/openehr-api-implementation-in-rust/3489)).
  A developer proposed a Rust CDR; the response was cautiously supportive and
  architecturally sceptical. Thomas Beale questioned whether "Rust is a really
  good fit for meta-models (which tend to contain inheritance)". **Have the
  answer ready**: this repository does not implement the Archetype Model at all
  (`lib:S1.4`), which removes the inheritance-heavy meta-model from the problem
  and is a deliberate scope decision rather than an omission.
- **FerroEHR**, announced 2026-08-24 by Ruben Talstra: a Rust CDR implementing
  ITS-REST 1.1.0 and AQL 1.1 on PostgreSQL 18, with a live sandbox, Docker
  Compose, and roughly 1,100 conformance test cases. The reception was warm and
  technically demanding. **This is not a competitor** — they ship a server, this
  ships libraries and six SQL dialects, and a server needs exactly what a library
  crate provides. Email them **before** announcing. Two Rust openEHR projects
  that appear together read as an ecosystem; two that appear a week apart with no
  mutual acknowledgement read as a fork.
- **openEHR-CLI** and the **openEHR MCP server** — recent tool announcements that
  landed well. Both are short, sectioned, link-first posts with a demo.

The incumbent implementations are EHRbase (open source, Germany), Better
(Slovenia), and Ocean Health Systems (Australia); Medblocks runs training and a
large developer Slack. None of them is a rival to a Rust library crate. All of
them are potential consumers of a byte-preserving canonical-JSON storage model.

## 4. Who "professionals" means here

| Segment | Where they are | What they need to hear | Proof they will demand |
| --- | --- | --- | --- |
| openEHR implementers and CDR engineers | Discourse, Medblocks Slack, EHRCON | "Persistence you can reuse; the precision bugs are already found" | The conformance matrix, `db:D-08`, the fuzz targets |
| Health-system architects (NHS, Nordic regions, German university clinics) | LinkedIn, affiliate groups, national events | "An auditable, specification-traced implementation with no vendor attached" | Who else runs it; licence; maintenance story |
| Rust engineers in regulated domains | r/rust, TWiR, conference talks, HN | "What a specification-first Rust codebase looks like when the domain punishes guessing" | The `PartialOrd` removal, `Real` over `f64`, zero-warning tree |
| Academic health informatics | MIE, MEDINFO, JAMIA/IJMI, JOSS | "A citable, reproducible reference implementation" | A paper, a DOI, six months of public history |
| Database and embedded builders | crates.io, lib.rs, HN | "openEHR in a SQLite file — offline clinics, edge devices" | A working example; the Verified level |

## 5. Channel A — the openEHR community (highest value per hour spent)

### 5.1 Discourse

[discourse.openehr.org](https://discourse.openehr.org) is the centre of gravity.
Categories that matter, and which one to use:

| Category | Use it for |
| --- | --- |
| [Implementation](https://discourse.openehr.org/c/implem/39) | **The announcement.** This is where implementations are discussed and where both Rust threads live |
| [Tool Support](https://discourse.openehr.org/c/tool-support/29) | Follow-ups about tooling and modelling support |
| [Software Program — Open](https://discourse.openehr.org/c/spb/152) | Introducing the project to the Software Program Board's orbit; the "DRAFT: openEHR Tooling and Software overview" thread is where a listing gets argued into existence |
| [Integration](https://discourse.openehr.org/c/integration/100) | Anything touching FHIR, HL7, or EHDS |
| [Regional Communities](https://discourse.openehr.org/c/reg-com/110), [Affiliates](https://discourse.openehr.org/c/openehr-affiliates/11) | Local follow-up after the international post |
| [openEHR News](https://discourse.openehr.org/c/openehr-news/9) | **Not yours.** It is announcements from openEHR International leadership |

**Earn the post before making it.** Read for a week; answer two or three other
people's questions substantively. A first post that is an announcement from an
account with no history is the weakest possible version of the same content.

**Anatomy of an announcement that works here**, taken from what the FerroEHR and
openEHR-CLI threads actually contained and what they were asked afterwards:

1. One sentence on what it is and what it is not (libraries and dialects, not a
   CDR; no Archetype Model).
2. The conformance table, levels included, with the matrix linked.
3. A runnable thing in under five minutes — the SQLite store, a `cargo run`
   tutorial, the DDL example.
4. Licence, stated plainly.
5. The AI statement, linked.
6. An explicit ask: which dialect do you need promoted, and would you review the
   DDL for your engine?

Expect these questions, because they were asked of FerroEHR within a day:
handling of a spec edge case with a precise citation, benchmarks against
EHRbase, licence terms of vendored artefacts, and the risk of building against
development-generation specifications. Answer the benchmark question by refusing
it honestly — this tree runs benchmarks and asserts nothing about wall-clock
(`W0.35`), and saying so is a better answer than a number from a laptop.

### 5.2 The openEHR Software Program

The Software Program Board launched in 2026 to "coordinate, support, and
promote software developments within the openEHR ecosystem", and takes
applications for board participation via the Discourse thread
[t/11830](https://discourse.openehr.org/t/application-to-the-software-program-board/11830).
The official
[Libraries page](https://openehr.org/products_tools/libraries/) currently lists
only Archie, the Atomik SDK, and EHR Craft, and publishes **no submission
process** — so the route is the Software Program category plus the site's
contact form. Getting onto that page is a small, durable, high-signal win: it is
the page procurement documents cite.

### 5.3 Events, with dates

| Event | When and where | What to do |
| --- | --- | --- |
| **EHRCON26** — annual openEHR International conference | 22–23 September 2026, Meervaart Theatre, Amsterdam (pre-conference clinical modelling workshop 21 September) | The programme is already live, so the speaking slot for 2026 is gone. **Attend.** Four weeks of preparation buys a year of introductions; the themes are interoperability, EHDS, sustainability, and agentic AI, and the third and fourth are both hooks this project can speak to |
| **Collabrathon** | from 5 November 2026, hybrid, two days, teams building an International Patient Summary solution in 24 hours | The best possible fit for a library. Offer the crates as tooling, sit in the channel, and fix what breaks live. Nothing else on this list produces feedback that fast |
| Affiliate meetups | Rolling — affiliates exist for Brasil, Finland, Germany, Japan, Netherlands, Oman, Portugal, Spain, Sweden, Switzerland, the UK, the USA, and Life Sciences, with Australia, New Zealand, Belgium, Ireland, Norway, Poland, and Italy in progress | Local groups give talk slots that the international conference does not. Pick two where openEHR is already deployed at scale |

### 5.4 Medblocks

[medblocks.com/community](https://medblocks.com/community) runs a Slack for
healthcare developers working on openEHR and FHIR, and an openEHR bootcamp whose
cohorts ship real projects. A bootcamp cohort is a stream of new implementers
looking for exactly this kind of tooling, and they are the audience least
committed to an existing stack.

## 6. Channel B — the Rust community

### 6.1 r/rust first, because *This Week in Rust* reads it

TWiR **no longer accepts pull requests for the Project/Tooling Updates
section**; its editors monitor r/rust and pick from what is posted there. So one
r/rust post seeds two channels. Post the release, not the repository: r/rust
rewards a specific technical claim over a project pitch.

### 6.2 This Week in Rust

- **Crate of the Week**: nominations and votes go in the weekly nomination
  thread on the TWiR repository.
- **Blog posts, CFPs, and event listings**: still submitted as a pull request
  against the file in `drafts/` in
  [rust-lang/this-week-in-rust](https://github.com/rust-lang/this-week-in-rust).
- Machine-generated articles **must** be disclosed.

### 6.3 The essay, and Hacker News

The strongest story here is not "openEHR in Rust" — that interests a hundred
people. It is one of:

- **"MySQL silently rewrote a clinical measurement of 1.10 as 1.1"** (`db:D-08`).
  A reproducible, consequential, checkable finding about a database everyone
  uses, in a domain where the digit matters.
- **"We removed `PartialOrd` from every ordered type in a medical data model"**
  (`lib:A-35`), on why `==` over all fields plus ordering over one field is a
  trap, and why making `==` semantic would have reintroduced `db:D-08`.
- **"A path that resolves to nothing is not an error"** — the navigation-table
  silence that hid fifty untested match arms (`lib:A-28`).

Write it on a blog, post it to r/rust, submit it once as a **Show HN**, and let
it carry the crates. Lobsters is invite-only and hostile to self-promotion
without standing; do not start there.

### 6.4 Podcasts and stages

- **Rust in Production** (corrode.dev), biweekly, welcomes guest suggestions —
  but its frame is companies running Rust in production. Pitch it **after** a
  first deployment exists, not before.
- **Rustacean Station** takes community episode proposals with a lower bar.
- **FOSDEM 2027**: devroom calls for participation open around September 2026
  and individual talk CFPs run November–December. There was no dedicated
  healthcare devroom in 2026, so target an adjacent one — Rust, databases, or
  open research — or watch for a health devroom proposal.
- **RustConf**, **EuroRust**, **Rust Nation UK**: "Rust in a domain that
  punishes guessing" is a talk shape these programmes reliably accept.

### 6.5 Discoverability plumbing

crates.io keywords and categories today: `openehr` carries
`["openehr", "ehr", "healthcare", "archetype", "aql"]` in `data-structures`,
`science`, and `encoding`; the store and dialect crates carry `database`.
Categories aid browsing, keywords feed search ranking, and five keywords is the
cap — so spend them on words a searcher would actually type
(`interoperability`, `hl7`, `clinical`) rather than on repeating the crate name.
Add the repository to `awesome-health`, `awesome-healthcare`, and the relevant
Rust lists by pull request; each is a one-hour, permanent, zero-risk backlink.

## 7. Channel C — press and public relations

**The honest position: trade press covers deployments, money, and policy, not
libraries.** A pitch about a three-week-old crate with three stars will not run,
and burning an introduction on it costs more than the coverage would earn. Get a
named user first; the story is then "*X* built *Y* on it", which is a story.

When there is one:

| Outlet | Route | Notes |
| --- | --- | --- |
| **HTN Health Tech News** (UK) | `press@htn.co.uk` | Explicitly invites story submissions for the daily and weekly round-ups. The lowest-friction outlet on this list |
| **Digital Health** (digitalhealth.net) | Editorial contact form | The UK health-IT trade paper; NHS openEHR deployments are its beat |
| **Healthcare IT News** (HIMSS) | Editorial contacts, journalist-by-journalist | Large reach, interoperability is a standing topic; pitch a named reporter, never a general address |
| **Becker's Health IT** | Editorial submissions | US CIO audience |
| **Open Health News** | Accepts press releases | Specifically covers open source and open standards in health — the best fit for an announcement that is *only* an announcement |

A pitch is 120 words: one claim, one piece of evidence, one named human
available for interview, and no attachment. Do not buy wire distribution; in
this sector it produces republished noise and no readers.

**The policy hook** is EHDS — the European Health Data Space is on EHRCON26's
programme, and "open-source implementations of open standards" is the frame that
currently gets picked up. It is also a claim you can make without exaggerating.

## 8. Channel D — academic and standards literature

A citation is what makes a library quotable inside a procurement document, which
is the professional audience's real gate.

- **JOSS** (Journal of Open Source Software) requires **at least six months of
  public repository history** with steady, iterative development, plus evidence
  of research use — not a burst of commits. This tree's public history starts
  2026-08-01, so the earliest honest submission is around **February 2027**.
  Everything else JOSS asks for (tests, CI, documentation, a contributing guide)
  already exists here; the missing piece is time and a user. Start collecting
  the "used in research" evidence now, because that is the criterion that fails
  people.
- **SoftwareX**, **JAMIA Open**, and the **International Journal of Medical
  Informatics** take software and implementation papers with a longer form.
- **MIE 2027** — Medical Informatics Europe, 26–30 April 2027, Tallinn. CFP not
  yet published; watch [mie2027.efmi.org](https://mie2027.efmi.org/).
- **MEDINFO 2027** — 30 August to 4 September 2027, Dubai. The biennial world
  congress; abstract deadlines typically fall six to nine months ahead.

## 9. Channel E — direct email

Small, individual, and specific. Eight to twelve messages, each written by hand,
each with a question rather than an announcement — "does your dialect handle a
partial date like `2024-05` as two columns, or one?" is an email that gets
answered; "here is my project" is not.

**Who**, in order of expected return:

1. The FerroEHR author — the single highest-value message on this list, and it
   should go out before any public announcement.
2. EHRbase maintainers and engineers at Better, Ocean, and Medblocks — each owns
   a persistence layer and has met `db:D-08` whether or not they know it.
3. Affiliate leads in countries with live openEHR programmes.
4. University groups publishing on openEHR persistence.
5. The named participants of the 2023 Rust thread, who already care about this
   exact intersection.

**Do not** scrape the forum for addresses, run a mail merge, or buy a list.
Beyond the GDPR exposure, this community would notice within a day — and a
public Discourse thread reaches every one of these people at once, on the
record, at a fraction of the cost. Direct mail is for the handful of cases where
you want a private answer or a collaboration.

## 10. Channel F — social

- **LinkedIn is the professional square for health IT.** openEHR International,
  the affiliates, the vendors, and NHS informatics leaders are all there and all
  post. Publish as a person, not as a project account: one long-form text post
  per release, one image (the conformance matrix reads well), a link in the
  first comment, tagging openEHR International, with `#openEHR`,
  `#digitalhealth`, `#interoperability`, and `#rustlang`.
- **Mastodon and Bluesky** for the Rust audience; tagging
  `@thisweekinrust.bsky.social` or `@ThisWeekinRust@mastodon.social` is an
  accepted way to get something noticed.
- **X** matters mainly because some health-IT outlets still monitor it.
- Cadence: one substantive post per release. Not per commit — the audience is
  senior, busy, and unforgiving of volume.

## 11. A sequence, anchored to real dates

**By 2026-09-05 — make the artefact survive inspection.** Fix the README install
version; add `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `AI_STATEMENT.md`; set the
GitHub topics; add the licence-choice sentence. Email the FerroEHR author. Start
reading Discourse daily and answer two threads that are not about this project.

**2026-09-08 to 2026-09-21 — announce, before the conference.** Post to
[Implementation](https://discourse.openehr.org/c/implem/39) using the anatomy in
§5.1. Post the release to r/rust. Nominate the crate on TWiR. Introduce the
project in the Software Program category. Then go to **EHRCON26 on 22–23
September** with the thread already live, so the conversations start at "I saw
your post" instead of at zero.

**October–November 2026 — convert attention into use.** Take part in the
**Collabrathon from 5 November**. Publish the technical essay (§6.3) and submit
it once as a Show HN. Submit a FOSDEM 2027 talk when the devroom CFPs open.
Convert one EHRCON conversation into a pilot user who will let you name them.

**December 2026 – February 2027 — make it citable.** MIE 2027 abstract when the
CFP appears. JOSS submission once the six-month history exists on 2027-02-01,
with the research-use evidence attached. Pitch *Rust in Production* only if a
deployment now exists. Pitch HTN only if that deployment has a name.

**Continuous.** One release note per version; one LinkedIn post per release; a
reply to every forum question within a day, because in a community this size
response latency *is* the reputation.

## 12. Templates

**Discourse announcement** —

> **openehr-rust: the Reference Model and SQL persistence as Rust libraries**
>
> Short version: eight crates on crates.io implementing the openEHR Reference
> Model — validation, paths, AQL parsing, change-control security — plus an
> engine-agnostic persistence layer and DDL for six SQL engines. It is not a
> CDR, and it does not implement the Archetype Model.
>
> Conformance, stated by level rather than by feature: SQLite is Verified (full
> store, re-checked in CI); PostgreSQL, MySQL, and MariaDB are at Schema (a real
> server executed the DDL and the append-only tables were observed refusing
> `UPDATE` and `DELETE`); SQL Server and Oracle are at Dialect (no server has
> parsed the DDL). The matrix is here: <link>.
>
> One finding you may care about regardless of the crates: MySQL's `JSON` type
> rewrote a stored magnitude of `1.10` as `1.1`. Details and reproduction:
> <link>.
>
> Licence: <one sentence>. How it was written, including machine assistance:
> <AI_STATEMENT link>.
>
> What would help most: tell me which engine you need at Store level, and
> whether the DDL for your engine looks right to you.

**Crate-of-the-week nomination** — two sentences: what it does, and the one
unusual thing (the conformance ladder, or `Real` instead of `f64`). No pitch.

**Cold email to a peer implementer** —

> Subject: how do you store a partial date like `2024-05`?
>
> Hi <name> — I maintain openehr-rust, a Rust implementation of the RM plus SQL
> persistence. We store times as two columns: an authoritative exact text column
> and a derived nullable UTC one, because `2024-05` is a date known to the month
> and not `2024-05-01`. I would like to know what <their system> does, and
> whether you have hit anything like our finding that MySQL's `JSON` rewrote
> `1.10` as `1.1`. Happy to share the reproduction either way. — <name>

**Press pitch** — one claim, one evidence link, one named interviewee, 120 words,
no attachment.

## 13. How to tell whether it worked

Track: docs.rs and crates.io traffic; GitHub referrer sources; **replies from
named implementers** on Discourse; inbound issues that quote a requirement
identifier (the strongest possible signal — it means someone read the
specification); pilot conversations opened. Do not track stars.

**Kill criterion.** If two announcement cycles pass and no implementer has asked
a question that required reading the code, the problem is the artefact or its
scope, not the channel — and the answer is a user, not more posts.

## 14. Risks

- **Over-claiming outside version control.** A forum post is permanent and no
  script checks it. Rule: every claim in promotional copy traces to a
  requirement identifier or to the conformance matrix, and the copy links there.
  `W0.3` and `W0.4` apply to what is said about this repository, not only to
  what is said inside it.
- **Announcing "six databases" while one is Verified.** This audience checks. If
  the level is not in the sentence, the sentence is a defect.
- **Regulatory implication.** Never say safe, compliant, certified, or clinically
  validated. None of those is a property of a crate, and in this sector the words
  have legal meanings.
- **Bus factor.** One contributor and three weeks of history is the first thing a
  vendor will raise. Have a plain answer rather than a defensive one.
- **Volume.** In a community of this size, posting more than the project has
  earned is itself a negative signal.

## Sources

All accessed 2026-08-25.

- [openEHR Discourse — categories](https://discourse.openehr.org/categories),
  [Implementation](https://discourse.openehr.org/c/implem/39),
  [Software Program — Open](https://discourse.openehr.org/c/spb/152)
- [openEHR API implementation in Rust (2023 thread)](https://discourse.openehr.org/t/openehr-api-implementation-in-rust/3489)
- [FerroEHR announcement, 2026-08-24](https://discourse.openehr.org/t/ferroehr-a-new-rust-based-openehr-cdr-looking-for-testers/17230)
- [openEHR-CLI announcement](https://discourse.openehr.org/t/new-open-source-tool-for-openehr-openehr-cli/11807)
- [Application to the Software Program Board](https://discourse.openehr.org/t/application-to-the-software-program-board/11830)
- [openEHR libraries listing](https://openehr.org/products_tools/libraries/),
  [affiliates](https://openehr.org/affiliates/affiliate-page/)
- [EHRCON26](https://openehr.org/event/ehrcon26/)
- [Medblocks community](https://medblocks.com/community)
- [this-week-in-rust submission process](https://github.com/rust-lang/this-week-in-rust)
- [crates.io categories](https://crates.io/categories),
  [category slugs](https://crates.io/category_slugs)
- [Rust community channels](https://rust-lang.org/community)
- [Rust in Production podcast](https://corrode.dev/podcast/)
- [FOSDEM 2026 call for participation](https://fosdem.org/2026/news/2025-09-21-call-for-participation/)
- [JOSS submission requirements](https://joss.readthedocs.io/en/latest/submitting.html),
  [review criteria](https://joss.readthedocs.io/en/latest/review_criteria.html)
- [MIE 2027](https://mie2027.efmi.org/), [MEDINFO 2027](https://www.medinfo2027.org/)
- [HTN — submit your news](https://htn.co.uk/suppliers/), [HTN contact](https://htn.co.uk/contact/)
- [Healthcare IT News](https://www.himss.org/hitn/),
  [Becker's Health IT](https://www.beckershospitalreview.com/healthcare-information-technology/),
  [Open Health News press releases](https://www.openhealthnews.com/resources/news/press-release)

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
