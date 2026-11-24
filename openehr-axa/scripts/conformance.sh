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
# Requires only `curl`.
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

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
