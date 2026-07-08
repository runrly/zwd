---
title: "ADR-0006: Use zwd and dock lock metadata names"
status: "Accepted"
date: "2026-07-07"
authors: "Project maintainer"
tags: ["architecture", "naming", "workspace", "dock"]
supersedes: ""
superseded_by: ""
---

# ADR-0006: Use zwd and dock lock metadata names

## Status

Accepted

## Context

Workspace files and dock metadata still used the older `zed-dock` name after the product moved public command, package, repository, schema namespace, and state paths to `zwd`. The dock metadata file also used marker terminology even though it acts as the lock-style ownership record for a managed dock.

## Decision

Use `zwd` as the workspace configuration key and `.zwd-lock.json` as the dock ownership file. Rename marker terminology to dock lock across code, docs, schemas, and errors. Do not support `zed-dock` as a compatibility alias.

## Consequences

### Positive

- **POS-001**: Workspace JSON, schema names, dock metadata, and product namespace all use `zwd`.
- **POS-002**: Dock lock terminology better matches ecosystem lockfile language such as `devbox.lock` and `pnpm-lock`.
- **POS-003**: Rejecting `zed-dock` avoids silently opening legacy workspaces in folder mode.

### Negative

- **NEG-001**: This is a breaking change for existing `.code-workspace` files.
- **NEG-002**: Existing dock cache contents require manual lock file rename before reuse.
- **NEG-003**: The lock file records ownership state but does not implement process-level file locking.

## Alternatives Considered

### Compatibility alias

- **ALT-001**: **Description**: Read both `zed-dock` and `zwd`, while writing only `zwd`.
- **ALT-002**: **Rejection Reason**: Compatibility would keep the old public contract alive and weaken the namespace cleanup.

### `.zwd.json`

- **ALT-003**: **Description**: Rename the dock metadata file to `.zwd.json`.
- **ALT-004**: **Rejection Reason**: The name is short but less precise about the file's role as dock ownership state.

### Keep marker terminology

- **ALT-005**: **Description**: Rename only the file and workspace key while keeping marker language in code and docs.
- **ALT-006**: **Rejection Reason**: Mixed terminology would make safety rules and support docs harder to follow.

## Implementation Notes

- **IMP-001**: New workspace files serialize the config object as `zwd`.
- **IMP-002**: Legacy workspaces using `zed-dock` fail with a migration error instead of falling back to folder mode.
- **IMP-003**: Manual migration is `zed-dock` to `zwd` in workspace files and `.zed-dock.json` to `.zwd-lock.json` in dock directories.

## References

- **REF-001**: ADR-0003
- **REF-002**: ADR-0005
- **REF-003**: `README.md`
