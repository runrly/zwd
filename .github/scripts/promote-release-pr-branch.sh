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

encode_path_segment() {
  "$JQ_BIN" -rn --arg value "$1" '$value | @uri'
}

matching_ref_sha() {
  local branch="$1"
  "$GH_BIN" api "repos/$GITHUB_REPOSITORY/git/matching-refs/heads/$branch" \
    --jq ".[] | select(.ref == \"refs/heads/$branch\") | .object.sha"
}

verify_pr_head() {
  local pr_number="$1"
  local expected_branch="$2"
  local actual_branch
  actual_branch="$("$GH_BIN" api "repos/$GITHUB_REPOSITORY/pulls/$pr_number" --jq '.head.ref')"
  [[ "$actual_branch" == "$expected_branch" ]] \
    || fail "release PR #$pr_number head is $actual_branch, expected $expected_branch"
}

restore_canonical_branch() {
  local previous_sha="$1"
  local current_sha

  [[ -n "$previous_sha" ]] || return 0

  current_sha="$(matching_ref_sha "$CANONICAL_BRANCH")"
  if [[ "$current_sha" == "$previous_sha" ]]; then
    return 0
  fi

  [[ -z "$current_sha" ]] \
    || fail "cannot restore $CANONICAL_BRANCH: it now points at unexpected SHA $current_sha"

  "$GH_BIN" api --method POST "repos/$GITHUB_REPOSITORY/git/refs" \
    -f "ref=refs/heads/$CANONICAL_BRANCH" \
    -f "sha=$previous_sha" > /dev/null
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

main() {
  require_environment

  local pr_number head_branch staging_sha previous_canonical_sha encoded_staging_branch
  pr_number="$(release_pr_number)"
  head_branch="$("$JQ_BIN" -er '.head_branch | strings' <<<"$RELEASE_PR_JSON")"

  if [[ "$head_branch" == "$CANONICAL_BRANCH" ]]; then
    verify_pr_head "$pr_number" "$CANONICAL_BRANCH"
    echo "release PR #$pr_number already uses $CANONICAL_BRANCH"
    return 0
  fi

  [[ "$head_branch" == "$STAGING_PREFIX"?* ]] \
    || fail "release PR #$pr_number has unexpected staging branch $head_branch"
  verify_pr_head "$pr_number" "$head_branch"

  staging_sha="$(matching_ref_sha "$head_branch")"
  [[ -n "$staging_sha" ]] || fail "staging branch $head_branch does not exist"
  previous_canonical_sha="$(matching_ref_sha "$CANONICAL_BRANCH")"

  if [[ -n "$previous_canonical_sha" ]]; then
    "$GH_BIN" api --method DELETE \
      "repos/$GITHUB_REPOSITORY/git/refs/heads/$CANONICAL_BRANCH" > /dev/null
  fi

  encoded_staging_branch="$(encode_path_segment "$head_branch")"
  if ! "$GH_BIN" api --method POST \
    "repos/$GITHUB_REPOSITORY/branches/$encoded_staging_branch/rename" \
    -f "new_name=$CANONICAL_BRANCH" > /dev/null; then
    restore_canonical_branch "$previous_canonical_sha"
    fail "failed to promote $head_branch to $CANONICAL_BRANCH; the staging branch was retained"
  fi

  verify_pr_head "$pr_number" "$CANONICAL_BRANCH"
  [[ "$(matching_ref_sha "$CANONICAL_BRANCH")" == "$staging_sha" ]] \
    || fail "$CANONICAL_BRANCH does not point at the promoted staging commit"
  [[ -z "$(matching_ref_sha "$head_branch")" ]] \
    || fail "staging branch $head_branch still exists after promotion"
  echo "promoted release PR #$pr_number to $CANONICAL_BRANCH"
}

main "$@"
