# Conformance runs — bring-your-own-SUT

Non-normative. A discovery record, the same discipline
[`openehr/spec/corpus.md`](../../openehr/spec/corpus.md) applies to the
archetype corpus: what happened when the runner was pointed at a real
system-under-test, dated, with the evidence kept rather than summarised away.

The runner is [`scripts/conformance.sh`](../scripts/conformance.sh):

```sh
sh scripts/conformance.sh <base-url> <auth-header-value> <existing-ehr-id>
```

## Why two endpoints of the eleven, not eleven

This crate serves eleven routes. Checked directly against a real, running
**EHRbase 2.35.1** (`docker.io/ehrbase/ehrbase:latest`, its own `/v3/api-docs`
read live, not assumed from documentation) rather than guessed, most of them
cannot be one shared conformance case yet:

| This crate's route | Shared case? | Why |
| --- | --- | --- |
| `GET /metadata` | No | Not part of ITS-REST. Absent from EHRbase's own OpenAPI paths entirely — this crate's own invention, with nothing to check it against. |
| `POST /ehr` | No | The request itself differs. This crate expects a caller-built `Ehr` JSON body. EHRbase's real `POST /ehr` takes `subject_id`/`subject_namespace` query parameters (or an `EHR_STATUS` body) and builds the `Ehr` server-side — confirmed by exercising both: this crate's shape 400s against EHRbase and EHRbase's shape is not what this crate's handler reads. Neither is "the bug" without first deciding what this crate's `POST /ehr` contract should be, which is the "ITS-REST completeness" backlog item's question, not this runner's. |
| `GET /ehr/{ehr_id}` | **Yes** | Identical shape, identical semantics, both engines. |
| `POST /ehr/{ehr_id}/contribution` | No | Blocked — see compositions below; a contribution's own body wraps compositions. |
| `GET /composition` (search) | No | Not part of ITS-REST. Absent from EHRbase's own OpenAPI paths; a real openEHR client searches through AQL, which neither implementation here executes yet. |
| `POST .../composition` | No | EHRbase refuses a composition with no `archetype_details/template_id` at all: `{"error":"Bad Request","message":"Composition missing mandatory attribute: archetype details/template_id"}}`, confirmed by posting one with and then without `archetype_details`. Uploading a template first would let this run for real, but `openehr` has no OPT 1.4 reader to build one from (the "OPT 1.4 ingestion" backlog item) — a template crafted by hand for this runner alone would be testing this runner's own fixture, not this crate. |
| `GET`/`PUT`/`DELETE .../composition/{uid}` | No | Same blocker: nothing to read, update, or delete without a composition that was ever accepted. |
| `GET .../composition/{uid}/_history` | No | Same blocker, and a second one: EHRbase's own history lives under a **different resource entirely** — `versioned_composition/{uid}/revision_history`, not `composition/{uid}/_history`. This crate's path does not exist in ITS-REST's own shape, template blocker aside. |
| `GET .../composition/{uid}/version/{version_uid}` | No | Same two blockers as history: no composition to version-read, and EHRbase's own path is `versioned_composition/{uid}/version/{version_uid}`, not `composition/{uid}/version/{version_uid}`. |

What is left, and genuinely shared without qualification: reading an EHR
that exists, and reading one that does not. Two cases, not eleven — a smaller
claim than the backlog item's own headline, and the honest one.

## Run 1 — 2026-09-08

**Target A: `openehr-axa`.** `cargo run --example generate_test_token` for a
throwaway keypair, `OPENEHR_SQLITE_PATH=/tmp/conformance-test.sqlite3` and the
printed `OPENEHR_PASETO_PUBLIC_KEYS`, `cargo run -- start`. EHR created by
hand first (`POST /ehr` with this crate's own `Ehr` JSON shape — its own
contract, exercised on its own terms here since this run is not testing
`POST /ehr`).

```
PASS  read_existing_ehr            http://localhost:5150/openehr/v1/ehr/87284370-2D4B-4E3D-A3F3-F303D2F4F34B (wanted 200, got 200)
PASS  read_missing_ehr             http://localhost:5150/openehr/v1/ehr/00000000-0000-0000-0000-000000000000 (wanted 404, got 404)

2 passed, 0 failed
```

**Target B: a stock EHRbase.**

```sh
podman network create ehrbase-net
podman run -d --name ehrdb --network ehrbase-net \
  -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres \
  -e EHRBASE_USER_ADMIN=ehrbase -e EHRBASE_PASSWORD_ADMIN=ehrbase \
  -e EHRBASE_USER=ehrbase_restricted -e EHRBASE_PASSWORD=ehrbase_restricted \
  docker.io/ehrbase/ehrbase-v2-postgres:16.2
podman run -d --name ehrbase --network ehrbase-net -p 8080:8080 \
  -e DB_URL=jdbc:postgresql://ehrdb:5432/ehrbase \
  -e DB_USER_ADMIN=ehrbase -e DB_PASS_ADMIN=ehrbase \
  -e DB_USER=ehrbase_restricted -e DB_PASS=ehrbase_restricted \
  -e SECURITY_AUTHUSER=ehrbase-user -e SECURITY_AUTHPASSWORD=SuperSecretPassword \
  docker.io/ehrbase/ehrbase:latest
```

