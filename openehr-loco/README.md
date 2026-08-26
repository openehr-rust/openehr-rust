# openehr-loco

A RESTful openEHR® API server on **Axum** and **Loco 1.0.1**, over
[`openehr-sqlite`](../openehr-sqlite).

> openEHR® is the registered trademark of the openEHR Foundation. Use of the
> trademark does not constitute endorsement of this product by openEHR
> International or openEHR Foundation.

Not published, and it sits **outside the conformance ladder** — every rung there
is defined by DDL, a `Store` implementation, or a database server, and this
crate is none of those. So it states evidence instead of a level (`W0.32`).

**Demonstrated.** 53 tests. `tests/http.rs` serves real requests through Loco's
own router: `410` for a deleted composition against `404` for one that never
existed, the history still readable behind that `410`, `401` on every clinical
route without a token and an identical body whether or not the record exists,
the weak `ETag`, `_count`/`_offset`/`total` and the paging cap, `501` on
`DELETE`, `503` rather than `404` when the store is missing, a commit read back
immediately, `403` for a committer who is not the caller against `422` for one
who cannot be identified, and `If-Match` required, stale, starred, and in both
spellings. Both of the
first two were mutation-checked — the branch was disabled and the test went red.
`src/auth.rs` covers key rotation, expiry, audience binding, the implicit
assertion, a token naming nobody, a `v4.local` token offered as `v4.public`,
and spoofed identity headers. `tests/tasks.rs` executes the **built binary** and
reads its output.

The server has also been booted by hand and answered `curl`: `/metadata` served,
`401` with `WWW-Authenticate: Bearer` on a clinical route, and a refusal to start
at all when `OPENEHR_PASETO_PUBLIC_KEYS` was wrong.

**Not demonstrated.** No run against a real deployment, no concurrency
behaviour, no TLS, and no engine other than SQLite.

## Why this crate exists separately

The database crates deliberately ship no server, so a program that wants storage
does not also acquire a web framework. This is where that surface lives instead.

The dependency runs **inward only**: this crate depends on the store, nothing
depends on this, and deleting it changes nothing else. `S1.7` was amended to
allow it — narrowed to "the core builds no service", which is the boundary the
original wording was actually protecting.

## Its job is narrow by design

Translate HTTP to store calls, and get the status codes right.

Everything it *appears* to promise — versioned history, the tamper-evident
chain, search, decimal fidelity — is the storage crate's work. This crate adds
no clinical behaviour and must not (`S1.19`): a rule enforced at the HTTP edge
stops applying the moment somebody uses the store directly.

## The distinction it does own

**A resource that was deleted answers `410 Gone`. One that never existed answers
`404 Not Found`.**

openEHR removes nothing — a deletion is a new version carrying a deleted
lifecycle state, and the history stays. Collapsing the two would tell a caller
that a record it once held never was, and a clinician or auditor told "not
found" stops looking (`S1.20`).

## Endpoints

| Method | Path | |
| --- | --- | --- |
| `GET` | `/openehr/v1/metadata` | what this is, and what it does **not** do |
| `POST` | `/openehr/v1/ehr` | create a record |
| `GET` | `/openehr/v1/ehr/{ehr_id}` | read one |
| `POST` | `/openehr/v1/ehr/{ehr_id}/contribution` | declare a change set |
| `POST` | `…/composition?contribution=…` | commit a first version |
| `PUT` | `…/composition/{uid}?contribution=…` | commit a successor — **`If-Match` required** |
| `GET` | `/openehr/v1/ehr/{ehr_id}/composition?archetype_id=…` | search the index |
| `GET` | `…/composition/{uid}` | latest version — **`410` if deleted** |
| `GET` | `…/composition/{uid}/_history` | every version, oldest first |
| `GET` | `…/composition/{uid}/version/{version_uid}` | vread |
| `DELETE` | `…/composition/{uid}` | `501` — see below |

