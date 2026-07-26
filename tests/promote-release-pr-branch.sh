#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly PROMOTE_SCRIPT="$REPO_ROOT/.github/scripts/promote-release-pr-branch.sh"
readonly FAKE_GH="$REPO_ROOT/tests/fixtures/fake-gh.sh"
readonly STAGING_BRANCH="release-plz/2026-07-26T00-00-00Z"

fail() {
  echo "$*" >&2
  exit 1
}

state_value() {
  printf '%s' "$(<"$1/$2")"
}

assert_state_equals() {
  local state_dir="$1"
  local file_name="$2"
  local expected="$3"
  local actual
  actual="$(state_value "$state_dir" "$file_name")"
  [[ "$actual" == "$expected" ]] \
    || fail "expected $file_name to equal $expected, found $actual"
}

new_state() {
  local state_dir
  state_dir="$(mktemp -d)"
  : > "$state_dir/calls"
  printf '%s' "$1" > "$state_dir/head"
  printf '%s' "$2" > "$state_dir/canonical"
  printf '%s' "$3" > "$state_dir/staging"
  printf '%s' "$4" > "$state_dir/canonical-pr-state"
  printf '%s' "" > "$state_dir/canonical-pr-head"
  printf '%s' "open" > "$state_dir/staging-pr-state"
  printf '%s' "main" > "$state_dir/base"
  printf '%s' "release: version zwd" > "$state_dir/title"
  printf '%s' "release body" > "$state_dir/body"
  printf '%s' "false" > "$state_dir/draft"
  printf '%s' "release" > "$state_dir/label"
  printf '%s' "" > "$state_dir/canonical-pr-label"
  echo "$state_dir"
}

run_promotion() {
  local state_dir="$1"
  local release_pr_json="$2"
  env \
    GITHUB_REPOSITORY="runrly/zwd" \
    GH_TOKEN="test-token" \
    GH_BIN="$FAKE_GH" \
    FAKE_GH_STATE="$state_dir" \
    RELEASE_PR_JSON="$release_pr_json" \
    bash "$PROMOTE_SCRIPT"
}

test_canonical_branch_is_a_noop() {
  local state_dir
  state_dir="$(new_state "release-plz/main" "canonical-sha" "" "open")"
  trap 'rm -rf "$state_dir"' RETURN

  run_promotion "$state_dir" '{"number":42,"head_branch":"release-plz/main"}'
  assert_state_equals "$state_dir" canonical "canonical-sha"
  [[ ! -s "$state_dir/calls" || "$(<"$state_dir/calls")" != *"--method POST"* ]] \
    || fail "canonical branch should not create another pull request"
}

test_staging_pr_is_replaced_by_canonical_pr() {
  local state_dir calls
  state_dir="$(new_state "$STAGING_BRANCH" "old-canonical-sha" "staging-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"
  assert_state_equals "$state_dir" canonical "staging-sha"
  assert_state_equals "$state_dir" staging ""
  assert_state_equals "$state_dir" staging-pr-state "closed"
  assert_state_equals "$state_dir" canonical-pr-state "open"
  assert_state_equals "$state_dir" canonical-pr-head "release-plz/main"
  assert_state_equals "$state_dir" canonical-pr-label "release"
  calls="$(<"$state_dir/calls")"
  [[ "$calls" == *"title=release: version zwd"* ]] || fail "canonical PR must preserve title"
  [[ "$calls" == *"body=release body"* ]] || fail "canonical PR must preserve body"
  [[ "$calls" == *"base=main"* ]] || fail "canonical PR must preserve base"
  [[ "$calls" == *"draft=false"* ]] || fail "canonical PR must preserve draft state"
}

test_missing_canonical_ref_is_created() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "" "staging-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"
  assert_state_equals "$state_dir" canonical "staging-sha"
}

test_existing_canonical_pr_rejects_staging_pr() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "canonical-sha" "staging-sha" "open")"
  trap 'rm -rf "$state_dir"' RETURN

  if run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"; then
    fail "staging promotion must reject an existing canonical pull request"
  fi
  assert_state_equals "$state_dir" canonical "canonical-sha"
  assert_state_equals "$state_dir" staging "staging-sha"
}

