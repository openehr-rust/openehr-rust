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
# Requires podman (or docker, via $CONTAINER) and the `openssl` command used to
# create a short-lived private CA and server certificate. Provisions the server
# itself and tears it down after — a server somebody else prepared is a server
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
PLAIN_NAME="openehr-verify-store-postgresql-plain"
PASS="openehr_Passw0rd"
# A high, unusual port: distinct from PostgreSQL's own default (5432) so this
# never collides with a real server already running on the host, and
# published only to loopback so nothing outside this machine ever sees it.
PORT=54329
PLAIN_PORT=54330
TLS_DIR=$(mktemp -d)

cleanup() {
    $CONTAINER rm -f "$NAME" "$PLAIN_NAME" >/dev/null 2>&1 || true
    rm -rf "$TLS_DIR"
}
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

# A private CA and a server certificate for exactly `localhost`. The live
# negative cases use the same address with a different verification name, an
# unrelated CA, and a second server with TLS disabled. No test succeeds by
# disabling verification (`db:O10.18`).
openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 2 \
    -subj '/CN=openEHR PostgreSQL test CA' \
    -addext 'basicConstraints=critical,CA:TRUE' \
    -keyout "$TLS_DIR/ca.key" -out "$TLS_DIR/ca.crt" >/dev/null 2>&1
openssl req -newkey rsa:2048 -sha256 -nodes -subj '/CN=localhost' \
    -keyout "$TLS_DIR/server.key" -out "$TLS_DIR/server.csr" >/dev/null 2>&1
printf '%s\n' 'subjectAltName=DNS:localhost' 'extendedKeyUsage=serverAuth' \
    >"$TLS_DIR/server.ext"
openssl x509 -req -sha256 -days 2 -in "$TLS_DIR/server.csr" \
    -CA "$TLS_DIR/ca.crt" -CAkey "$TLS_DIR/ca.key" -CAcreateserial \
    -extfile "$TLS_DIR/server.ext" -out "$TLS_DIR/server.crt" >/dev/null 2>&1
openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 2 \
    -subj '/CN=unrelated test CA' \
    -addext 'basicConstraints=critical,CA:TRUE' \
    -keyout "$TLS_DIR/wrong-ca.key" -out "$TLS_DIR/wrong-ca.crt" >/dev/null 2>&1

$CONTAINER run -d --rm --name "$NAME" -e POSTGRES_PASSWORD="$PASS" \
    -e POSTGRES_DB=openehr -p "127.0.0.1:${PORT}:5432" "$IMAGE" >/dev/null

pg() { $CONTAINER exec -e PGPASSWORD="$PASS" "$NAME" psql -h 127.0.0.1 -U postgres -d openehr "$@"; }
await pg -c 'SELECT 1'

# PostgreSQL requires its private key to be owned by the server user and not
# readable by anyone else. Copy after initialization, set those permissions in
# the container, then restart under an explicit TLS configuration.
$CONTAINER cp "$TLS_DIR/server.crt" "$NAME:/var/lib/postgresql/server.crt"
$CONTAINER cp "$TLS_DIR/server.key" "$NAME:/var/lib/postgresql/server.key"
$CONTAINER exec "$NAME" chown postgres:postgres \
    /var/lib/postgresql/server.crt /var/lib/postgresql/server.key
$CONTAINER exec "$NAME" chmod 600 /var/lib/postgresql/server.key
pg -c "ALTER SYSTEM SET ssl = 'on'"
pg -c "ALTER SYSTEM SET ssl_cert_file = '/var/lib/postgresql/server.crt'"
pg -c "ALTER SYSTEM SET ssl_key_file = '/var/lib/postgresql/server.key'"
$CONTAINER restart "$NAME" >/dev/null
await pg -c 'SELECT 1'

$CONTAINER run -d --rm --name "$PLAIN_NAME" -e POSTGRES_PASSWORD="$PASS" \
    -e POSTGRES_DB=openehr -p "127.0.0.1:${PLAIN_PORT}:5432" "$IMAGE" >/dev/null
pg_plain() { $CONTAINER exec -e PGPASSWORD="$PASS" "$PLAIN_NAME" psql -h 127.0.0.1 -U postgres -d openehr "$@"; }
await pg_plain -c 'SELECT 1'

export OPENEHR_POSTGRESQL_URL="host=localhost hostaddr=127.0.0.1 port=${PORT} user=postgres password=${PASS} dbname=openehr"
export OPENEHR_POSTGRESQL_WRONG_HOST_URL="host=wrong.example hostaddr=127.0.0.1 port=${PORT} user=postgres password=${PASS} dbname=openehr"
export OPENEHR_POSTGRESQL_PLAINTEXT_URL="host=localhost hostaddr=127.0.0.1 port=${PLAIN_PORT} user=postgres password=${PASS} dbname=openehr sslmode=disable"
export OPENEHR_POSTGRESQL_CA="$TLS_DIR/ca.crt"
export OPENEHR_POSTGRESQL_WRONG_CA="$TLS_DIR/wrong-ca.crt"

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
    --test store --test concurrency --test tls -- \
    --ignored --test-threads=1
