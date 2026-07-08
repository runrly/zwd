---
title: "ADR-0004: Use Windows directory symlinks for dock mode"
status: "Accepted"
date: "2026-06-29"
authors: "Project maintainer"
tags: ["architecture", "windows", "dock"]
supersedes: ""
superseded_by: ""
---

# ADR-0004: Use Windows directory symlinks for dock mode

## Status

Accepted

## Context

Dock mode exists to give Zed one visible root where project entries are links. Windows has several link-like options, including directory symbolic links and junctions.

## Decision

Use real Windows directory symbolic links through the platform symlink API for dock mode. If link creation fails, return an actionable error instead of switching modes.

## Consequences

### Positive

- **POS-001**: `symlink` mode means symlinks across supported platforms.
- **POS-002**: The symlink dock glossary stays literal on Windows.
- **POS-003**: Users get an explicit error when Windows permissions block symlink creation.

### Negative

- **NEG-001**: Windows users may need Developer Mode, `SeCreateSymbolicLinkPrivilege`, or an administrator shell.
- **NEG-002**: Dock mode may fail where folder mode would have worked.
- **NEG-003**: Windows support needs platform-specific validation around symlink behavior.

## Alternatives Considered

### Junctions

- **ALT-001**: **Description**: Create Windows junctions for dock entries.
- **ALT-002**: **Rejection Reason**: Junctions would make `symlink` mode behave differently from its name and documentation.

### Silent folder fallback

- **ALT-003**: **Description**: Fall back to folder mode when Windows symlink creation fails.
- **ALT-004**: **Rejection Reason**: Silent fallback would remove the one-root dock behavior users requested.

## Implementation Notes

- **IMP-001**: Use the Windows directory symlink API for project folder links.
- **IMP-002**: Surface permission failures in the CLI error message.
- **IMP-003**: Keep folder mode available as an explicit user choice.

## References

- **REF-001**: `src/dock.rs`
- **REF-002**: `src/error.rs`
- **REF-003**: `README.md`