test_unexpected_branch_does_not_mutate_refs() {
  local state_dir
  state_dir="$(new_state "feature/untrusted" "canonical-sha" "staging-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  if run_promotion "$state_dir" '{"number":42,"head_branch":"feature/untrusted"}'; then
    fail "unexpected release branch must fail"
  fi
  assert_state_equals "$state_dir" canonical "canonical-sha"
  assert_state_equals "$state_dir" staging "staging-sha"
  [[ ! -s "$state_dir/calls" ]] || fail "unexpected release branch must not call GitHub"
}

test_failure_creating_canonical_pr_restores_ref() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "old-canonical-sha" "staging-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  if FAKE_GH_FAIL_AT=create-canonical-pr run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"; then
    fail "canonical PR creation failure must fail"
  fi
  assert_state_equals "$state_dir" canonical "old-canonical-sha"
  assert_state_equals "$state_dir" staging "staging-sha"
  assert_state_equals "$state_dir" staging-pr-state "open"
}

test_failure_copying_labels_rolls_back() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "old-canonical-sha" "staging-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  if FAKE_GH_FAIL_AT=copy-labels run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"; then
    fail "label copy failure must fail"
  fi
  assert_state_equals "$state_dir" canonical "old-canonical-sha"
  assert_state_equals "$state_dir" staging "staging-sha"
  assert_state_equals "$state_dir" staging-pr-state "open"
  assert_state_equals "$state_dir" canonical-pr-state "closed"
}

test_failure_closing_staging_pr_rolls_back() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "old-canonical-sha" "staging-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  if FAKE_GH_FAIL_AT=close-staging-pr run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"; then
    fail "staging close failure must fail"
  fi
  assert_state_equals "$state_dir" canonical "old-canonical-sha"
  assert_state_equals "$state_dir" staging "staging-sha"
  assert_state_equals "$state_dir" staging-pr-state "open"
  assert_state_equals "$state_dir" canonical-pr-state "closed"
}

test_failure_deleting_staging_branch_rolls_back() {
  local state_dir
  state_dir="$(new_state "$STAGING_BRANCH" "old-canonical-sha" "staging-sha" "")"
  trap 'rm -rf "$state_dir"' RETURN

  if FAKE_GH_FAIL_AT=delete-staging-branch run_promotion "$state_dir" "{\"number\":42,\"head_branch\":\"$STAGING_BRANCH\"}"; then
    fail "staging branch deletion failure must fail"
  fi
  assert_state_equals "$state_dir" canonical "old-canonical-sha"
  assert_state_equals "$state_dir" staging "staging-sha"
  assert_state_equals "$state_dir" staging-pr-state "open"
  assert_state_equals "$state_dir" canonical-pr-state "closed"
}

test_release_pr_number_must_be_a_positive_integer() {
  local state_dir release_pr_json
  state_dir="$(new_state "release-plz/main" "canonical-sha" "" "open")"
  trap 'rm -rf "$state_dir"' RETURN

  for release_pr_json in \
    '{"number":"42","head_branch":"release-plz/main"}' \
    '{"number":42.5,"head_branch":"release-plz/main"}' \
    '{"number":0,"head_branch":"release-plz/main"}' \
    '{"number":-42,"head_branch":"release-plz/main"}'; do
    if run_promotion "$state_dir" "$release_pr_json"; then
      fail "release PR number must be a positive integer: $release_pr_json"
    fi
  done
  [[ ! -s "$state_dir/calls" ]] || fail "invalid release PR numbers must not call GitHub"
}

test_canonical_branch_is_a_noop
test_staging_pr_is_replaced_by_canonical_pr
test_missing_canonical_ref_is_created
test_existing_canonical_pr_rejects_staging_pr
test_unexpected_branch_does_not_mutate_refs
test_failure_creating_canonical_pr_restores_ref
test_failure_copying_labels_rolls_back
test_failure_closing_staging_pr_rolls_back
test_failure_deleting_staging_branch_rolls_back
test_release_pr_number_must_be_a_positive_integer
