---
title: "ADR-0005: Use zwd for repository, package, and state paths"
status: "Accepted"
date: "2026-06-30"
authors: "Project maintainer"
tags: ["architecture", "naming", "state"]
supersedes: ""
superseded_by: ""
---

# ADR-0005: Use zwd for repository, package, and state paths

## Status

Accepted

## Context

Zed Workspace Dock already uses `zwd` as the public command. Keeping repository, Cargo package, release assets, schema namespace, and managed state directories under longer names would make user-visible paths and release URLs diverge from the command.

## Decision

Keep Zed Workspace Dock as the full product name. Use `zwd` for the repository slug, Cargo package, release assets, schema namespace, and managed state directories.

## Consequences

### Positive

- **POS-001**: Command, repository, package, assets, schemas, and state paths share the same short namespace.
- **POS-002**: Release URLs and installer names stay aligned with the command users type.
- **POS-003**: The short namespace is easier to remember in docs and support threads.

### Negative

- **NEG-001**: The short namespace can be less descriptive outside project context.
- **NEG-002**: Older preview names are not migrated by the CLI.
- **NEG-003**: Docs must keep explaining that Zed Workspace Dock is the product and `zwd` is the short public surface.

## Alternatives Considered

### Keep long repository and state names

- **ALT-001**: **Description**: Keep `zed-workspace-dock` for repository, package, release assets, schemas, and state paths.
- **ALT-002**: **Rejection Reason**: Those names would be longer than the command and harder to type in release and install workflows.

### Mixed namespace

- **ALT-003**: **Description**: Use `zwd` for the command but long names for persistent state.
- **ALT-004**: **Rejection Reason**: Mixed naming would create avoidable confusion when troubleshooting local files.

## Implementation Notes

- **IMP-001**: Registered workspaces live under `zwd/workspaces/`.
- **IMP-002**: Dock roots live under `zwd/docks/`.
- **IMP-003**: Existing preview releases and tags predate public use; the CLI does not migrate old `zed-workspace-dock` config or cache directories.

## References

- **REF-001**: ADR-0003
- **REF-002**: `Cargo.toml`
- **REF-003**: `README.md`
