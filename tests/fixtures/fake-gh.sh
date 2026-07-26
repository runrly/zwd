#!/usr/bin/env bash
set -euo pipefail

readonly canonical_branch="release-plz/main"
readonly staging_branch="release-plz/2026-07-26T00-00-00Z"

state_value() {
  printf '%s' "$(<"$FAKE_GH_STATE/$1")"
}

set_state_value() {
  printf '%s' "$2" > "$FAKE_GH_STATE/$1"
}

record_call() {
  printf '%s\n' "$*" >> "$FAKE_GH_STATE/calls"
}

main() {
  record_call "$@"
  local args="$*"

  if [[ "$args" == *"/pulls/42"* ]]; then
    state_value head
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

  if [[ "$args" == *"--method DELETE"*"git/refs/heads/$canonical_branch"* ]]; then
    set_state_value canonical ""
    return 0
  fi

  if [[ "$args" == *"branches/release-plz%2F2026-07-26T00-00-00Z/rename"* ]]; then
    [[ "${FAKE_GH_FAIL_RENAME:-false}" != true ]] || return 1
    set_state_value canonical "$(state_value staging)"
    set_state_value staging ""
    set_state_value head "$canonical_branch"
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
