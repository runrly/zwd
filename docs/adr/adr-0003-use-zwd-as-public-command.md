---
title: "ADR-0003: Use zwd as public command"
status: "Accepted"
date: "2026-06-29"
authors: "Project maintainer"
tags: ["architecture", "cli", "naming"]
supersedes: ""
superseded_by: ""
---

# ADR-0003: Use zwd as public command

## Status

Accepted

## Context

The full product name is Zed Workspace Dock, but repeated terminal usage favors a short command. Keeping both a long binary and a short alias would create a public compatibility burden.

## Decision

Use `zwd` as the only installed CLI binary and user-facing command. Keep Zed Workspace Dock as the full product name.

## Consequences

### Positive

- **POS-001**: The command users type is short and consistent across docs, tests, and install output.
- **POS-002**: Removing the long binary before broad public use avoids a compatibility promise for an unwanted alias.
- **POS-003**: CLI examples stay compact.

### Negative

- **NEG-001**: Users must learn that `zwd` expands to Zed Workspace Dock.
- **NEG-002**: Product prose and command examples use different names.
- **NEG-003**: Any older local references to `zed-workspace-dock` as a command must be updated.

## Alternatives Considered

### Keep long command only

- **ALT-001**: **Description**: Install only `zed-workspace-dock`.
- **ALT-002**: **Rejection Reason**: The command is too long for repeated terminal workflows.

### Keep long command and `zwd` alias

- **ALT-003**: **Description**: Install both binaries.
- **ALT-004**: **Rejection Reason**: Supporting both would create unnecessary compatibility and documentation burden.

## Implementation Notes

- **IMP-001**: The public binary target is `src/bin/zwd.rs`.
- **IMP-002**: Docs should introduce the product as Zed Workspace Dock and use `zwd` for commands.
- **IMP-003**: ADR-0005 supersedes the earlier decision to keep repository, package, and managed state directories named `zed-workspace-dock`.

## References

- **REF-001**: ADR-0005
- **REF-002**: `Cargo.toml`
- **REF-003**: `README.md`