No Keycloak container: the upstream `docker-compose.yml` makes it a startup
dependency, but EHRbase's own log ("`UserDetailsService` bean with name
`inMemoryUserDetailsManager`") shows Basic auth against the configured
`SECURITY_AUTHUSER`/`SECURITY_AUTHPASSWORD` works without it — confirmed by
using it successfully below, not assumed. EHR created by hand first (`POST
/ehr?subject_id=…&subject_namespace=local`, EHRbase's own contract).

```
PASS  read_existing_ehr            http://localhost:8080/ehrbase/rest/openehr/v1/ehr/6ee5a7d2-56e0-4d9a-85fd-4a46c0cfdc1f (wanted 200, got 200)
PASS  read_missing_ehr             http://localhost:8080/ehrbase/rest/openehr/v1/ehr/00000000-0000-0000-0000-000000000000 (wanted 404, got 404)

2 passed, 0 failed
```

**A trap worth naming for the next person who runs this by hand:** macOS's
own `base64` appends `\r\n`, not `\n`. Captured through `$(...)`, the shell
strips the trailing `\n` and leaves the `\r` — invisible at a terminal, and
enough to make Tomcat reject the request outright ("`The HTTP header line
[...] does not conform to RFC 7230`"), which looks exactly like a real
authentication or interoperability failure until the container's own log is
read. `printf ... | base64 | tr -d '\r\n'` avoids it; `curl -u user:pass`
sidesteps the whole question by encoding internally. Neither crate nor
EHRbase caused this — pure local tooling, and the reason to read a server's
own log before trusting what a client library's error implies.

## Run 2 — 2026-11-27: creating an EHR (`db:S1.22`)

The runner gained the creation cases `S1.22` makes shared, now that this
service's `POST /ehr` follows ITS-REST: create with the default status and read
it straight back; create with a supplied `EHR_STATUS` and read it back; a
malformed body; and create with a chosen id, then again. Each checks status,
headers and body. Where ITS-REST states a response shape, the runner accepts
only that shape.

**Target A: `openehr-axa`** at this revision (uncommitted working tree), started
with `cargo run -- start`, a throwaway key from `generate_test_token` (grant
`"*"` read and write), and a fresh SQLite file. The existing record was
created with `PUT /ehr/{id}`.

```
PASS  read_existing_ehr            (wanted 200, got 200)
PASS  read_missing_ehr             (wanted 404, got 404)
PASS  create_default_status        POST /ehr -> 201, Location present, ehr_id ad8d3836-…
FAIL  read_default_status          GET .../ehr_status -> 200, is_modifiable null, is_queryable null
      body: {"uid":"b9d8b328-…::openehr-axa.local::1",…,"data":{"_type":"EHR_STATUS",…
FAIL  create_supplied_status       POST /ehr (EHR_STATUS body) -> 422, ehr_id none
      body: …"EHR_STATUS.Is_archetype_root — an EHR_STATUS has no archetype_details…"
PASS  create_invalid_body          POST /ehr (malformed body) -> 400, wanted 400
PASS  create_named                 PUT /ehr/{id} -> 201, wanted 201
PASS  create_named_twice           PUT /ehr/{id} again -> 409, wanted 409

6 passed, 2 failed
```

**Target B: EHRbase 2.38.0** (`docker.io/ehrbase/ehrbase:latest` on this date,
`/ehrbase/rest/status` reporting `ehrbase_version 2.38.0`, openEHR SDK 2.36.0,
Archie 3.13.0, PostgreSQL 16.2), started as in Run 1, with Basic auth.

```
PASS  read_existing_ehr            (wanted 200, got 200)
PASS  read_missing_ehr             (wanted 404, got 404)
PASS  create_default_status        POST /ehr -> 201, Location present, ehr_id 7476a17f-…
PASS  read_default_status          GET .../ehr_status -> 200, is_modifiable true, is_queryable true
PASS  create_supplied_status       POST /ehr (EHR_STATUS body) -> 201, ehr_id b7fa4706-…
PASS  read_supplied_status         GET .../ehr_status -> 200, is_modifiable false
PASS  create_invalid_body          POST /ehr (malformed body) -> 400, wanted 400
PASS  create_named                 PUT /ehr/{id} -> 201, wanted 201
PASS  create_named_twice           PUT /ehr/{id} again -> 409, wanted 409

9 passed, 0 failed
```

**What the two failures are.** Both are this service's, registered in
`spec/databases/audit.md`:
- `D-31`: the status read answers in this service's own version view, where
  ITS-REST answers with the resource.
- `D-32`: the RM's `Is_archetype_root` invariant refuses a status with no
  `archetype_details`, which EHRbase accepts. Because that create failed,
  `read_supplied_status` did not run here.

**What now agrees.** The two implementations agree on these points: creation
with no body and the immediate existence of a status; the `Location` header
and the minted id in the body; `400` for a malformed body; `PUT` with a named
id; and `409` on a second `PUT`.

## Run 3 — 2026-11-27: after `S1.23`

The same day, after `D-31` was fixed (the ITS-REST routes answer with the
resource) and `D-32` decided (a supplied `EHR_STATUS` must carry
`archetype_details`, so the runner's supplied-status body now does). Same
two targets, with a fresh throwaway token. The first attempt against this
service used Run 2's token, an hour past its expiry, and got `401` on every
case.

That attempt also exposed a runner defect, now fixed. Under `set -e`, a lone
failing `[ … ]` ended the script before its verdict, so it stopped after four
cases with no summary. Every condition is now `cond && ok=0 || ok=1`. Checked
with a bad token: all cases run, `0 passed, 7 failed`, and the two reads that
need a created id are skipped.

**Target A: `openehr-axa`**

```
PASS  read_existing_ehr            (wanted 200, got 200)
PASS  read_missing_ehr             (wanted 404, got 404)
PASS  create_default_status        POST /ehr -> 201, Location present, ehr_id 6850214b-…
PASS  read_default_status          GET .../ehr_status -> 200, is_modifiable true, is_queryable true
PASS  create_supplied_status       POST /ehr (EHR_STATUS body) -> 201, ehr_id bc24597b-…
PASS  read_supplied_status         GET .../ehr_status -> 200, is_modifiable false
PASS  create_invalid_body          POST /ehr (malformed body) -> 400, wanted 400
PASS  create_named                 PUT /ehr/{id} -> 201, wanted 201
PASS  create_named_twice           PUT /ehr/{id} again -> 409, wanted 409

9 passed, 0 failed
```

**Target B: EHRbase 2.38.0**

```
PASS  read_existing_ehr            (wanted 200, got 200)
PASS  read_missing_ehr             (wanted 404, got 404)
PASS  create_default_status        POST /ehr -> 201, Location present, ehr_id dd5c5561-…
PASS  read_default_status          GET .../ehr_status -> 200, is_modifiable true, is_queryable true
PASS  create_supplied_status       POST /ehr (EHR_STATUS body) -> 201, ehr_id 493b114c-…
PASS  read_supplied_status         GET .../ehr_status -> 200, is_modifiable false
PASS  create_invalid_body          POST /ehr (malformed body) -> 400, wanted 400
PASS  create_named                 PUT /ehr/{id} -> 201, wanted 201
PASS  create_named_twice           PUT /ehr/{id} again -> 409, wanted 409

9 passed, 0 failed
```

**What the run establishes.** The two implementations agree on all 9
cases: reading an existing and a missing record, creating one with and
without a supplied status, reading the status straight back in ITS-REST's
shape, refusing a malformed body, and creating with a chosen id once.
**What it does not cover:** the composition and contribution routes (still
blocked on templates, candidate 2 below), and the minimal status body
`D-32` refuses by decision.

## Run 4 — 2026-11-27: the status revision history (`db:S1.24`)

The runner gained `status_revision_history`. Right after the default create,
`GET …/versioned_ehr_status/revision_history` must answer `200` with one
item, a creation (`249`), with a committer. Both targets were started afresh,
as in Run 3.

```
openehr-axa:     10 passed, 0 failed
EHRbase 2.38.0:  10 passed, 0 failed

PASS  status_revision_history  -> 200, items 1, change 249, committer PARTY_IDENTIFIED   (both)
```

The other nine cases are unchanged from Run 3, and pass on both.

## Candidates for the next run

In the order a fix would unblock them:

1. ~~Decide `POST /ehr`'s real contract~~. Done 2026-11-27 (`db:S1.22`, Run 2).
   `D-31` fixed and `D-32` decided the same day (Run 3: 9/9 on both).
2. **OPT 1.4 ingestion**, then a template uploaded for real — unblocks every
   composition and contribution case at once, six of the remaining nine.
3. **`versioned_composition/...`** as this crate's own path for history and
   version-read, matching EHRbase's — a naming fix, not blocked on anything
   else, and closes the `_history`/`version` cases without needing a
   template at all.
4. `GET /composition` (search) and `GET /metadata` have no ITS-REST
   reference to check against and are not candidates for this runner; they
   are this crate's own surface, tested elsewhere (`tests/http.rs`).

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used
with the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
