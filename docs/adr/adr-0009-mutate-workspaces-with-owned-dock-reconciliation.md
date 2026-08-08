---
title: "ADR-0009: Mutate workspaces with owned dock reconciliation"
status: "Accepted"
date: "2026-08-08"
authors: "Project maintainer"
tags: ["architecture", "cli", "workspace", "dock", "safety"]
supersedes: ""
superseded_by: ""
---

# ADR-0009: Mutate workspaces with owned dock reconciliation

## Status

Accepted

## Context

`create --force` could replace a workspace's complete folder list, but it was an awkward and disruptive maintenance path for an active symlink dock. The dock lock records ownership and managed links, so incremental workspace mutations must update that live materialization without touching project folders or ambiguous cache contents.

## Decision

Add `add`, `remove`, `delete`, and `status` commands. `add` and `remove` edit canonical workspace membership, preserve unrelated workspace JSON, and reconcile an existing owned dock incrementally. They infer a workspace only from the root of a valid dock; otherwise callers pass `--workspace`. `delete` previews by default and requires `--force` to remove a workspace file and its valid owned dock. `status` is read-only and reports unhealthy folders or docks through a non-zero exit status.

## Consequences

### Positive

- **POS-001**: Active docks receive only the changed symlink operations, reducing disruption for Zed sessions that remain open.
- **POS-002**: The lock remains derived ownership state rather than a second configuration source.
- **POS-003**: Dry-run deletion and strict ownership checks protect workspace files, docks, and project folders.

### Negative

- **NEG-001**: `add` and `remove` require the dock root for implicit targeting because project symlinks can resolve outside the dock.
- **NEG-002**: The CLI does not attempt to discover or close external processes using a dock; `--force` confirms deletion but cannot make that process state safe.
- **NEG-003**: A pre-existing dock with an invalid lock or unmanaged content blocks mutations and deletion until it is remediated manually.

## Alternatives Considered

### Recreate every workspace with `create --force`

- **ALT-001**: **Description**: Keep all membership changes as full workspace replacement.
- **ALT-002**: **Rejection Reason**: It requires repeating the complete folder list and does not synchronize an already-open dock incrementally.

### Rebuild every dock on mutation

- **ALT-003**: **Description**: Delete and recreate all managed symlinks after each edit.
- **ALT-004**: **Rejection Reason**: Unchanged projects would disappear and reappear in an active Zed workspace without need.

### Bypass ownership checks with `delete --force`

- **ALT-005**: **Description**: Let confirmation remove any dock path associated with the workspace.
- **ALT-006**: **Rejection Reason**: Confirmation of workspace deletion must not authorize deletion of unmanaged cache content.

## Implementation Notes

- **IMP-001**: `add` and `remove` use canonical folder targets and are idempotent.
- **IMP-002**: Existing docks are synchronized only when their lock is valid and owned by the edited workspace; absent docks remain absent until `open` materializes them.
- **IMP-003**: `remove` and `delete` remove symlinks and dock metadata only, never symlink targets.

## References

- **REF-001**: ADR-0002
- **REF-002**: ADR-0006
- **REF-003**: `README.md`
- **REF-004**: `CONTEXT.md`
