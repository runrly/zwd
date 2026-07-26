#!/usr/bin/env bash
set -euo pipefail

readonly canonical_branch="release-plz/main"
readonly staging_branch="release-plz/2026-07-26T00-00-00Z"
readonly staging_pr_number="42"
readonly canonical_pr_number="43"

state_value() {
  printf '%s' "$(<"$FAKE_GH_STATE/$1")"
}

set_state_value() {
  printf '%s' "$2" > "$FAKE_GH_STATE/$1"
}

record_call() {
  printf '%s\n' "$*" >> "$FAKE_GH_STATE/calls"
}

should_fail() {
  [[ "${FAKE_GH_FAIL_AT:-}" == "$1" ]]
}

pull_metadata() {
  printf '{"number":%s,"head":{"ref":"%s"},"base":{"ref":"%s"},"title":"%s","body":"%s","draft":%s,"labels":[{"name":"%s"}]}' \
    "$staging_pr_number" \
    "$(state_value head)" \
    "$(state_value base)" \
    "$(state_value title)" \
    "$(state_value body)" \
    "$(state_value draft)" \
    "$(state_value label)"
}

main() {
  record_call "$@"
  local args="$*"

  if [[ "$args" == *"pulls?state=open"* ]]; then
    if [[ "$args" == *"--jq"* ]]; then
      if [[ "$(state_value canonical-pr-state)" == "open" ]]; then
        printf '%s\n' "$canonical_pr_number"
      fi
    elif [[ "$(state_value canonical-pr-state)" == "open" ]]; then
      printf '[{"number":%s}]\n' "$canonical_pr_number"
    else
      printf '[]\n'
    fi
    return 0
  fi

  if [[ "$args" == *"/pulls/$staging_pr_number"* && "$args" != *"--method"* ]]; then
    if [[ "$args" == *"--jq .head.ref"* ]]; then
      state_value head
      return 0
    fi
    pull_metadata
    return 0
  fi

  if [[ "$args" == *"/pulls/$canonical_pr_number"* && "$args" != *"--method"* ]]; then
    if [[ "$args" == *"--jq .head.ref"* ]]; then
      state_value canonical-pr-head
      return 0
    fi
    printf '{"number":%s,"head":{"ref":"%s"}}\n' \
      "$canonical_pr_number" "$(state_value canonical-pr-head)"
    return 0
  fi

  if [[ "$args" == *"matching-refs/heads/$canonical_branch"* ]]; then
    state_value canonical
    return 0
  fi

  if [[ "$args" == *"matching-refs/heads/$staging_branch"* ]]; then
    state_value staging
    return 0
  fi

  if [[ "$args" == *"--method POST"*"/pulls"* ]]; then
    should_fail create-canonical-pr && return 1
    set_state_value canonical-pr-state "open"
    set_state_value canonical-pr-head "$canonical_branch"
    printf '%s\n' "$canonical_pr_number"
    return 0
  fi

  if [[ "$args" == *"--method POST"*"issues/$canonical_pr_number/labels"* ]]; then
    should_fail copy-labels && return 1
    set_state_value canonical-pr-label "${args##*labels[]=}"
    return 0
  fi

  if [[ "$args" == *"--method PATCH"*"/pulls/$staging_pr_number"*"state=closed"* ]]; then
    should_fail close-staging-pr && return 1
    set_state_value staging-pr-state "closed"
    return 0
  fi

  if [[ "$args" == *"--method PATCH"*"/pulls/$staging_pr_number"*"state=open"* ]]; then
    set_state_value staging-pr-state "open"
    return 0
  fi

  if [[ "$args" == *"--method PATCH"*"/pulls/$canonical_pr_number"*"state=closed"* ]]; then
    set_state_value canonical-pr-state "closed"
    return 0
  fi

  if [[ "$args" == *"--method PATCH"*"git/refs/heads/$canonical_branch"* ]]; then
    set_state_value canonical "${args##*sha=}"
    return 0
  fi

  if [[ "$args" == *"--method DELETE"*"git/refs/heads/$canonical_branch"* ]]; then
    set_state_value canonical ""
    return 0
  fi

  if [[ "$args" == *"--method DELETE"*"git/refs/heads/$staging_branch"* ]]; then
    should_fail delete-staging-branch && return 1
    set_state_value staging ""
    return 0
  fi

  if [[ "$args" == *"--method POST"*"git/refs"* ]]; then
    set_state_value canonical "${args##*sha=}"
    return 0
  fi

  echo "unexpected gh invocation: $args" >&2
  return 1
}

main "$@"
