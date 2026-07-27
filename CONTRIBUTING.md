# Contributing

Thanks for contributing to Zed Workspace Dock.

## Local checks

Use the pinned Rust toolchain directly, or run commands through Devbox when the host environment does not provide it.

```sh
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

The equivalent Devbox form is `devbox run -- <command>`, for example `devbox run -- cargo test --locked`.

## Pull requests

Keep pull requests focused and use Conventional Commit messages with lowercase imperative subjects, such as `fix: reject unmanaged dock contents`. Explain the behavior change, validation performed, and any CLI, workspace-format, dock-safety, release, schema, installer, persistence, or platform impact.

Preserve the established safety contract: never mutate an unmanaged dock, never copy or delete project folders through a dock, and require `--force` before overwriting a registered workspace. Add or update integration coverage in `tests/cli.rs` when changing CLI behavior.

For user-facing documentation, keep `README.md` aligned with the public `zwd` command and its current create, open, install, and list behavior. Do not introduce an incremental `add` workflow unless the CLI gains that command.

## Releases and security

Maintainers manage Release PRs, signed version tags, and GitHub Releases. Contributors must not create release tags or release artifacts manually.

Do not open public issues or pull requests for suspected vulnerabilities. Follow the private reporting process in [SECURITY.md](SECURITY.md) instead.

All contributors are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
