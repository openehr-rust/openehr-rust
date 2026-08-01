#!/bin/sh
# Runs a dialect's generated DDL against the engine it names, and checks the
# three things a golden test cannot: that the engine parses it, that running it
# twice is a no-op, and that the append-only tables actually refuse.
#
# This is what separates conformance level *Schema* from *Dialect*
# (openehr-store/spec/conformance.md). It found A-13, A-14, and A-15.
#
#   usage: verify-schema.sh postgresql|mysql|mariadb
#
# Requires podman (or docker, via $CONTAINER). Provisions the engine itself and
# tears it down after; it does not use an existing database, because a database
# somebody else prepared is a database whose state is not evidence.
#
# `mariadb` is a separate branch from `mysql` and must stay one. The MariaDB
# crate previously claimed Schema level citing this script, which at the time
# rejected the argument outright — so the documented reproducer could never have
# been run by anyone, and the DDL it would have run was MySQL's byte for byte.

set -eu

ENGINE="${1:?usage: verify-schema.sh postgresql|mysql|mariadb}"
CONTAINER="${CONTAINER:-podman}"
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
NAME="openehr-verify-$ENGINE"
PASS="openehr_Passw0rd"

cleanup() { $CONTAINER rm -f "$NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

fail() { printf '\n  FAIL: %s\n' "$1" >&2; exit 1; }

sql=$(mktemp) || exit 1
cargo run -q --manifest-path "$ROOT/openehr-$ENGINE/Cargo.toml" --example ddl >"$sql"

case "$ENGINE" in
postgresql)
  IMAGE=docker.io/library/postgres:18-alpine
  $CONTAINER run -d --rm --name "$NAME" -e POSTGRES_PASSWORD="$PASS" \
    -e POSTGRES_DB=openehr "$IMAGE" >/dev/null
  # Readiness is a successful query against the *target database*, not
  # `pg_isready`. The official images run a temporary server during
  # initialization, and a liveness probe answers for it — before the real server
  # is up and before POSTGRES_DB exists. A probe that can pass while its subject
  # is absent is the same defect the schema check itself guards against
  # (`C0.12`), and it is what made the MySQL job fail in CI while passing
  # locally: the DDL ran against a database that did not exist yet.
  i=0
  while ! $CONTAINER exec "$NAME" psql -U postgres -d openehr -c 'SELECT 1' \
      >/dev/null 2>&1; do
    i=$((i + 1))
    [ "$i" -gt 90 ] && fail "PostgreSQL did not become ready"
    sleep 2
  done
  apply() {
    $CONTAINER exec -i "$NAME" psql -U postgres -d openehr -v ON_ERROR_STOP=1 -f - <"$sql" 2>&1 |
      grep -E '^(psql:.*)?ERROR' || true
  }
  # A row must exist before the mutations: a FOR EACH ROW trigger on zero rows
  # never fires, so an empty table reports refusal it never performed.
  seed() {
    $CONTAINER exec -i "$NAME" psql -U postgres -d openehr -q >/dev/null 2>&1 <<'SEED'
INSERT INTO openehr_ehr VALUES ('e1','sys','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','st1','ac1');
INSERT INTO openehr_versioned_object VALUES ('vo1','e1','VERSIONED_COMPOSITION','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
INSERT INTO openehr_contribution VALUES ('c1','e1','249','sys','Committer','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
INSERT INTO openehr_version (uid, versioned_object_uid, creating_system_id, trunk_version,
  lifecycle_state_code, is_deleted, contribution_uid, audit_system_id,
  audit_change_type_code, audit_time_committed_text)
VALUES ('vo1::sys::1','vo1','sys',1,'532',false,'c1','sys','249','2026-01-01T00:00:00Z');
SEED
  }
  rows() { $CONTAINER exec "$NAME" psql -U postgres -d openehr -Atc "SELECT count(*) FROM openehr_version"; }
  refuse() { $CONTAINER exec "$NAME" psql -U postgres -d openehr -c "$1" 2>&1 | grep -c 'append-only' || true; }
  ;;
mysql)
  IMAGE=docker.io/library/mysql:8.4
  $CONTAINER run -d --rm --name "$NAME" -e MYSQL_ROOT_PASSWORD="$PASS" \
    -e MYSQL_DATABASE=openehr "$IMAGE" >/dev/null
  # Not `mysqladmin ping`: see the note in the postgresql branch. MySQL's
  # entrypoint starts a temporary server to run initialization, and ping answers
  # for it while MYSQL_DATABASE does not yet exist. This loop waits for a real
  # query against the real database.
  i=0
  while ! $CONTAINER exec "$NAME" mysql -uroot -p"$PASS" openehr \
      -e 'SELECT 1' >/dev/null 2>&1; do
    i=$((i + 1))
    [ "$i" -gt 90 ] && fail "MySQL did not become ready"
    sleep 2
  done
  # `|| true` because grep exits 1 when it filters every line, and a bare
  # `seed` call under `set -e` would then abort the script before `fail` could
  # report why — which is how this script first appeared to fail with no message.
  my() {
    $CONTAINER exec -i "$NAME" mysql -uroot -p"$PASS" openehr "$@" 2>&1 |
      { grep -v 'Using a password' || true; }
  }
  apply() { my <"$sql" | grep -E '^ERROR' || true; }
  seed() {
    my >/dev/null 2>&1 <<'SEED'
INSERT INTO openehr_ehr VALUES ('e1','sys','2026-01-01T00:00:00Z','2026-01-01 00:00:00','st1','ac1');
INSERT INTO openehr_versioned_object VALUES ('vo1','e1','VERSIONED_COMPOSITION','2026-01-01T00:00:00Z','2026-01-01 00:00:00');
INSERT INTO openehr_contribution VALUES ('c1','e1','249','sys','Committer','2026-01-01T00:00:00Z','2026-01-01 00:00:00');
INSERT INTO openehr_version (uid, versioned_object_uid, creating_system_id, trunk_version,
  lifecycle_state_code, is_deleted, contribution_uid, audit_system_id,
  audit_change_type_code, audit_time_committed_text)
VALUES ('vo1::sys::1','vo1','sys',1,'532',0,'c1','sys','249','2026-01-01T00:00:00Z');
SEED
  }
  rows() { my -Nse "SELECT count(*) FROM openehr_version"; }
  refuse() { my -e "$1" | grep -c 'append-only' || true; }
  ;;
mariadb)
  IMAGE=docker.io/library/mariadb:11.4
  $CONTAINER run -d --rm --name "$NAME" -e MARIADB_ROOT_PASSWORD="$PASS" \
    -e MARIADB_DATABASE=openehr "$IMAGE" >/dev/null
  # A real query against the real database, not `mariadb-admin ping`: see the
  # note in the postgresql branch. This branch happened to pass in CI, which is
  # luck rather than a difference — the same race is present.
  i=0
  while ! $CONTAINER exec "$NAME" mariadb -uroot -p"$PASS" openehr \
      -e 'SELECT 1' >/dev/null 2>&1; do
    i=$((i + 1))
    [ "$i" -gt 90 ] && fail "MariaDB did not become ready"
    sleep 2
  done
  # The client is `mariadb`, not `mysql`: MariaDB 11 renamed every binary and
  # the compatibility symlinks are deprecated. Using `mysql` here would work
  # today and break on the release that drops them, which is the sort of
  # difference this branch exists to keep visible.
  my() {
    $CONTAINER exec -i "$NAME" mariadb -uroot -p"$PASS" openehr "$@" 2>&1 |
      { grep -v 'Using a password' || true; }
  }
  apply() { my <"$sql" | grep -E '^ERROR' || true; }
  seed() {
    my >/dev/null 2>&1 <<'SEED'
INSERT INTO openehr_ehr VALUES ('e1','sys','2026-01-01T00:00:00Z','2026-01-01 00:00:00','st1','ac1');
INSERT INTO openehr_versioned_object VALUES ('vo1','e1','VERSIONED_COMPOSITION','2026-01-01T00:00:00Z','2026-01-01 00:00:00');
INSERT INTO openehr_contribution VALUES ('c1','e1','249','sys','Committer','2026-01-01T00:00:00Z','2026-01-01 00:00:00');
INSERT INTO openehr_version (uid, versioned_object_uid, creating_system_id, trunk_version,
  lifecycle_state_code, is_deleted, contribution_uid, audit_system_id,
  audit_change_type_code, audit_time_committed_text)
VALUES ('vo1::sys::1','vo1','sys',1,'532',0,'c1','sys','249','2026-01-01T00:00:00Z');
SEED
  }
  rows() { my -Nse "SELECT count(*) FROM openehr_version"; }
  refuse() { my -e "$1" | grep -c 'append-only' || true; }
  ;;
