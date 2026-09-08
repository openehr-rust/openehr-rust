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

**Target A: `openehr-loco`.** `cargo run --example generate_test_token` for a
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

## Candidates for the next run

In the order a fix would unblock them:

1. **Decide `POST /ehr`'s real contract** (`tasks.md` "ITS-REST completeness")
   — once decided, this run gains its second shared case.
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
