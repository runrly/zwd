# Simplify the create command

We changed `create` to accept positional folder paths, default to dock mode, and use `--name` for the workspace file stem. `--output` now names an output directory instead of an exact `.code-workspace` file, and created workspaces store canonical absolute folder paths so output-directory workspaces behave like registered workspaces. This is a breaking CLI change, but it makes `create ./api ./web --name work` match Zed's own multi-path command shape and removes the older `--folder name=path` syntax from the creation path.
