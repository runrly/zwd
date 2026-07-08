---
title: "ADR-0001: Use release-plz and cargo-dist for releases"
status: "Accepted"
date: "2026-06-24"
authors: "Project maintainer"
tags: ["architecture", "release", "automation"]
supersedes: ""
superseded_by: ""
---

# ADR-0001: Use release-plz and cargo-dist for releases

## Status

Accepted

## Context

Zed Workspace Dock needs automated version bumps, changelog updates, Release PRs, git tags, GitHub Release artifacts, and generated installers. The project also needs a human approval point before publishing each version.

## Decision

Use release-plz for version bumps, changelog updates, Release PRs, and git tags. Use cargo-dist for GitHub Release artifacts and generated installers. Merging a Release PR is the human approval step before publishing a new version.

## Consequences

### Positive

- **POS-001**: Release PRs keep version and changelog changes reviewable before publication.
- **POS-002**: cargo-dist owns release archives, installers, checksums, attestations, and GitHub Release assets.
- **POS-003**: The release flow can add Homebrew, npm, Scoop, or winget later on top of the same GitHub Release artifacts.

### Negative

- **NEG-001**: Release behavior spans two tools, so CI failures can come from either release-plz or cargo-dist.
- **NEG-002**: Generated cargo-dist workflow files should not be casually hand-edited.
- **NEG-003**: Release-plz must be configured so it does not take over GitHub Release body publication.

## Alternatives Considered

### release-plz only

- **ALT-001**: **Description**: Let release-plz handle versioning, tags, changelog, and GitHub Release publication.
- **ALT-002**: **Rejection Reason**: GitHub Release artifacts and installers are produced by cargo-dist, so release-plz should not own the release body.

### Custom release scripts

- **ALT-003**: **Description**: Replace release-plz and cargo-dist with hand-written scripts.
- **ALT-004**: **Rejection Reason**: Custom scripts would duplicate mature release tooling and increase maintenance cost.

## Implementation Notes

- **IMP-001**: The release workflow creates or updates Release PRs for normal feature and fix commits.
- **IMP-002**: A post-merge release-plz job creates the version tag.
- **IMP-003**: The tag-triggered cargo-dist workflow publishes archives, installers, checksums, attestations, and the GitHub Release.

## References

- **REF-001**: `release-plz.toml`
- **REF-002**: `dist-workspace.toml`
- **REF-003**: `.github/workflows/release-plz.yml`
