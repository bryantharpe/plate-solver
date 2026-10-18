#!/usr/bin/env bash
# e2e-eval.sh — does the product solve real images, end to end, the way the oracle does?
#
# WHY THIS EXISTS
# ---------------
# On 2026-10-17, main (a275521) had merged through every gate in ci.yml and it solved
# 0 of the 21 reference images. cedar solves 19. (docs/audit-e2e-parity.md.) Every
# other gate asks whether the code is GOOD (rustfmt, clippy, coverage, docs, deny).
# None asked whether it WORKS. The 156 unit tests ran against empty databases and
# symmetric coordinates: tests that could not fail.
#
# Parity with cedar is the product's correctness definition (ENVIRONMENT.md), so this
# gate blocks every PR.
#
# THE TIERS — each anchored to the independent oracle (cedar), on the product's own
# entry points, with the real database:
#
#   E1  centroid parity    crates/plate-solver/tests/reference_parity.rs  identical stars, <=5",
#                          with and without an FOV estimate
#   E2  image entry point  crates/plate-solver/tests/image_parity.rs      JPEG -> solve_from_image, <=10"
#   E3  gRPC service       crates/grpc-service/tests/e2e_service.rs       over the wire, <=10"
#
# WHAT IT REFUSES TO TRUST. A required check is a name; everything that decides what
# runs under that name is checked here first:
#   - no repo-level cargo config (.cargo/config*): a `runner` or an `[env]` there can
#     wrap or steer every test binary without touching a test file;
#   - each tier's test target resolves to the owned file above (`cargo metadata`): a
#     `[[test]]` entry in a Cargo.toml can otherwise point the name at another file;
#   - the run leaves every tracked file as it found it: a build script or a test that
#     rewrites the fixture, the database or a test mid-run fails here.
#
# FAIL CLOSED. Each tier writes its scorecard before it asserts; e2e-scorecard.sh
# fails on a missing, malformed or red one, and names a tier that crashed before
# writing one. Every tier runs even when an earlier one fails.
#
# ARMED BY DEFAULT. Disarming needs E2E_ARMED=false-until-YYYY-MM-DD (UTC), for a
# from-specs rebuild that cannot solve yet. Until that date the tiers still run and
# publish the score, nothing fails, and the attestation says DISARMED; after it the
# gate re-arms by itself. Any other value — unset, empty, plain "false", a typo — is
# armed. Setting the Actions variable takes repo admin (gh variable set E2E_ARMED);
# note that the e2e tests also run inside test-coverage, which this does not disarm.
#
# EXIT: 0 = every tier green (or disarmed) · 1 = a tier failed or cannot be trusted.

set -uo pipefail

: "${E2E_REPORT_DIR:?E2E_REPORT_DIR must be set}"
dir="$E2E_REPORT_DIR"
mkdir -p "$dir"
# Remove only what this script writes: never rm -rf a directory someone passed in.
rm -f "$dir"/E1.json "$dir"/E2.json "$dir"/E3.json "$dir"/E1.log "$dir"/E2.log "$dir"/E3.log
export E2E_REPORT_DIR="$dir"

disarmed_until=""
if [[ "${E2E_ARMED:-}" =~ ^false-until-([0-9]{4}-[0-9]{2}-[0-9]{2})$ ]]; then
  until_date="${BASH_REMATCH[1]}"
  if [[ "$(date -u +%F)" < "$until_date" || "$(date -u +%F)" == "$until_date" ]]; then
    disarmed_until="$until_date"
  else
    echo "::warning::E2E_ARMED disarm expired on $until_date — the gate is armed again."
  fi
elif [[ -n "${E2E_ARMED:-}" ]]; then
  echo "::warning::E2E_ARMED='${E2E_ARMED}' is not false-until-YYYY-MM-DD — the gate stays armed."
fi
export E2E_DISARMED_UNTIL="$disarmed_until"

finish() { # STATUS — apply the arming rule and exit
  if [[ -n "$disarmed_until" ]]; then
    echo "::warning::e2e-eval is DISARMED until $disarmed_until (vars.E2E_ARMED): reported, not enforced."
    exit 0
  fi
  exit "$1"
}

status=0
fail() { echo "::error::$*"; status=1; }

# ── Preflight: is the thing we are about to run the thing we mean to run? ────────────
if [[ ! -f Cargo.toml ]]; then
  fail "no Cargo workspace at the repo root: there is no product to evaluate, so it cannot have passed"
  bash "$(dirname "$0")/e2e-scorecard.sh" "$dir"
  finish 1
fi
for cfg in .cargo/config .cargo/config.toml; do
  [[ -e "$cfg" ]] && fail "$cfg exists: a repo-level cargo config can wrap or steer test binaries. e2e-eval will not run under one; if the repo needs one, teach this script about it (an owned path)."
done
expect_target() { # PACKAGE TARGET RELATIVE_PATH
  local got
  got="$(cargo metadata --format-version 1 --no-deps --locked 2>/dev/null | jq -r --arg p "$1" --arg t "$2" \
    '.packages[] | select(.name == $p) | .targets[] | select(.name == $t and (.kind | index("test"))) | .src_path')"
  if [[ "$got" != "$PWD/$3" ]]; then
    fail "test target $1::$2 resolves to '${got:-nothing}', not $3 — a Cargo.toml [[test]] entry can point the name at another file"
  fi
}
expect_target plate-solver reference_parity crates/plate-solver/tests/reference_parity.rs
expect_target plate-solver image_parity crates/plate-solver/tests/image_parity.rs
expect_target grpc-service e2e_service crates/grpc-service/tests/e2e_service.rs
if ((status)); then
  bash "$(dirname "$0")/e2e-scorecard.sh" "$dir"
  finish 1
fi

# ── The tiers, each its own run so a red one cannot hide the others ─────────────────
# Snapshot the tracked tree without touching it (`git stash create` writes a commit
# object only), so the files the run changed can be named afterwards.
snap() { local s; s="$(git stash create 2>/dev/null)"; echo "${s:-$(git rev-parse HEAD)}"; }
before="$(snap)"
run_tier() { # TIER PACKAGE TEST
  local log="$dir/$1.log"
  if cargo test --release --locked -p "$2" --test "$3" 2>&1 | tee "$log"; then
    return 0
  fi
  return 1
}
run_tier E1 plate-solver reference_parity || status=1
run_tier E2 plate-solver image_parity || status=1
run_tier E3 grpc-service e2e_service || status=1
changed="$(git diff --name-only "$before" "$(snap)")"
if [[ -n "$changed" ]]; then
  fail "the eval run modified tracked files — a build script or test rewrote something it reads: ${changed//$'\n'/, }"
fi

bash "$(dirname "$0")/e2e-scorecard.sh" "$dir" || status=1
finish "$status"
