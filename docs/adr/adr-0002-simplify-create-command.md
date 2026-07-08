---
title: "ADR-0002: Simplify create command"
status: "Accepted"
date: "2026-06-26"
authors: "Project maintainer"
tags: ["architecture", "cli", "workspace"]
supersedes: ""
superseded_by: ""
---

# ADR-0002: Simplify create command

## Status

Accepted

## Context

The original create command used a more verbose folder syntax and made output naming less consistent between registered workspaces and explicit output directories. The CLI needed a command shape that matched how developers already think about opening multiple paths in Zed.

## Decision

Change `create` to accept positional folder paths, default to dock mode, and use `--name` for the workspace file stem. `--output` names an output directory instead of an exact `.code-workspace` file, and created workspaces store canonical absolute folder paths.

## Consequences

### Positive

- **POS-001**: `zwd create ./api ./web --name work` matches Zed's own multi-path command shape.
- **POS-002**: Registered workspaces and output-directory workspaces resolve folder paths consistently.
- **POS-003**: The create path no longer needs the older `--folder name=path` syntax.

### Negative

- **NEG-001**: This is a breaking CLI change for users of the removed folder syntax.
- **NEG-002**: Incremental folder addition remains unsupported; users must recreate workspaces with `--force`.
- **NEG-003**: Canonical absolute folder paths make created workspace files less relocatable.

## Alternatives Considered

### Keep `--folder name=path`

- **ALT-001**: **Description**: Continue requiring each folder through a named flag.
- **ALT-002**: **Rejection Reason**: It made common multi-folder workspace creation unnecessarily noisy.

### Exact output file path

- **ALT-003**: **Description**: Keep `--output` as the complete `.code-workspace` destination file.
- **ALT-004**: **Rejection Reason**: It made output-directory workspaces behave differently from registered workspaces and conflicted with using `--name` as the workspace name.

## Implementation Notes

- **IMP-001**: `create <paths>... --name <name>` writes a registered workspace.
- **IMP-002**: `create <paths>... --name <name> --output <dir>` writes a workspace file inside `<dir>`.
- **IMP-003**: Recreating an existing registered workspace requires `--force`.

## References

- **REF-001**: `README.md`
- **REF-002**: `src/cli.rs`
- **REF-003**: `tests/cli.rs`
