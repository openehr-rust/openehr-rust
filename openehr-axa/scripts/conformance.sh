#!/bin/sh
# A conformance runner for the ITS-REST surface, bring-your-own-SUT.
#
#   usage: conformance.sh <base-url> <auth-header-value> <existing-ehr-id>
#
# `<base-url>` is everything before `/ehr` — for this crate,
# `http://localhost:5150/openehr/v1`; for EHRbase, something shaped like
# `http://localhost:8080/ehrbase/rest/openehr/v1`. `<auth-header-value>` is
# passed verbatim as the `Authorization` header, so it works for any scheme
# the target actually uses (`Bearer <PASETO token>` here, `Basic <base64>`
# for EHRbase's default). `<existing-ehr-id>` names an EHR the target already
# has — see below for why creation is not this script's job.
#
# Requires `curl`, `jq` and `uuidgen` (the creation cases read response bodies).
#
# # Why this tests two endpoints of the eleven, not all eleven
#
# `spec/conformance-runs.md` has the full account, verified against a real,
# running EHRbase 2.35.1 rather than assumed from its documentation:
#
#   - `POST /ehr` genuinely cannot be one shared request. This crate expects
#     a caller-built `Ehr` object; EHRbase's own OpenAPI (`/v3/api-docs`)
#     shows it expects query parameters or an `EHR_STATUS` body and builds
#     the `Ehr` itself. Neither is more "correct" without deciding what this
#     crate's own `POST /ehr` contract should be, which is a design question
#     for the "ITS-REST completeness" item, not this script.
#   - Every composition and contribution case is blocked on a real server:
#     EHRbase refuses a composition with no `archetype_details/template_id`
#     ("Composition missing mandatory attribute"), and creating one needs an
#     uploaded, valid OPT — infrastructure `openehr` does not have yet (the
#     "OPT 1.4 ingestion" item). A catalogue that skipped this and posted a
#     template-less composition would not be testing ITS-REST; it would be
#     testing what EHRbase tolerates when asked incorrectly.
#   - `GET /composition` (search) is not part of ITS-REST at all — it does
#     not appear in EHRbase's own OpenAPI paths — and `/metadata` is this
#     crate's own invention, not a spec endpoint either. Neither has a
#     reference to check against.
#
# What is left, and genuinely shared: reading an EHR that exists, and reading
# one that does not. Small, but every assertion below was checked against a
# real answer from a real EHRbase, not read off its documentation.

set -eu

BASE="${1:?usage: conformance.sh <base-url> <auth-header-value> <existing-ehr-id>}"
AUTH="${2:?usage: conformance.sh <base-url> <auth-header-value> <existing-ehr-id>}"
EHR_ID="${3:?usage: conformance.sh <base-url> <auth-header-value> <existing-ehr-id>}"
MISSING_ID="00000000-0000-0000-0000-000000000000"

pass=0
fail=0

# `$1` name, `$2` URL, `$3` expected status.
case_get() {
    name="$1"
    url="$2"
    want="$3"
    body=$(curl -s -w '\n%{http_code}' -H "Authorization: $AUTH" "$url")
    got=$(printf '%s' "$body" | tail -1)
    if [ "$got" = "$want" ]; then
        printf 'PASS  %-28s %s (wanted %s, got %s)\n' "$name" "$url" "$want" "$got"
        pass=$((pass + 1))
    else
        printf 'FAIL  %-28s %s (wanted %s, got %s)\n' "$name" "$url" "$want" "$got"
        printf '      body: %s\n' "$(printf '%s' "$body" | sed '$d')"
        fail=$((fail + 1))
    fi
}

case_get "read_existing_ehr" "$BASE/ehr/$EHR_ID" "200"
case_get "read_missing_ehr" "$BASE/ehr/$MISSING_ID" "404"

# --- creating an EHR, ITS-REST 1.1.0 (db:S1.22) ------------------------------
#
# Each case checks status, headers and body together. Where ITS-REST states the
# response shape, the check uses ITS-REST's shape and accepts no other: an
# implementation that answers in its own shape fails the case. That is the
# finding to report, not something for this runner to paper over.

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# $1 method, $2 URL, $3 body file or "" for none. Leaves headers in $TMP/h,
# body in $TMP/b, and prints the status.
send() {
    if [ -n "$3" ]; then
        curl -s -D "$TMP/h" -o "$TMP/b" -w '%{http_code}' -X "$1" \
            -H "Authorization: $AUTH" -H "Prefer: return=representation" \
            -H "Content-Type: application/json" --data-binary "@$3" "$2"
    else
        curl -s -D "$TMP/h" -o "$TMP/b" -w '%{http_code}' -X "$1" \
            -H "Authorization: $AUTH" -H "Prefer: return=representation" "$2"
    fi
}

# $1 name, $2 0 when the case held, $3 what was seen. Each condition is
# evaluated as `cond && ok=0 || ok=1`, never alone: under `set -e` a lone
# failing `[ ... ]` would end the script before its verdict was printed.
verdict() {
    if [ "$2" -eq 0 ]; then
        printf 'PASS  %-28s %s\n' "$1" "$3"
        pass=$((pass + 1))
    else
        printf 'FAIL  %-28s %s\n' "$1" "$3"
        printf '      body: %s\n' "$(head -c 400 "$TMP/b")"
        fail=$((fail + 1))
    fi
}

