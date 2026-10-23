# Contributing

## Pull requests

- Work on a branch with a type prefix (`feature/`, `fix/`, `chore/`) and open a pull request to `main`. `main` only accepts pull requests, and they are merged with a merge commit.
- To bring a branch up to date with `main`, rebase it locally and push it with `git push --force-with-lease`, or use "Update branch" on the pull request.
- Run the local checks below before opening or updating a pull request.

## Local checks

The Rust version is pinned in `rust-toolchain.toml`. Install it once, and again whenever that file changes:

```bash
rustup toolchain install
```

Then run the same checks as the `PR checks` workflow, from the root of the repository:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- --deny warnings
cargo test --workspace --all-features --locked
cargo install --locked cargo-deny --version 0.20.2
cargo deny --locked check
```

| Check | What it verifies |
| --- | --- |
| `cargo fmt` | The code follows the rustfmt style |
| `cargo clippy` | No clippy warnings, in any crate or target |
| `cargo test` | All tests pass |
| `cargo deny` | No dependency with a known vulnerability, unmaintained or yanked, only the licenses allowed in `deny.toml`, and only crates from crates.io |

`cargo install` only builds cargo-deny the first time; afterwards it reports that the version is already installed. When the workflow changes the cargo-deny version, update it here too.

## Continuous integration

`PR checks` (`.github/workflows/pr-checks.yml`) runs on every pull request to `main`, and on demand with "Run workflow". It has one job per check above: `format`, `lint`, `test` and `dependencies`. The workflow only uses actions created by GitHub, and its token can only read the repository.
