#!/bin/sh
# Runs the shared conformance suite — the same one `openehr-sqlite` runs on
# every `cargo test` — against a real PostgreSQL server, via the `Store`
# trait rather than raw SQL. This is what separates conformance level *Store*
# from *Schema* (openehr-store/spec/conformance.md): *Schema* is
# `openehr-store/scripts/verify-schema.sh postgresql`, the generated DDL
# executed and its append-only tables observed refusing a mutation, both
# checked with `psql` and no Rust code involved; *Store* is this script,
# `openehr-postgresql`'s own `PostgresqlStore` committing, reading, and
# refusing exactly what `conformance::run`/`run_ehr_status`/
# `run_is_modifiable_gate` assert of every engine that claims it.
#
#   usage: sh openehr-postgresql/scripts/verify-store.sh
#
# Requires podman (or docker, via $CONTAINER). Provisions the server itself
# and tears it down after — a server somebody else prepared is a server
# whose state is not evidence, the same reason `verify-schema.sh` gives.
#
# Unlike `verify-schema.sh`'s own container, reached only through
# `podman exec psql` from inside the container's own network namespace, this
# one publishes its port to the host: `PostgresqlStore` is a real
# `postgres`-crate client making a real TCP connection from the *test
# process*, not a shell issuing commands inside the container.

set -eu

CONTAINER="${CONTAINER:-podman}"
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
NAME="openehr-verify-store-postgresql"
PASS="openehr_Passw0rd"
# A high, unusual port: distinct from PostgreSQL's own default (5432) so this
# never collides with a real server already running on the host, and
# published only to loopback so nothing outside this machine ever sees it.
PORT=54329

cleanup() { $CONTAINER rm -f "$NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

fail() {
    printf '\n  FAIL: %s\n' "$1" >&2
    printf '  --- last 40 lines of the postgresql container log ---\n' >&2
    $CONTAINER logs --tail 40 "$NAME" 2>&1 | sed 's/^/  | /' >&2 || true
    exit 1
}

# Mirrors `verify-schema.sh`'s own `await`: generous on purpose, and a
# real query against the target database rather than a liveness probe that
# can pass against the image's own temporary initialization server (see that
# script's own comment on the postgresql branch for the CI failure this
# guards against).
await() {
    i=0
    until "$@" >/dev/null 2>&1; do
        i=$((i + 1))
        if [ "$i" -gt 150 ]; then
            fail "PostgreSQL did not become ready within 300s"
        fi
        sleep 2
    done
}

IMAGE=docker.io/library/postgres:18-alpine
$CONTAINER run -d --rm --name "$NAME" -e POSTGRES_PASSWORD="$PASS" \
    -e POSTGRES_DB=openehr -p "127.0.0.1:${PORT}:5432" "$IMAGE" >/dev/null

pg() { $CONTAINER exec -e PGPASSWORD="$PASS" "$NAME" psql -h 127.0.0.1 -U postgres -d openehr "$@"; }
await pg -c 'SELECT 1'

export OPENEHR_POSTGRESQL_URL="host=127.0.0.1 port=${PORT} user=postgres password=${PASS} dbname=openehr"

# `--test-threads=1`: every test in `tests/store.rs` and `tests/
# concurrency.rs` drops and recreates the whole schema against this one
# shared server (`reset`, `tests/store.rs`'s own doc) rather than opening a
# throwaway database the way `SqliteStore::in_memory` lets `openehr-sqlite`'s
# own suite do per test. Running them concurrently would have one test's
# `DROP TABLE` firing while another is mid-commit. `tests/concurrency.rs`'s
# own within-one-test thread pools are unaffected — this flag limits how many
# *test functions* the harness runs at once, not threads a test spawns
# itself.
cargo test --manifest-path "$ROOT/openehr-postgresql/Cargo.toml" \
    --test store --test concurrency -- \
    --ignored --test-threads=1
