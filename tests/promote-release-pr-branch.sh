#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly PROMOTE_SCRIPT="$REPO_ROOT/.github/scripts/promote-release-pr-branch.sh"
readonly FAKE_GH="$REPO_ROOT/tests/fixtures/fake-gh.sh"
readonly FAKE_JQ="$REPO_ROOT/tests/fixtures/fake-jq.sh"
readonly STAGING_BRANCH="release-plz/2026-07-26T00-00-00Z"

fail() {
  echo "$*" >&2
  exit 1
}

new_state() {
  local state_dir
  state_dir="$(mktemp -d)"
  : > "$state_dir/calls"
  printf '%s' "$1" > "$state_dir/head"
  printf '%s' "$2" > "$state_dir/canonical"
  printf '%s' "$3" > "$state_dir/staging"
  echo "$state_dir"
}

run_promotion() {
  local state_dir="$1"
  local release_pr_json="$2"
  env \
    GITHUB_REPOSITORY="runrly/zwd" \
    GH_TOKEN="test-token" \
    GH_BIN="$FAKE_GH" \
    JQ_BIN="$FAKE_JQ" \
    FAKE_GH_STATE="$state_dir" \
    RELEASE_PR_JSON="$release_pr_json" \
    bash "$PROMOTE_SCRIPT"
}

assert_file_equals() {
  local expected="$1"
  local path="$2"
  [[ "$(<"$path")" == "$expected" ]] \
    || fail "expected $path to equal $expected, found $(<"$path")"
}

test_canonical_branch_is_a_noop() {
  local state_dir
  state_dir="$(new_state "release-plz/main" "canonical-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  run_promotion "$state_dir" '{"number":42,"head_branch":"release-plz/main"}'
  assert_file_equals "canonical-sha" "$state_dir/canonical"
  [[ ! -s "$state_dir/calls" || "$(<"$state_dir/calls")" != *"--method DELETE"* ]] \
    || fail "canonical branch should not be deleted"
}

test_staging_branch_replaces_canonical_branch() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "old-canonical-sha" "staging-sha")"
  trap 'rm -rf "$state_dir"' RETURN

  run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"
  assert_file_equals "release-plz/main" "$state_dir/head"
  assert_file_equals "staging-sha" "$state_dir/canonical"
  assert_file_equals "" "$state_dir/staging"
}

test_unexpected_branch_does_not_mutate_refs() {
  local state_dir
  state_dir="$(new_state "feature/untrusted" "canonical-sha" "staging-sha")"
  trap 'rm -rf "$state_dir"' RETURN

  if run_promotion "$state_dir" '{"number":42,"head_branch":"feature/untrusted"}'; then
    fail "unexpected branch must fail"
  fi
  assert_file_equals "canonical-sha" "$state_dir/canonical"
  assert_file_equals "staging-sha" "$state_dir/staging"
}

test_rename_failure_restores_previous_canonical_branch() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "old-canonical-sha" "staging-sha")"
  trap 'rm -rf "$state_dir"' RETURN

  if FAKE_GH_FAIL_RENAME=true run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"; then
    fail "rename failure must fail"
  fi
  assert_file_equals "old-canonical-sha" "$state_dir/canonical"
  assert_file_equals "staging-sha" "$state_dir/staging"
}

test_canonical_branch_is_a_noop
test_staging_branch_replaces_canonical_branch
test_unexpected_branch_does_not_mutate_refs
test_rename_failure_restores_previous_canonical_branch
