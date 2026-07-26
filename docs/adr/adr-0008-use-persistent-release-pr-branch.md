---
title: "ADR-0008: Use a persistent Release PR branch"
status: "Accepted"
date: "2026-07-26"
authors: "Project maintainer"
tags: ["architecture", "release", "automation", "security"]
supersedes: ""
superseded_by: ""
---

# ADR-0008: Use a persistent Release PR branch

## Status

Accepted

## Context

Release-plz creates a timestamped branch whenever it opens a new Release PR. That leaves stale protected refs after each release and makes the release authorization boundary broader than necessary. The project needs one discoverable Release PR branch while preserving release-plz for versioning and changelog generation.

## Decision

Use `release-plz/main` as the persistent protected Release PR branch. Configure release-plz with the `release-plz/` prefix so it can update an open canonical PR. When it creates a timestamped staging branch, the workflow validates the release-plz output, moves the canonical ref to the staging commit, opens a replacement canonical PR with the staging PR metadata, then closes the staging PR and removes its ref.

Only the Runrly Echo App can create, update, or delete the canonical branch and PR. The tag classifier accepts only a merged Release PR whose exact head ref is `release-plz/main`. If replacement fails, the workflow closes the replacement PR, reopens the staging PR when needed, and restores the prior canonical ref.

## Consequences

### Positive

- **POS-001**: Maintainers always find the pending Release PR on one stable branch.
- **POS-002**: Timestamped branches do not accumulate after successful runs.
- **POS-003**: The signing path authorizes one exact branch instead of a prefix.

### Negative

- **NEG-001**: Promotion needs a small GitHub API helper because release-plz does not expose a fixed branch-name setting.
- **NEG-002**: A staging ref and a replacement PR exist briefly during reconciliation and are retained only when rollback cannot complete.

## Alternatives Considered

### Keep timestamped release-plz branches

- **ALT-001**: Retain the default release-plz branch lifecycle.
- **ALT-002**: Rejected because release refs accumulate and the signing classifier remains prefix-based.

### Replace release-plz with Changesets

- **ALT-003**: Adopt the fixed-branch mechanism used by the reference repository.
- **ALT-004**: Rejected because the existing release-plz versioning and changelog contract remains appropriate for this Rust crate.

## Implementation Notes

- **IMP-001**: The remote ruleset targets exactly `refs/heads/release-plz/main` and bypasses only the Runrly Echo App.
- **IMP-002**: The legacy `release-plz-2026-07-18T23-10-58Z` ref is removed after the ruleset migration; its merged PR history remains intact.
- **IMP-003**: The first `ci(release)` merge after this change exercises the production promotion path.

## References

- **REF-001**: `release-plz.toml`
- **REF-002**: `.github/workflows/release-plz.yml`
- **REF-003**: `.github/scripts/promote-release-pr-branch.sh`
