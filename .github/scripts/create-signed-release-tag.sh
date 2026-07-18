#!/usr/bin/env bash
set -euo pipefail

: "${RUNNER_TEMP:?missing runner temp directory}"
readonly KEY_PATH="$RUNNER_TEMP/zwd-release-signing-key"
readonly PUBLIC_KEY_PATH="$KEY_PATH.pub"
readonly ALLOWED_SIGNERS_PATH="$RUNNER_TEMP/zwd-release-allowed-signers"
readonly TAGGER_NAME="runrly-echo"
readonly TAGGER_EMAIL="echo@runrly.dev"

require_environment() {
  : "${RUNRLY_ECHO_SIGNING_PRIVATE_KEY:?missing release-signing secret}"
}

cleanup() {
  rm -f "$KEY_PATH" "$PUBLIC_KEY_PATH" "$ALLOWED_SIGNERS_PATH"
}

package_version() {
  cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | select(.name == "zwd") | .version'
}

configure_signing_key() {
  umask 077
  printf '%s\n' "$RUNRLY_ECHO_SIGNING_PRIVATE_KEY" > "$KEY_PATH"
  ssh-keygen -y -f "$KEY_PATH" > "$PUBLIC_KEY_PATH"

  git config user.name "$TAGGER_NAME"
  git config user.email "$TAGGER_EMAIL"
  git config gpg.format ssh
  git config user.signingkey "$PUBLIC_KEY_PATH"
  printf '%s namespaces="git" %s\n' "$TAGGER_EMAIL" "$(<"$PUBLIC_KEY_PATH")" > "$ALLOWED_SIGNERS_PATH"
  git config gpg.ssh.allowedSignersFile "$ALLOWED_SIGNERS_PATH"
}

main() {
  require_environment
  trap cleanup EXIT
  configure_signing_key

  local tag="v$(package_version)"
  if git rev-parse -q --verify "refs/tags/$tag" > /dev/null; then
    echo "tag $tag already exists" >&2
    exit 1
  fi

  git tag --sign --message "Release $tag" "$tag"
  git verify-tag "$tag"
  git push origin "refs/tags/$tag"
}

main "$@"
