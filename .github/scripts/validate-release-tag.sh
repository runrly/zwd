#!/usr/bin/env bash
set -euo pipefail

readonly TAG_PATTERN='^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'

fail() {
  echo "$*" >&2
  exit 1
}

require_environment() {
  : "${GITHUB_REPOSITORY:?missing repository context}"
  : "${GITHUB_OUTPUT:?missing GitHub Actions output file}"
  : "${RELEASE_TAG:?missing release tag}"
}

validate_tag_format() {
  [[ "$RELEASE_TAG" =~ $TAG_PATTERN ]] \
    || fail "tag must be vX.Y.Z, optionally with SemVer prerelease or build metadata"
}

fetch_release_context() {
  git fetch --force --no-tags origin \
    "refs/tags/$RELEASE_TAG:refs/tags/$RELEASE_TAG" \
    "+refs/heads/main:refs/remotes/origin/main"
}

tag_object() {
  git rev-parse "$RELEASE_TAG^{tag}" \
    || fail "tag $RELEASE_TAG must be annotated"
}

tag_commit() {
  git rev-parse "$RELEASE_TAG^{}"
}

verify_github_signature() {
  local object="$1"
  local verified
  verified="$(gh api "repos/$GITHUB_REPOSITORY/git/tags/$object" --jq '.verification.verified')"
  [[ "$verified" == true ]] || fail "tag $RELEASE_TAG is not GitHub-verified"
}

verify_main_ancestry() {
  git merge-base --is-ancestor "$1" origin/main \
    || fail "tag $RELEASE_TAG is not reachable from main"
}

package_version() {
  cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | select(.name == "zwd") | .version'
}

verify_package_version() {
  local version
  version="$(package_version)"
  [[ "$RELEASE_TAG" == "v$version" ]] \
    || fail "tag $RELEASE_TAG does not match Cargo package version $version"
}

write_outputs() {
  local commit="$1"
  echo "tag=$RELEASE_TAG" >> "$GITHUB_OUTPUT"
  echo "commit=$commit" >> "$GITHUB_OUTPUT"
}

main() {
  require_environment
  validate_tag_format
  fetch_release_context

  local object commit
  object="$(tag_object)"
  commit="$(tag_commit)"
  verify_github_signature "$object"
  verify_main_ancestry "$commit"
  git checkout --detach "$commit"
  verify_package_version
  write_outputs "$commit"
}

main "$@"
