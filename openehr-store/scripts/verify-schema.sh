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

# Every failure dumps the engine's own log. Without this the script reports
# *that* it failed and not *why*, which on a machine you cannot log into means
# the next step is a guess. Two CI failures were diagnosed by guessing before
# this was added; the second guess was wrong.
fail() {
    printf '\n  FAIL: %s\n' "$1" >&2
    printf '  --- last 40 lines of the %s container log ---\n' "$ENGINE" >&2
    $CONTAINER logs --tail 40 "$NAME" 2>&1 | sed 's/^/  | /' >&2 || true
    printf '  --- container state ---\n' >&2
    $CONTAINER ps -a --filter "name=$NAME" 2>&1 | sed 's/^/  | /' >&2 || true
    exit 1
}

# Waits for a command to succeed, then returns. Fails with the engine's log
# attached rather than a bare timeout.
#
# The budget is generous on purpose: a first-boot MySQL initializes far more
# slowly than MariaDB or PostgreSQL, and slowly again on a cold runner. A
# too-short wait and a genuinely broken engine look identical from here, so the
# wait is long enough that a timeout means something is actually wrong.
await() {
    what="$1"
    shift
    i=0
    until "$@" >/dev/null 2>&1; do
        i=$((i + 1))
        if [ "$i" -gt 150 ]; then
            # Run the probe once more with its output kept. Without this the
            # timeout says only that the probe never succeeded, never why --
            # and "why" was, twice, something no amount of staring at the
            # script would have revealed.
            printf '\n  the last probe said:\n' >&2
            "$@" 2>&1 | sed 's/^/  | /' >&2 || true
            fail "$what did not become ready within 300s"
        fi
        sleep 2
    done
}

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
  # Every connection is TCP, never the local socket. Two reasons, both learned
  # the hard way in CI:
  #
  #   * The socket path is not stable across images. The runner's MySQL served
  #     `/var/lib/mysql/mysql.sock` while the client defaulted elsewhere, so the
  #     probe never connected and the job burned its whole budget.
  #   * These images run a *temporary* server during initialization, and it is
  #     started with networking disabled. A socket probe can therefore answer
  #     from the temp server; a TCP probe cannot. That is the readiness race
  #     stated exactly (`C0.12`).
  pg() { $CONTAINER exec -e PGPASSWORD="$PASS" "$NAME" psql -h 127.0.0.1 -U postgres -d openehr "$@"; }
  await PostgreSQL pg -c 'SELECT 1'
  apply() {
    $CONTAINER exec -i -e PGPASSWORD="$PASS" "$NAME" \
      psql -h 127.0.0.1 -U postgres -d openehr -v ON_ERROR_STOP=1 -f - <"$sql" 2>&1 |
      grep -E '^(psql:.*)?ERROR' || true
  }
  # A row must exist before the mutations: a FOR EACH ROW trigger on zero rows
  # never fires, so an empty table reports refusal it never performed.
  seed() {
    $CONTAINER exec -i -e PGPASSWORD="$PASS" "$NAME" \
      psql -h 127.0.0.1 -U postgres -d openehr -q >/dev/null 2>&1 <<'SEED'
INSERT INTO openehr_ehr VALUES ('e1','sys','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','st1','ac1');
INSERT INTO openehr_versioned_object VALUES ('vo1','e1','VERSIONED_COMPOSITION','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
INSERT INTO openehr_contribution VALUES ('c1','e1','249','sys','Committer','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
INSERT INTO openehr_version (uid, versioned_object_uid, creating_system_id, trunk_version,
  lifecycle_state_code, is_deleted, contribution_uid, audit_system_id,
  audit_change_type_code, audit_time_committed_text, data_json,
  chain_previous, chain_content, chain_digest)
VALUES ('vo1::sys::1','vo1','sys',1,'532',false,'c1','sys','249','2026-01-01T00:00:00Z',
  '{"z":1,"a":2,"magnitude":1.10,"dup":"first"}',
  decode(repeat('00',32),'hex'), decode(repeat('11',32),'hex'), decode(repeat('22',32),'hex'));
SEED
  }
  rows() { pg -Atc "SELECT count(*) FROM openehr_version"; }
  json_out() { pg -Atc "SELECT data_json FROM openehr_version WHERE uid = 'vo1::sys::1'"; }
  refuse() { pg -c "$1" 2>&1 | grep -c 'append-only' || true; }
  ;;
mysql)
  IMAGE=docker.io/library/mysql:8.4
  $CONTAINER run -d --rm --name "$NAME" -e MYSQL_ROOT_PASSWORD="$PASS" \
    -e MYSQL_DATABASE=openehr "$IMAGE" >/dev/null
  # Not `mysqladmin ping`: see the note in the postgresql branch. MySQL's
  # entrypoint starts a temporary server to run initialization, and ping answers
  # for it while MYSQL_DATABASE does not yet exist. This loop waits for a real
  # query against the real database.
  # TCP, not the socket: see the note in the postgresql branch. This is the
  # difference that made this job, and only this job, fail in CI.
  await MySQL $CONTAINER exec "$NAME" \
    mysql -h 127.0.0.1 --protocol=TCP -uroot -p"$PASS" openehr -e 'SELECT 1'
  # `|| true` because grep exits 1 when it filters every line, and a bare
  # `seed` call under `set -e` would then abort the script before `fail` could
  # report why — which is how this script first appeared to fail with no message.
  my() {
    $CONTAINER exec -i "$NAME" \
      mysql -h 127.0.0.1 --protocol=TCP -uroot -p"$PASS" openehr "$@" 2>&1 |
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
  audit_change_type_code, audit_time_committed_text, data_json,
  chain_previous, chain_content, chain_digest)
VALUES ('vo1::sys::1','vo1','sys',1,'532',0,'c1','sys','249','2026-01-01T00:00:00Z',
  '{"z":1,"a":2,"magnitude":1.10,"dup":"first"}',
  UNHEX(REPEAT('00',32)), UNHEX(REPEAT('11',32)), UNHEX(REPEAT('22',32)));
SEED
  }
  rows() { my -Nse "SELECT count(*) FROM openehr_version"; }
  json_out() { my -Nse "SELECT data_json FROM openehr_version WHERE uid = 'vo1::sys::1'"; }
  refuse() { my -e "$1" | grep -c 'append-only' || true; }
  ;;
