#!/usr/bin/env bash
set -euo pipefail

readonly CANONICAL_BRANCH="release-plz/main"
readonly STAGING_PREFIX="release-plz/"
: "${GH_BIN:=gh}"
: "${JQ_BIN:=jq}"

fail() {
  echo "$*" >&2
  exit 1
}

require_environment() {
  : "${GITHUB_REPOSITORY:?missing repository context}"
  : "${GH_TOKEN:?missing GitHub App token}"
  : "${RELEASE_PR_JSON:?missing release-plz PR output}"
}

matching_ref_sha() {
  local branch="$1"
  "$GH_BIN" api "repos/$GITHUB_REPOSITORY/git/matching-refs/heads/$branch" \
    --jq ".[] | select(.ref == \"refs/heads/$branch\") | .object.sha"
}

create_ref() {
  local branch="$1"
  local sha="$2"
  "$GH_BIN" api --method POST "repos/$GITHUB_REPOSITORY/git/refs" \
    -f "ref=refs/heads/$branch" \
    -f "sha=$sha" > /dev/null
}

update_ref() {
  local branch="$1"
  local sha="$2"
  "$GH_BIN" api --method PATCH "repos/$GITHUB_REPOSITORY/git/refs/heads/$branch" \
    -F force=true \
    -f "sha=$sha" > /dev/null
}

delete_ref() {
  local branch="$1"
  "$GH_BIN" api --method DELETE "repos/$GITHUB_REPOSITORY/git/refs/heads/$branch" > /dev/null
}

set_canonical_ref() {
  local previous_sha="$1"
  local staging_sha="$2"

  if [[ -n "$previous_sha" ]]; then
    update_ref "$CANONICAL_BRANCH" "$staging_sha"
  else
    create_ref "$CANONICAL_BRANCH" "$staging_sha"
  fi
}

restore_canonical_ref() {
  local previous_sha="$1"
  local current_sha
  current_sha="$(matching_ref_sha "$CANONICAL_BRANCH")"

  if [[ -z "$previous_sha" ]]; then
    [[ -z "$current_sha" ]] || delete_ref "$CANONICAL_BRANCH"
    return 0
  fi

  if [[ -z "$current_sha" ]]; then
    create_ref "$CANONICAL_BRANCH" "$previous_sha"
  elif [[ "$current_sha" != "$previous_sha" ]]; then
    update_ref "$CANONICAL_BRANCH" "$previous_sha"
  fi
}

release_pr_number() {
  "$JQ_BIN" -er '
    .number as $number
    | if ($number | type) != "number" then
        error("release PR number must be a positive integer")
      elif $number <= 0 or $number != ($number | floor) then
        error("release PR number must be a positive integer")
      else
        $number
      end
  ' <<<"$RELEASE_PR_JSON"
}

verify_pr_head() {
  local pr_number="$1"
  local expected_branch="$2"
  local actual_branch
  actual_branch="$("$GH_BIN" api "repos/$GITHUB_REPOSITORY/pulls/$pr_number" --jq '.head.ref')"
  [[ "$actual_branch" == "$expected_branch" ]] \
    || fail "release PR #$pr_number head is $actual_branch, expected $expected_branch"
}

open_canonical_pr_number() {
  local owner="${GITHUB_REPOSITORY%%/*}"
  "$GH_BIN" api \
    "repos/$GITHUB_REPOSITORY/pulls?state=open&head=$owner:$CANONICAL_BRANCH&base=main" \
    --jq 'if length == 0 then empty elif length == 1 then .[0].number else error("multiple canonical release PRs") end'
}

create_canonical_pr() {
  local title="$1"
  local body="$2"
  local base_branch="$3"
  local draft="$4"
  "$GH_BIN" api --method POST "repos/$GITHUB_REPOSITORY/pulls" \
    -f "title=$title" \
    -f "head=$CANONICAL_BRANCH" \
    -f "base=$base_branch" \
    -f "body=$body" \
    -F "draft=$draft" \
    --jq '.number'
}

copy_pr_labels() {
  local metadata="$1"
  local canonical_pr_number="$2"
  local labels=()
  local label

  mapfile -t labels < <("$JQ_BIN" -r '.labels[]?.name | strings' <<<"$metadata")
  for label in "${labels[@]}"; do
    "$GH_BIN" api --method POST "repos/$GITHUB_REPOSITORY/issues/$canonical_pr_number/labels" \
      -f "labels[]=$label" > /dev/null
  done
}

close_pr() {
  "$GH_BIN" api --method PATCH "repos/$GITHUB_REPOSITORY/pulls/$1" -f state=closed > /dev/null
}

reopen_pr() {
  "$GH_BIN" api --method PATCH "repos/$GITHUB_REPOSITORY/pulls/$1" -f state=open > /dev/null
}

