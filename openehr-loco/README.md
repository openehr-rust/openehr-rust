# openehr-loco

A RESTful openEHR API server on **Axum** and **Loco 1.0.1**, over
[`openehr-sqlite`](../openehr-sqlite).

Not published. It carries no conformance level, and `W16.14` forbids publishing
above one.

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
| `GET` | `/openehr/v1/ehr/{ehr_id}/composition?archetype_id=…` | search the index |
| `GET` | `…/composition/{uid}` | latest version — **`410` if deleted** |
| `GET` | `…/composition/{uid}/_history` | every version, oldest first |
| `GET` | `…/composition/{uid}/version/{version_uid}` | vread |
| `DELETE` | `…/composition/{uid}` | `501` — see below |

`_count` and `_offset` page, capped at 100. `_total` is returned as `total` and
is the count *before* paging: a short page without it is indistinguishable from
the end of the results.

`ETag` is **weak** (`W/"…"`). A strong tag asserts byte-for-byte equality of the
representation; this asserts only that the version is the same version, which is
the claim the service can actually keep.

### `DELETE` returns 501, deliberately

Deleting in openEHR is a *commit* carrying `AUDIT_DETAILS` — who did it and why.
That cannot be synthesised from a bare `DELETE` with no body: this service does
not authenticate (`S1.8`), so it has no committer to record, and inventing one
would put a false name in an audit trail. Commit a deleted version instead.

## Shape

```
src/
  app.rs           Hooks: routes, and before_run
  controllers/     mod.rs owns the status-code mapping
  views.rs         what goes over the wire
  initializers.rs
  tasks.rs         checkpoint, as a task and not an endpoint
```

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

`auth` is off for a different reason: a JWT layer here would imply this service
establishes who is calling. It does not (`S1.8`).

## Running

```sh
OPENEHR_SQLITE_PATH=openehr.sqlite3 cargo run
```

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](../LICENSE.md).