*)
  fail "unknown engine '$ENGINE' (postgresql|mysql|mariadb)"
  ;;
esac

printf '%s: ' "$ENGINE"

out=$(apply)
[ -z "$out" ] || fail "the engine rejected the DDL: $out"
printf 'parses '

out=$(apply)
[ -z "$out" ] || fail "re-running the DDL is not a no-op: $out"
printf 'idempotent '

seed
[ "$(rows | tr -d '[:space:]')" = "1" ] || fail "seed row absent; enforcement below would prove nothing"

# A text column, not `is_deleted`: PostgreSQL rejects `is_deleted = 1` as a type
# error at plan time, before any trigger fires, so the refusal counted would
# have been the wrong refusal. Booleans are exactly where these dialects differ.
for stmt in \
  "UPDATE openehr_version SET lifecycle_state_code = '523' WHERE uid = 'vo1::sys::1'" \
  "DELETE FROM openehr_version WHERE uid = 'vo1::sys::1'" \
  "UPDATE openehr_contribution SET audit_committer_name = 'x' WHERE uid = 'c1'"; do
  [ "$(refuse "$stmt" | tr -d '[:space:]')" = "0" ] &&
    fail "not refused, and it must be: $stmt"
done
printf 'append-only enforced '

[ "$(rows | tr -d '[:space:]')" = "1" ] || fail "a refused statement still changed the table"
printf 'row intact\n'
