# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust CLI for opening Zed workspace files through folder mode or a managed symlink dock. Core code lives in `src/`: `main.rs` wires the binary, `cli.rs` defines command-line parsing, and modules such as `workspace.rs`, `dock.rs`, `zed.rs`, `install.rs`, and `error.rs` hold the main behavior. Integration tests live in `tests/cli.rs`. JSON assets and schemas are under `resources/`, including Zed task templates and workspace/dock marker schemas. Release and project context live in `release-plz.toml`, `dist-workspace.toml`, `docs/adr/`, `CHANGELOG.md`, and `CONTEXT.md`.

## Build, Test, and Development Commands

Use the pinned Rust toolchain from `rust-toolchain.toml`; `devbox shell` prepares the local Cargo/Rustup paths when needed.

- `cargo build`: compile the CLI.
- `cargo run -- <args>`: run the binary locally, for example `cargo run -- list`.
- `cargo fmt --all`: format Rust code.
- `cargo fmt --all -- --check`: verify formatting in CI-style checks.
- `cargo check --all-targets`: type-check all targets.
- `cargo test`: run integration and unit tests.
- `cargo clippy --all-targets --all-features -- -D warnings`: enforce lint cleanliness.

`Makefile.toml` exposes equivalent `cargo make` tasks (`build`, `test`, `fmt-check`, `clippy`), but direct Cargo commands are the clearest validation path.

## Coding Style & Naming Conventions

Follow Rust 2024 idioms and `rustfmt` defaults. Keep modules focused by CLI domain concept, and prefer explicit error variants via the existing error module. The crate forbids unsafe code and denies nonstandard style plus key Clippy lints in `Cargo.toml`; treat warnings as blockers. Use snake_case for Rust items and kebab-case for CLI-facing command names, files, and release config.

## Testing Guidelines

Add integration coverage in `tests/cli.rs` for user-visible CLI behavior, filesystem safety, workspace parsing, and Zed invocation arguments. Tests should use temporary directories and `/bin/echo` or another harmless executable when asserting command invocation. Name tests after the behavior being verified, for example `open_rejects_non_code_workspace_input`.

## Commit & Pull Request Guidelines

Commits use conventional messages with lowercase imperative subjects, for example `feat: implement workspace dock MVP` or `ci: add release automation`. Allowed types include `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, and `test`; keep subjects under 72 characters.

Pull requests should describe the behavior change, note validation commands run, and call out any release, schema, installer, or platform-impacting changes. Include CLI examples or screenshots only when they clarify user-facing behavior.
