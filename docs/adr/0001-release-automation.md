# Use release-plz and dist for releases

We use release-plz for version bumps, changelog updates, release PRs, and git tags, and we use dist for GitHub Release artifacts and generated installers. This keeps Rust versioning close to Cargo conventions while treating Zed Workspace Dock as an installable CLI instead of a crate published to crates.io. Homebrew, npm, Scoop, and winget can be added later on top of the same GitHub Release artifacts.
