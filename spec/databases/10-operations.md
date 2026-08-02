# 10. Operations

**Rewritten 2026-08-01.** Most of this section specified a **service** — liveness
and readiness endpoints, connection pooling, TLS termination, edge rate limits, a
metrics port. No service exists in this repository and none is in scope
(`S1.7`, `C0.6`). What survives is what binds a library: logging, migration,
connection security, and release evidence. See [`spec/audit.md`](../audit.md)
**W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `O10.13`
(`C0.19`).

Requirement prefix: `O10`.

## Logging

- **O10.2** *(amended)* A store MUST NOT log stored content. Log records may name
  identifiers, tables, operations, and rules; they MUST NOT name a value from a
  record (`M3.38`).

  A store's log is the one that reaches a connection-pool log, an APM trace, and
  a paging alert at once — three systems with three retention policies, none of
  them chosen for PHI.

- **O10.13** A store MUST NOT log a query's bound parameters at any level a
  production deployment would enable. Parameters are where the values are; a
  statement logged with placeholders intact is safe, and the same statement
  logged with substitutions is a disclosure.

## Installing and migrating

- **O10.4** *(amended)* `install()` MUST be idempotent (`G2.13`). A deployment
  that runs it twice, or retries after a partial failure, MUST NOT get a hard
  error the second time.
- **O10.14** *(amended)* There is **no migration mechanism before 1.0**, and
  this is a decision rather than a gap. The schema is expected to change while
  the model is still settling — it has changed twice already — and an upgrade
  route maintained against a moving shape is machinery that is wrong more often
  than it is right. A deployment before 1.0 exports, recreates, and reloads.

- **O10.15** A store MUST record the schema version its database was installed
  under, and `install()` MUST **refuse** a database recorded under a different
  one.

  This is the obligation that survives dropping migrations. `install()` is
  `CREATE TABLE IF NOT EXISTS`: run against an older database every statement
  no-ops, `install()` returns `Ok`, and the *first commit* fails on a column
  that is not there. Success followed by an unexplained failure is worse than
  refusing to start, and it is the pattern this specification rejects everywhere
  else (`S1.11`).

- **O10.16** A database holding data but recording **no** version MUST be
  refused, not treated as fresh. The absence is ambiguous — it means "new" or
  "older than versioning" — and the two are distinguished by whether the
  database holds anything. Guessing "new" runs the DDL over an unknown shape.

- **O10.17** The version MUST be bumped whenever the schema changes in a way an
  existing database cannot serve: a new column, a changed type, a new
  constraint. Not for a comment.

  It follows that the version is **not** the crate version. A release that
  changes no table leaves it alone, and two crate versions may share a schema.

- **O10.4a** *(amended — **not applicable**)* A migration changing stored derived
  values would need a recompute path. The one derived value here is the `…_utc`
  half of each instant pair (`M3.24`), and it is recomputable from `…_text` by
  the shared projection — so the recompute exists in principle even though the
  migration machinery does not.

## Connection security

- **O10.7** **The database connection carries PHI and MUST be encrypted** unless
  the database is in the same process.

  This is the one operational requirement in this section with no service caveat,
  because it binds any caller of any store. `openehr-sqlite` is exempt only
  because it is in-process: there is no connection.

- **O10.18** An engine crate that opens a connection MUST document how TLS is
  configured for its driver, and MUST NOT default to an unverified connection. No
  crate here opens a network connection yet, so this requirement binds the first
  one that does.

## Backup

- **O10.6** Backup and restore is the engine's own — `pg_dump` and PITR, a copied
  SQLite file, whatever the engine provides. This layer MUST NOT offer a
  bespoke backup format.

  A backup taken by the engine is one an operator already knows how to restore
  and already tests. A bespoke one is a second thing to get right, and it will be
  discovered to be wrong at the worst moment.

- **O10.19** Because `openehr_version` and `openehr_contribution` are append-only
  (`M3.17`), a restore to a point in time cannot lose a committed version without
  losing the rows after it. That is a property worth relying on and worth stating:
  the history is not rewritten in place, so a backup is a prefix rather than a
  snapshot of mutable state.

## Release evidence

- **O10.10** A release MUST ship supply-chain evidence: an advisory and licence
  audit over the dependency graph, and a lock file committed so that "the tests
  passed" and "the audit ran against these versions" mean the same thing twice.

  `Cargo.lock` is committed in every crate here, unusually for libraries, for
  exactly that reason.

- **O10.11** A published version MUST match the source that claims it. A tag, the
  crate's `version`, and the published artefact MUST agree.
- **O10.12** *(amended)* A contributor's local verification and CI MUST run the
  **same** script, not two implementations of one check. Two ways of doing one
  check drift, and the one that drifts is always the one nobody runs
  (`T11.19`).

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).
All were marked **[service]** and describe a component this repository does not
build (`S1.7`).

| Id | Was |
| --- | --- |
| `O10.1` | liveness and readiness endpoints on an admin port |
| `O10.3` | connection pooling; pool exhaustion returns 503 |
| `O10.5` | TLS terminated at a service edge |
| `O10.8` | resource limits enforced at the edge |
| `O10.9` | metrics and health served on a separate port |

These are withdrawn rather than retained-as-future because a service, if one is
ever built, will be a different component with its own specification. Keeping
requirements here for it would mean this section described two things.

---

Part of the [openEHR persistence specification](index.md).