Every endpoint except `/metadata` requires a token — see
[Verifying the caller](#verifying-the-caller). `/metadata` is open because a
caller has to be able to discover how to authenticate before it can, and nothing
there is clinical.

`_count` and `_offset` page, capped at 100. `_total` is returned as `total` and
is the count *before* paging: a short page without it is indistinguishable from
the end of the results.

`ETag` is **weak** (`W/"…"`). A strong tag asserts byte-for-byte equality of the
representation; this asserts only that the version is the same version, which is
the claim the service can actually keep.

### Writing

The body of a write is a whole `VERSION`, not a bare `COMPOSITION`. The
alternative would have this service mint the version identifier, the commit
time, and the `AUDIT_DETAILS` around a composition it was handed — inventing the
record of who did what, when, and why, which belongs to the caller (`S1.19`,
`PR12.9`). A `CONTRIBUTION` is declared first for the same reason: it is a
change set with its own audit (`PR12.10`), and minting one per commit would mean
inventing the act.

**The committer must be the caller** (`PR12.19`). A body naming somebody else is
`403` — authentication succeeded, the claim about who did the work did not. A
committer carrying no identifier is `422`, because that caller has not tried to
impersonate anyone; they have sent valid openEHR this service cannot attribute.
Comparison is on `external_ref.id` or any `DV_IDENTIFIER.id`, **never on the
name**: two clinicians share a name, and one clinician changes theirs.

`If-Match` is **required** on `PUT`, accepted as either `W/"<version-uid>"` or
the bare `<version-uid>` — the first is what a client round-trips from the
`ETag`, the second is what the openEHR REST API specifies. A stale one is `412`,
not `409`: *the statement you made about the world is false* and *the store
refused this commit* are different instructions, and a caller that conflates
them retries the wrong one. `If-Match: *` is refused, since it would let a
caller overwrite a version they have never seen.

This departs from RFC 9110 §13.1.1, which requires strong comparison for
`If-Match` — under which a weak tag never matches. Declared rather than hidden
(`H5.15`): the tag names a *version*, which is precisely what optimistic
concurrency asks about, and strong comparison would reject two byte-different
serialisations of one version, which are the same clinical fact.

### `DELETE` returns 501, deliberately

Deleting in openEHR is a *commit* carrying `AUDIT_DETAILS` — who did it, what
kind of change it was, and **why** — plus a `preceding_version_uid` placing it in
the history.

This used to say the service had no committer to record. It now has one: the
verified subject would map onto `AUDIT_DETAILS.committer` perfectly well. What a
bare `DELETE` still cannot supply is the reason, and defaulting it to something
like `"deleted via HTTP DELETE"` would synthesise the field an investigation
actually reads (`PR12.9`). So the refusal stands — now by choice rather than by
inability. Commit a deleted version instead.

## Shape

```
config/            development.yaml, test.yaml
src/
  main.rs          the binary
  app.rs           Hooks: routes, before_run, and open_store
  auth.rs          PASETO v4.public verification
  controllers/     mod.rs owns the status-code mapping
  views.rs         what goes over the wire
  tasks.rs         checkpoint and verify — tasks, not endpoints
```

There is no `initializers.rs`. It held one initializer whose `after_routes`
returned the router unchanged while its doc comment described echoing a request
id. Loco's own `request_id` middleware does that, it is switched on in
`config/development.yaml`, and the header is visible on every response.

**Stores are opened in `before_run`, not `boot`.** `boot` is not on the path
`start` takes; initialising there left every request answering `503` while the
health check stayed green — the worst combination available, because a load
balancer keeps a wholly broken instance in rotation and reports it healthy.

The chain checkpoint is a **task**, not an endpoint, for a related reason: a
checkpoint is only worth anything published somewhere the database administrator
does not control (`M3.16c`), and an endpoint on this service invites storing it
beside the data it attests to.

## Loco features

`default-features = false`, with only `cli`.

Loco's defaults include `with-db`, which pulls SeaORM, `sea-orm-migration`, and
`sqlx`. This repository already has a persistence layer built on a storage model
that is deliberately *not* shredded (`S1.5`); taking Loco's as well would mean
two persistence layers with two ideas of what a record is, and the one Loco
brings does not know what a `COMPOSITION` is.

`auth` is off for a different reason: it is `jsonwebtoken`, and this service
verifies PASETO instead. See below.

## Verifying the caller

This service is a **relying party, not an identity provider** (`PR12.13`). It
checks that an issuer signed a statement about who is calling; it holds no
credential, checks no password, and has no registration or recovery path. A
careless issuer produces careless attributions and every signature still
verifies — which is why `PR12.8` still says this layer does not authenticate.

### PASETO replaces the header

The principal comes from the token and from nothing else. No header names a
caller here — not `X-Principal`, not `X-Forwarded-User`, not `X-On-Behalf-Of`,
not `Remote-User`, not `X-Provenance`. There is no trusted-proxy mode and no
allow-listed-peer mode (`PR12.21`).

```
Before   X-Principal: clinician-4417
         ↑ believed because of where it arrived from

Now      Authorization: Bearer v4.public.…
         ↑ believed because the signature verifies
```

The difference is *where the check lives*. A trusted header puts it in the
network diagram — and network diagrams are edited by people who are not reading
this. A header that is safe behind one ingress becomes attacker-controlled the
day a second route to the service exists, and nothing in the code changes to
mark that day. A signature is checked here, on every request, and does not
depend on any claim about topology still being true.

Two tests hold this: one that such headers alone never authenticate, and one
that alongside a valid token they do not alter the subject. Both were
mutation-checked by adding the `X-Forwarded-User` fallback and watching them go
red — otherwise the prohibition is satisfied by nobody having written the
feature yet, which is a different thing.

### Why PASETO and not JWT

JWT negotiates its algorithm *inside the token*, and the token comes from the
caller. That one decision is the root of `alg: none`, of RS256 verified as HS256
against the public key used as an HMAC secret, and of a decade of advisories
that are all the same advisory rediscovered.

PASETO removes the negotiation. The version **is** the algorithm: `v4.public` is
Ed25519, and there is no field in which a caller can propose otherwise
(`PR12.14`). Same instinct as `ColTy` not being `#[non_exhaustive]` — prefer the
design where the dangerous thing cannot be *expressed* over the one where it can
be expressed and must be caught.

### Public key only, and why that matters more here

This crate holds a verification key, contains no code that signs, and never
loads a secret key (`PR12.15`).

Symmetric verification — PASETO `v4.local`, or JWT's HS256 — means the verifier
holds the key that *mints*. Ordinarily that is an accepted trade. It is not
acceptable here, because a verified subject becomes an `AUDIT_DETAILS.committer`
inside an append-only, hash-chained history whose whole purpose is to be
evidence years later. An attacker reaching a minting service could attribute a
commit to a clinician who never touched the system, and that attribution would
then be chained, append-only, and indistinguishable from the real ones.

Asymmetric verification bounds the worst case to misuse of tokens actually
presented. Fabricated evidence and misused evidence are different incidents.

### Configuration

| Variable | | |
| --- | --- | --- |
| `OPENEHR_PASETO_PUBLIC_KEYS` | **required** | PASERK `k4.public.…`, comma separated |
| `OPENEHR_PASETO_ISSUER` | optional | expected `iss` |
| `OPENEHR_PASETO_AUDIENCE` | optional | expected `aud` — **read the note** |
| `OPENEHR_PASETO_IMPLICIT_ASSERTION` | optional | binds tokens to one environment |

**Without a key the service refuses to start** (`PR12.16`). Not a warning, not a
development fallback. The failure being guarded against is silent in the worst
direction: the process starts, the health check is green, every request
succeeds, and the only symptom is that the whole record set is readable by
anyone who can reach the port. A refusal to start pages someone at 03:00; a
service that starts open produces a breach notification.

**Leaving `AUDIENCE` unset is a real exposure.** Where one issuer serves several
services and none check `aud`, a token handed to the least sensitive of them can
be replayed against this one. It is optional because a single-service deployment
genuinely does not need it, and stated loudly because the gap is invisible until
somebody goes looking (`PR12.17`).

Several keys may be listed. That is the rotation story: publish the incoming key
alongside the outgoing one, wait for every issued token to expire, then drop the
old entry. Every configured key is tried, rather than selecting one from the
token's own footer — an attacker-supplied field steering key selection is the
shape of mistake this crate chose PASETO to avoid.

Non-expiring tokens are refused (`PR12.17`). There is no revocation list here,
so a token that never expires cannot be withdrawn without rotating the key for
everybody at once.

### What it still does not do

**Verification is not authorization** (`PR12.18`). Every route below
`/openehr/v1` demands a valid token and **not one of them consults who it
names**. Deciding which records `clinician-4417` may open needs the care
relationship, the care team, the consent directives, and the break-glass rules —
a model this repository does not have. A service that checked a `roles` claim
and considered the matter settled would enforce a policy nobody wrote, which is
worse than enforcing none, because it looks like it works.

**Requiring a token did not create an audit trail** (`PR12.20`). The service now
establishes who is reading, on every single request, and discards it. That is a
worse statement than the old "this layer records no reads", not a better one:
the information exists and is thrown away. A deployment needing an access log
must still build one.

## Running

Generate a keypair with any PASETO v4 tooling, keep the secret at your issuer,
and give this service the public half:

```sh
OPENEHR_SQLITE_PATH=openehr.sqlite3 \
OPENEHR_PASETO_PUBLIC_KEYS=k4.public.… \
OPENEHR_PASETO_AUDIENCE=openehr-loco \
  cargo run -- start
```

```sh
curl -H 'Authorization: Bearer v4.public.…' \
  http://localhost:5150/openehr/v1/ehr/{ehr_id}
```

This section said `cargo run` from the day the crate was written, and there was
no `[[bin]]` target and no `config/`. It answered *"a bin target must be
available"*, and nothing noticed because the tests built the router directly and
never went near `boot`. Both now exist, and `tests/tasks.rs` executes the binary
so the claim stays true.

## Read auditing

Off by default. `OPENEHR_ACCESS_LOG=<path>` turns it on, and `/metadata` answers
`records_reads` either way so a caller never has to assume.

A version history records what **changed**; an access investigation asks who
**looked**. A clinician who opens a colleague's record and closes it again
leaves no trace in a history of commits (`PR12.5`). Verification made that worse
before it made it better: the service established who was calling on every
request and threw it away (`PR12.20`).

**The record is written and flushed before any clinical content reaches the
caller, and a read whose record cannot be written is refused with `503`**
(`PR12.6`). Return-first-record-after never blocks a read and loses exactly the
records an attacker most wants lost — a crash or a full disk between the
response and the write leaves the access unlogged and the data delivered. The
guarantee worth having is *no unaudited access*, and it costs a synchronous
append per read. A deployment that cannot pay that turns the log off and is told
so, rather than getting a quiet best-effort version nobody can testify from.

Records name identifiers and never content (`PR12.22`). An access log is shipped
to a collector, indexed, and retained on a schedule nobody chose for PHI; one
quoting what a record said is a second copy of the data under weaker protection
than the first.

Failed reads are recorded too, with `not_found`, `gone`, or `refused`
(`PR12.23`) — someone probing for records they cannot see is what an
investigation is looking for, and `gone` is kept separate from `ok` because who
looked at a withdrawn record is a sharper question.

```json
{"at":"2026-08-02T13:36:25.119903Z","subject":"clinician-4417",
 "action":"read","ehr":"87284370-…","target":"87284370-…","outcome":"ok"}
```

## Tasks

```sh
cargo loco task checkpoint container:<versioned-object-uid> [path:<db>]
cargo loco task verify     container:<versioned-object-uid> [path:<db>]
```

`checkpoint` prints a count, a head digest, and the head version's identifier,
and no clinical content — so it can be sent somewhere clinical data may not.
That is the only arrangement in which it is worth anything: a checkpoint stored
beside the data it attests to can be rewritten by whoever truncated the history
(`M3.16c`).

`verify` checks more than the chain links. It recomputes each version's content
digest from the stored bytes, which is what catches a document edited in place
while its chain columns were left alone (`M3.16d`). **It exits non-zero when the
history is not intact**, including when it verified nothing — a sweep against a
mistyped identifier must not report success for having checked an empty
container.

Both are tasks rather than endpoints. A verification endpoint answering "all
fine" is only as trustworthy as the process serving it, and an attacker who has
reached the database is one step from the process.

`path:` names a database explicitly, which is how a **restored backup** gets
checked — the copy an operator most wants verified, and never the one the
running service has open.

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](../LICENSE.md).
