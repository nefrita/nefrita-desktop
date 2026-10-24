# Contributing

## Pull requests

- Work on a branch with a type prefix (`feature/`, `fix/`, `chore/`) and open a pull request to `main`. `main` only accepts pull requests, and they are merged with a merge commit.
- To bring a branch up to date with `main`, rebase it locally and push it with `git push --force-with-lease`, or use "Update branch" on the pull request.
- Run the local checks below before opening or updating a pull request.

## Code layout

Each domain of a crate lives in its own folder, with a `mod.rs` that declares its modules and what they export. The unit tests of a module go in their own `tests.rs`: `message.rs` declares `#[cfg(test)] mod tests;`, and its tests live in `message/tests.rs`. A file, code or tests, should stay around 300 lines at most; when one gets close, look for a second responsibility and move it to its own file.

## Local checks

The Rust version is pinned in `rust-toolchain.toml`. Install it once, and again whenever that file changes:

```bash
rustup toolchain install
```

Then run the same checks as the `format`, `lint`, `test` and `dependencies` jobs of the `PR checks` workflow, from the root of the repository:

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

## Integration tests

The integration tests of `nefrita-device` talk to `libjade_daemon`: the Nefrita firmware compiled for a computer, built with XBT, listening on a TCP port. `cargo test` skips them. To run them locally you need Docker; the firmware commit and the builder image are the same as in the `device-integration` job:

```bash
firmware_commit=e18deffd96fb293730a51c9874c73ab4ca6e841a
jade_builder_image=blockstream/jade_builder@sha256:cbf0aabee7513dc8cad8f1d69a5e00f5b73bcd0965a44254627c0de4bfc8c4f2
firmware_directory="$PWD/target/firmware"
git clone https://github.com/Sharmaz/Jade.git "$firmware_directory"
git -C "$firmware_directory" checkout "$firmware_commit"
git -C "$firmware_directory" submodule update --init --recursive
docker run --rm --platform linux/amd64 --volume "$firmware_directory":/host/jade --workdir /host/jade \
  "$jade_builder_image" bash -c '
  git config --global --add safe.directory /host/jade &&
  pushd /opt/esp/idf >/dev/null && . ./export.sh >/dev/null && popd >/dev/null &&
  ./tools/switch_to.sh jade --dev --noradio &&
  ./libjade/make_libjade.sh Debug --xbt'
docker run --rm --detach --name libjade-daemon --platform linux/amd64 --publish 30121:30121 \
  --volume "$firmware_directory/build_linux/libjade":/libjade \
  "$jade_builder_image" /libjade/libjade_daemon --tcp 30121
NEFRITA_LIBJADE_ADDRESS=127.0.0.1:30121 cargo test --package nefrita-device --test libjade --locked \
  -- --ignored --test-threads=1
docker stop libjade-daemon
```

The firmware is cloned inside `target/`, which git ignores and `cargo clean` deletes. Building libjade takes several minutes, and longer on Apple silicon, where Docker emulates `linux/amd64`. While `target/firmware` and its `build_linux/libjade/libjade_daemon` exist, skip the clone and the build, and start from the `docker run --rm --detach` line. The tests wait up to two minutes for libjade to answer. When the workflow changes `FIRMWARE_COMMIT`, update `firmware_commit` here and build again.

## Continuous integration

`PR checks` (`.github/workflows/pr-checks.yml`) runs on every pull request to `main`, and on demand with "Run workflow". It has one job per check above, `format`, `lint`, `test` and `dependencies`, and `device-integration` for the integration tests: it builds `libjade_daemon` with XBT from the firmware commit in `FIRMWARE_COMMIT`, caches it per commit, and runs the integration tests against it. The workflow only uses actions created by GitHub, and its token can only read the repository; `device-integration` logs in to Docker Hub with the `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` repository secrets to pull the builder image.
