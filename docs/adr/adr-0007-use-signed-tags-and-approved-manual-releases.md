---
title: "ADR-0007: Use signed tags and approved manual releases"
status: "Accepted"
date: "2026-07-18"
authors: "Project maintainer"
tags: ["architecture", "release", "security", "automation"]
supersedes: "ADR-0001"
superseded_by: ""
---

# ADR-0007: Use signed tags and approved manual releases

## Status

Accepted

## Context

The project needs a reviewable version-and-changelog PR, a trustworthy immutable version tag, and a separate approval before costly artifact builds and GitHub Release publication. The prior tag-triggered cargo-dist flow published automatically after a Release PR merge and created lightweight unsigned tags.

## Decision

Use release-plz for Logto-style Release PRs and changelog entries. After a Release PR merges, the Runrly Echo GitHub App creates an annotated SSH-signed `vX.Y.Z` tag. GitHub Release publication is a manual workflow dispatch for an existing verified tag and requires approval in the `release-publication` environment before cargo-dist starts planning or building.

The signing key is isolated in the `release-signing` environment, restricted to `main`, while the `release-publication` approval environment contains no signing key. cargo-dist release CI is intentionally customized and configured with `allow-dirty = ["ci"]` so cargo-dist accepts the deliberately customized `release.yml`.

## Consequences

### Positive

- **POS-001**: Tags are annotated, SSH-signed, and created only by the release App.
- **POS-002**: A maintainer controls publication timing and approves before builds consume release credentials or runners.
- **POS-003**: Release PRs and `CHANGELOG.md` use concise Major, Minor, and Patch change sections.

### Negative

- **NEG-001**: Updating cargo-dist requires reviewing the custom Release workflow against its generated v0.32 template.
- **NEG-002**: Release publication is no longer immediate after a Release PR merge.

## Alternatives Considered

### Tag-triggered cargo-dist publication

- **ALT-001**: Automatically run cargo-dist when the version tag is pushed.
- **ALT-002**: Rejected because publication would begin without a separate maintainer approval.

### Repository-scoped signing secret

- **ALT-003**: Store the signing key as a repository secret.
- **ALT-004**: Rejected because the key should be isolated to the automatic tag job.

## Implementation Notes

- **IMP-001**: Only the Runrly Echo App bypasses the `v*` tag ruleset.
- **IMP-002**: `release` validates tag existence, annotation, GitHub signature verification, version, and `main` ancestry before cargo-dist runs.
- **IMP-003**: The public signing key is registered on the maintainer GitHub account as an SSH signing key.
- **IMP-004**: Only the Runrly Echo App bypasses the `release-plz-*` branch ruleset.
- **IMP-005**: Before the signing environment is reached, the main-push classifier requires a merged `main` PR from `runrly/zwd`, on a `release-plz-*` branch, authored by `runrly-echo[bot]`.

## References

- **REF-001**: `release-plz.toml`
- **REF-002**: `.github/workflows/release-plz.yml`
- **REF-003**: `.github/workflows/release.yml`