rollback_replacement() {
  local staging_pr_number="$1"
  local canonical_pr_number="$2"
  local previous_canonical_sha="$3"
  local staging_pr_closed="$4"
  local rollback_failed=false

  if [[ "$staging_pr_closed" == true ]] && ! reopen_pr "$staging_pr_number"; then
    rollback_failed=true
  fi
  if ! close_pr "$canonical_pr_number"; then
    rollback_failed=true
  fi
  if ! restore_canonical_ref "$previous_canonical_sha"; then
    rollback_failed=true
  fi

  [[ "$rollback_failed" == false ]]
}

fail_with_rollback() {
  local reason="$1"
  local staging_pr_number="$2"
  local canonical_pr_number="$3"
  local previous_canonical_sha="$4"
  local staging_pr_closed="$5"

  if ! rollback_replacement \
    "$staging_pr_number" \
    "$canonical_pr_number" \
    "$previous_canonical_sha" \
    "$staging_pr_closed"; then
    fail "$reason; rollback was incomplete"
  fi
  fail "$reason"
}

promote_staging_pr() {
  local staging_pr_number="$1"
  local staging_branch="$2"
  local metadata title body base_branch draft existing_canonical_pr
  local staging_sha previous_canonical_sha canonical_pr_number
  local staging_pr_closed=false

  verify_pr_head "$staging_pr_number" "$staging_branch"
  metadata="$("$GH_BIN" api "repos/$GITHUB_REPOSITORY/pulls/$staging_pr_number")"
  title="$("$JQ_BIN" -er '.title | strings' <<<"$metadata")"
  body="$("$JQ_BIN" -er '(.body // "") | strings' <<<"$metadata")"
  base_branch="$("$JQ_BIN" -er '.base.ref | strings' <<<"$metadata")"
  draft="$("$JQ_BIN" -er 'if .draft == true then "true" elif .draft == false then "false" else error("release PR draft must be boolean") end' <<<"$metadata")"

  existing_canonical_pr="$(open_canonical_pr_number)"
  [[ -z "$existing_canonical_pr" ]] \
    || fail "canonical release PR #$existing_canonical_pr already exists"

  staging_sha="$(matching_ref_sha "$staging_branch")"
  [[ -n "$staging_sha" ]] || fail "staging branch $staging_branch does not exist"
  previous_canonical_sha="$(matching_ref_sha "$CANONICAL_BRANCH")"
  set_canonical_ref "$previous_canonical_sha" "$staging_sha"

  if ! canonical_pr_number="$(create_canonical_pr "$title" "$body" "$base_branch" "$draft")"; then
    restore_canonical_ref "$previous_canonical_sha" \
      || fail "failed to create canonical release PR and restore $CANONICAL_BRANCH"
    fail "failed to create canonical release PR"
  fi

  if ! copy_pr_labels "$metadata" "$canonical_pr_number"; then
    fail_with_rollback \
      "failed to copy release PR labels" \
      "$staging_pr_number" \
      "$canonical_pr_number" \
      "$previous_canonical_sha" \
      "$staging_pr_closed"
  fi

  if ! close_pr "$staging_pr_number"; then
    fail_with_rollback \
      "failed to close staging release PR #$staging_pr_number" \
      "$staging_pr_number" \
      "$canonical_pr_number" \
      "$previous_canonical_sha" \
      "$staging_pr_closed"
  fi
  staging_pr_closed=true

  if ! delete_ref "$staging_branch"; then
    fail_with_rollback \
      "failed to delete staging branch $staging_branch" \
      "$staging_pr_number" \
      "$canonical_pr_number" \
      "$previous_canonical_sha" \
      "$staging_pr_closed"
  fi

  verify_pr_head "$canonical_pr_number" "$CANONICAL_BRANCH"
  [[ "$(matching_ref_sha "$CANONICAL_BRANCH")" == "$staging_sha" ]] \
    || fail "$CANONICAL_BRANCH does not point at the promoted staging commit"
  [[ -z "$(matching_ref_sha "$staging_branch")" ]] \
    || fail "staging branch $staging_branch still exists after replacement"
  echo "replaced staging release PR #$staging_pr_number with canonical release PR #$canonical_pr_number"
}

main() {
  require_environment

  local pr_number head_branch
  pr_number="$(release_pr_number)"
  head_branch="$("$JQ_BIN" -er '.head_branch | strings' <<<"$RELEASE_PR_JSON")"

  if [[ "$head_branch" == "$CANONICAL_BRANCH" ]]; then
    verify_pr_head "$pr_number" "$CANONICAL_BRANCH"
    echo "release PR #$pr_number already uses $CANONICAL_BRANCH"
    return 0
  fi

  [[ "$head_branch" == "$STAGING_PREFIX"?* ]] \
    || fail "release PR #$pr_number has unexpected staging branch $head_branch"
  promote_staging_pr "$pr_number" "$head_branch"
}

main "$@"