has_header() { grep -qi "^$1:" "$TMP/h"; }

# 1. The default status: no body. 201, a Location, and the new ehr_id.
got=$(send POST "$BASE/ehr" "")
new_id=$(jq -r '.ehr_id.value // empty' "$TMP/b" 2>/dev/null || true)
if has_header location; then location=present; else location=absent; fi
[ "$got" = 201 ] && [ "$location" = present ] && [ -n "$new_id" ] && ok=0 || ok=1
verdict "create_default_status" "$ok" "POST /ehr -> $got, Location $location, ehr_id ${new_id:-none}"

# 2. Read straight back: the status exists at once, and is the default.
#    ITS-REST answers with the EHR_STATUS itself.
if [ -n "$new_id" ]; then
    got=$(send GET "$BASE/ehr/$new_id/ehr_status" "")
    modifiable=$(jq -r '.is_modifiable | tostring' "$TMP/b" 2>/dev/null || true)
    queryable=$(jq -r '.is_queryable | tostring' "$TMP/b" 2>/dev/null || true)
    [ "$got" = 200 ] && [ "$modifiable" = true ] && [ "$queryable" = true ] && ok=0 || ok=1
    verdict "read_default_status" "$ok" "GET .../ehr_status -> $got, is_modifiable ${modifiable:-absent}, is_queryable ${queryable:-absent}"
fi

# 2b. Its revision history: one item, a creation, with a committer (db:S1.24).
if [ -n "$new_id" ]; then
    got=$(send GET "$BASE/ehr/$new_id/versioned_ehr_status/revision_history" "")
    count=$(jq -r '.items | length' "$TMP/b" 2>/dev/null || true)
    change=$(jq -r '.items[0].audits[0].change_type.defining_code.code_string // empty' "$TMP/b" 2>/dev/null || true)
    committer=$(jq -r '.items[0].audits[0].committer._type // empty' "$TMP/b" 2>/dev/null || true)
    [ "$got" = 200 ] && [ "$count" = 1 ] && [ "$change" = 249 ] && [ -n "$committer" ] && ok=0 || ok=1
    verdict "status_revision_history" "$ok" "GET .../versioned_ehr_status/revision_history -> $got, items ${count:-none}, change ${change:-none}, committer ${committer:-absent}"
fi

# 3. A supplied status is the one committed. It carries archetype_details: an
#    EHR_STATUS is an archetype root (RM EHR_STATUS.Is_archetype_root), and this
#    service refuses one without it by decision (db:S1.23, db:D-32), where
#    EHRbase accepts it. The case tests what both should share.
cat >"$TMP/status.json" <<'JSON'
{"_type":"EHR_STATUS","archetype_node_id":"openEHR-EHR-EHR_STATUS.generic.v1","archetype_details":{"archetype_id":{"value":"openEHR-EHR-EHR_STATUS.generic.v1"},"rm_version":"1.1.0"},"name":{"_type":"DV_TEXT","value":"EHR Status"},"subject":{"_type":"PARTY_SELF"},"is_modifiable":false,"is_queryable":true}
JSON
got=$(send POST "$BASE/ehr" "$TMP/status.json")
supplied_id=$(jq -r '.ehr_id.value // empty' "$TMP/b" 2>/dev/null || true)
[ "$got" = 201 ] && [ -n "$supplied_id" ] && ok=0 || ok=1
verdict "create_supplied_status" "$ok" "POST /ehr (EHR_STATUS body) -> $got, ehr_id ${supplied_id:-none}"
if [ -n "$supplied_id" ]; then
    got=$(send GET "$BASE/ehr/$supplied_id/ehr_status" "")
    modifiable=$(jq -r '.is_modifiable | tostring' "$TMP/b" 2>/dev/null || true)
    [ "$got" = 200 ] && [ "$modifiable" = false ] && ok=0 || ok=1
    verdict "read_supplied_status" "$ok" "GET .../ehr_status -> $got, is_modifiable ${modifiable:-absent}"
fi

# 4. A body that is not an EHR_STATUS is the caller's error.
printf '{not json' >"$TMP/bad.json"
got=$(send POST "$BASE/ehr" "$TMP/bad.json")
[ "$got" = 400 ] && ok=0 || ok=1
verdict "create_invalid_body" "$ok" "POST /ehr (malformed body) -> $got, wanted 400"

# 5. A named id: created once, then a conflict.
named=$(uuidgen | tr 'A-Z' 'a-z')
got=$(send PUT "$BASE/ehr/$named" "")
[ "$got" = 201 ] && ok=0 || ok=1
verdict "create_named" "$ok" "PUT /ehr/{id} -> $got, wanted 201"
got=$(send PUT "$BASE/ehr/$named" "")
[ "$got" = 409 ] && ok=0 || ok=1
verdict "create_named_twice" "$ok" "PUT /ehr/{id} again -> $got, wanted 409"

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
