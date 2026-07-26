#!/usr/bin/env bash
set -euo pipefail

main() {
  local args="$*"
  if [[ "$args" == *'-rn'* ]]; then
    local value
    value="${args#*--arg value }"
    value="${value%% *}"
    printf '%s\n' "${value//\//%2F}"
    return 0
  fi

  local input
  input="$(cat)"
  if [[ "$args" == *'.number | integers'* && "$input" =~ \"number\":([0-9]+) ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi
  if [[ "$args" == *'.head_branch | strings'* && "$input" =~ \"head_branch\":\"([^\"]+)\" ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi

  echo "unexpected jq invocation: $args with input: $input" >&2
  return 1
}

main "$@"
