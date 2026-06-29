# Use zwd as the public command

Zed Workspace Dock remains the product, repository, and Cargo package name, but `zwd` is the only installed CLI binary and user-facing command. This keeps repeated terminal usage short while preserving the descriptive project name for documentation, release ownership, and managed state directories.

The longer `zed-workspace-dock` binary target is removed before the first public release so there is no compatibility burden for a command name we do not want to support.