mariadb)
  IMAGE=docker.io/library/mariadb:11.4
  $CONTAINER run -d --rm --name "$NAME" -e MARIADB_ROOT_PASSWORD="$PASS" \
    -e MARIADB_DATABASE=openehr "$IMAGE" >/dev/null
  # A real query against the real database, not `mariadb-admin ping`: see the
  # note in the postgresql branch. This branch happened to pass in CI, which is
  # luck rather than a difference — the same race is present.
  # TCP, not the socket: see the note in the postgresql branch.
  await MariaDB $CONTAINER exec "$NAME" \
    mariadb -h 127.0.0.1 --protocol=TCP -uroot -p"$PASS" openehr -e 'SELECT 1'
  # The client is `mariadb`, not `mysql`: MariaDB 11 renamed every binary and
  # the compatibility symlinks are deprecated. Using `mysql` here would work
  # today and break on the release that drops them, which is the sort of
  # difference this branch exists to keep visible.
  my() {
    $CONTAINER exec -i "$NAME" \
      mariadb -h 127.0.0.1 --protocol=TCP -uroot -p"$PASS" openehr "$@" 2>&1 |
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
  audit_change_type_code, audit_time_committed_text, data_json,
  chain_previous, chain_content, chain_digest)
VALUES ('vo1::sys::1','vo1','sys',1,'532',0,'c1','sys','249','2026-01-01T00:00:00Z',
  '{"z":1,"a":2,"magnitude":1.10,"dup":"first"}',
  UNHEX(REPEAT('00',32)), UNHEX(REPEAT('11',32)), UNHEX(REPEAT('22',32)));
SEED
  }
  rows() { my -Nse "SELECT count(*) FROM openehr_version"; }
  json_out() { my -Nse "SELECT data_json FROM openehr_version WHERE uid = 'vo1::sys::1'"; }
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

# M3.43: canonical JSON must come back as the bytes it went in as.
#
# The chain's content digest is SHA-256 over those bytes (M3.16), so a column
# that returns an equivalent document returns a value the digest cannot be
# recomputed from. The probe is built to fail loudly against a normalizing type:
# the keys are deliberately not sorted, and the magnitude carries a trailing
# zero that openEHR treats as precision (lib:J9.13).
#
# This is the check that does not depend on a list of bad type names. The
# denylist in `conformance::check_dialect` catches `jsonb` and `json` at
# `cargo test`; this catches whatever the engine actually does. D-08 was found
# because `jsonb` and MySQL's `JSON` were both in use and neither round-tripped
# — MySQL rewrote 1.10 as 1.1.
JSON_IN='{"z":1,"a":2,"magnitude":1.10,"dup":"first"}'
JSON_OUT=$(json_out)
[ "$JSON_OUT" = "$JSON_IN" ] || fail "canonical JSON did not round-trip (M3.43, D-08)
  in:  $JSON_IN
  out: $JSON_OUT"
printf 'json byte-exact '

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
