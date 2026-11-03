# Contributing

## Pull requests

- Work on a branch with a type prefix (`feature/`, `fix/`, `chore/`) and open a pull request to `main`. `main` only accepts pull requests, and they are merged with a merge commit.
- To bring a branch up to date with `main`, rebase it locally and push it with `git push --force-with-lease`, or use "Update branch" on the pull request.
- Run the local checks below before opening or updating a pull request.

## Code layout

Each domain of a crate lives in its own folder, with a `mod.rs` that declares its modules and what they export. The unit tests of a module go in their own `tests.rs`: `message.rs` declares `#[cfg(test)] mod tests;`, and its tests live in `message/tests.rs`. A file, code or tests, should stay around 300 lines at most; when one gets close, look for a second responsibility and move it to its own file.

The frontend, in `crates/nefrita-app/frontend/src`, is organized by view. Each view has its own folder, named in PascalCase like its components, with an `index.ts` that exports them; its tests go in `__tests__/`, in files ending in `.test.tsx`, and its hooks and translations, when it has them, in `hooks/` and `locales/`. A view with several sections has a subfolder per section, laid out the same way. Inside a view, files import each other directly, and the rest of the app imports the view through its `index.ts`. The typed calls to the commands of the app live in `commands/`, one module per Rust domain, because several views can share them. The root of `src/` keeps what belongs to the whole app (`main.tsx`, `App.tsx` and `styles.css`) and `bindings/`, the TypeScript types that `cargo test` generates from Rust. For example, `DeviceConnection/` holds `BoardPortList.tsx`, its `index.ts` and `__tests__/BoardPortList.test.tsx`, and `commands/boardPorts.ts` holds the call that searches the board ports.

### Adding a command

A command is a Rust function that the frontend calls through the Tauri IPC. `search_board_ports`, which lists the connected boards, shows each step; the paths are inside `crates/nefrita-app/`:

1. Write it in the module of its domain, with `#[tauri::command]`, as in `src/board_ports/mod.rs`, and add it to `tauri::generate_handler!` in `src/main.rs`.
2. Add its name to `AppManifest::commands` in `build.rs`. From that list, `tauri-build` generates two permissions per command, `allow-<command>` and `deny-<command>`, with the underscores turned into dashes; a command missing from the list has no permission to give.
3. Allow it in `capabilities/main-window.json`: `allow-search-board-ports` lets the main window call `search_board_ports`, and Tauri rejects the calls of any command without its permission.
4. Derive `ts_rs::TS` on the types it takes and returns, only for the tests: `#[cfg_attr(test, derive(ts_rs::TS), ts(export))]`. `cargo test` writes them into `frontend/src/bindings/`, following their serde attributes. Commit them with the command: the `test` job fails if, after its tests, they differ from the ones in the repository.
5. Write its typed call in `frontend/src/commands/`, in the module of its domain: a function that calls `invoke` from `@tauri-apps/api/core` with the name of the command and returns the types of `frontend/src/bindings/`, as `searchBoardPorts` does in `frontend/src/commands/boardPorts.ts`.
6. Test it in Rust, in the `tests.rs` of its domain, and in the frontend, in the `__tests__/` of the view that uses it. There, `mockIPC` from `@tauri-apps/api/mocks` answers in place of the command, and `clearMocks()` removes the mock after each test.

## Local checks

The app crate, `nefrita-app`, is built on Tauri, which uses the web view of each system, so clippy, the tests and the release build need what this table lists for each system to compile it:

| System | What to install |
| --- | --- |
| Debian and Ubuntu | `sudo apt-get update`, then `sudo apt-get install --no-install-recommends build-essential pkg-config libwebkit2gtk-4.1-dev` |
| macOS | The Xcode Command Line Tools: `xcode-select --install` |
| Windows | The Microsoft C++ Build Tools, with the "Desktop development with C++" workload, and WebView2, which comes with Windows 10 from version 1803 and with later versions. On x86_64, aws-lc-sys, the cryptography library under the TLS of the device crate, also needs NASM or its prebuilt objects: set `AWS_LC_SYS_PREBUILT_NASM=1` in the shell that runs cargo, as the CI does (in PowerShell, `$env:AWS_LC_SYS_PREBUILT_NASM = "1"`), or install NASM |

The Linux commands install the same package as the CI, plus the C compiler and `pkg-config` that the GitHub runners already have; they were tested on Ubuntu 24.04 with the Docker block below. On other distributions, the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) give the name of the WebKitGTK development package; their list also has packages that the build does not use.

The Rust version is pinned in `rust-toolchain.toml`. Install it once, and again whenever that file changes:

```bash
rustup toolchain install
```

The frontend, in `crates/nefrita-app/frontend`, needs Node and pnpm. Its `package.json` pins Node by its major version, 26, in `devEngines`, and pnpm by its exact version, in `packageManager`; pnpm stops with an error when the Node version does not match. Install Node 26, from [nodejs.org](https://nodejs.org/) or with a version manager, and then pnpm with npm:

```bash
npm install --global pnpm@11.28.9
```

When `package.json` changes either version, update it here too.

Then run the same checks as the `PR checks` workflow. First those of the `frontend` job, from `crates/nefrita-app/frontend`:

```bash
pnpm install --frozen-lockfile
pnpm check-format-and-lint
pnpm typecheck
pnpm test
pnpm build
pnpm audit --prod
```

The release build puts the `frontend/dist` that `pnpm build` wrote inside the app, and fails without it, so these go after the frontend checks: those of the `format`, `lint`, `test` and `app-build` jobs, from the root of the repository:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- --deny warnings
cargo clippy --workspace --all-targets --locked -- --deny warnings
cargo test --workspace --all-features --locked
cargo build --release --locked --package nefrita-app --features tauri/custom-protocol
```

Then those of the `dependencies` job, from the root of the repository, the `nefrita-desktop` folder. `cargo deny` only looks for `Cargo.toml` in the folder where it runs, so from `crates/nefrita-app/frontend`, where the frontend checks leave you, go back to the root first with `cd ../../..`:

```bash
cargo install --locked cargo-deny --version 0.20.2
cargo deny --locked check
```

| Check | What it verifies |
| --- | --- |
| `pnpm install` | The lockfile matches `package.json`, and pnpm installs exactly the versions it lists |
| `pnpm check-format-and-lint` | The frontend follows the formatting and lint rules of `biome.json` |
| `pnpm typecheck` | TypeScript finds no type errors in `src/` or in `vite.config.ts` |
| `pnpm test` | The frontend tests pass, in jsdom, with `mockIPC` answering in place of the commands |
| `pnpm build` | Vite builds the frontend into `frontend/dist` |
| `pnpm audit` | No high or critical advisory in the dependencies that ship with the app |
| `cargo fmt` | The code follows the rustfmt style |
| `cargo clippy` | No clippy warnings, in any crate or target, with all features and with the default ones |
| `cargo test` | All tests pass, and the TypeScript types they write into `frontend/src/bindings` are the ones in the repository: if git shows them changed, commit them with your Rust change |
| `cargo build` | The app compiles in release mode, with the frontend inside the binary, on your system; the CI builds it on Linux, macOS and Windows |
| `cargo deny` | No dependency with a known vulnerability, unmaintained or yanked, only the licenses allowed in `deny.toml`, and only crates from crates.io |

`pnpm audit --prod` only fails on high or critical advisories (`audit.level` in `pnpm-workspace.yaml`), and only in the dependencies that ship with the app, not in the development tools. When an advisory has a fixed version, `pnpm audit --fix` adds an override to it, and when that version is younger than the 3-day wait of `minimumReleaseAge`, it exempts that version alone in `minimumReleaseAgeExclude`; then `pnpm install` installs it without waiting. When an advisory has no fix yet, `pnpm audit --ignore` with its GHSA ID ignores it. These exceptions stay in `pnpm-workspace.yaml`: review them in the pull request, and remove them once they are no longer needed.

`cargo install` only builds cargo-deny the first time; afterwards it reports that the version is already installed. When the workflow changes the cargo-deny version, update it here too.

`cargo deny` only checks the platforms listed under `[graph] targets` in `deny.toml`: the six desktop targets the app is released for (Linux, macOS and Windows, on x86_64 and ARM64). Update that list together with the release builds.

### Running the Linux jobs with Docker

On a system other than Linux, Docker can run the Linux jobs, `lint`, `test` and the Linux `app-build`, in a clean Ubuntu 24.04 with the same package as the CI. Run the frontend checks first: the container copies the `frontend/dist` that `pnpm build` wrote, which is the same on every system, so it does not need Node. From the root of the repository:

```bash
docker run --rm --env DEBIAN_FRONTEND=noninteractive --volume "$PWD":/host/nefrita-desktop:ro \
  ubuntu:24.04 bash -c '
  set -e
  apt-get update
  apt-get install --yes --no-install-recommends ca-certificates curl build-essential pkg-config
  apt-get install --yes --no-install-recommends libwebkit2gtk-4.1-dev
  mkdir /nefrita-desktop
  tar --exclude=./target --exclude=./.git --exclude=./crates/nefrita-app/gen \
    --exclude=./crates/nefrita-app/frontend/node_modules \
    -C /host/nefrita-desktop -cf - . | tar -C /nefrita-desktop -xf -
  cd /nefrita-desktop
  curl --proto "=https" --tlsv1.2 --silent --show-error --fail https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain none
  . "$HOME/.cargo/env"
  rustup toolchain install
  cargo clippy --workspace --all-targets --all-features --locked -- --deny warnings
  cargo clippy --workspace --all-targets --locked -- --deny warnings
  cargo test --workspace --all-features --locked
  cargo build --release --locked --package nefrita-app --features tauri/custom-protocol
  echo "all Linux jobs passed"
'
```

The container first installs what the GitHub runners already have (certificates, `curl`, a C compiler and `pkg-config`), then `libwebkit2gtk-4.1-dev`. It works on a copy of the repository without `target/` or the `node_modules` of the frontend, so it does not touch yours. Then it installs the pinned Rust version and runs the same commands as the jobs, except the `test` job's check of the TypeScript types, which needs git: the `cargo test` of your system covers it. It stops at the first command that fails, and if a system library is missing, the build error names it. It compiles Tauri three times, so it takes a while.

The block is for a bash shell and has only been tried on macOS. On Apple silicon, Docker runs Ubuntu for ARM64, while the CI uses x86_64; the package names are the same.

## Running the app

The Tauri CLI runs the app in development; `pnpm install` installs it with the frontend. From `crates/nefrita-app/frontend`:

```bash
pnpm tauri dev
```

This starts the Vite development server, compiles the app in debug mode and opens its window. A change in the frontend reaches the window right away, through Vite, and a change in Rust makes the CLI rebuild and restart the app; `crates/nefrita-app/.taurignore` keeps the frontend out of what the CLI watches. The window lists the boards connected by USB, with their port, chip, product and serial number, and a button searches again. The app has one command of its own, `search_board_ports`, and `crates/nefrita-app/capabilities/main-window.json` gives the main window that command alone, with no permission of the Tauri core. In debug builds, right-clicking the page opens a menu with the web inspector.

In development the page comes from the Vite development server, with the content security policy of `vite.config.ts`: Tauri only applies the one in `tauri.conf.json` to the pages it serves itself. A plain `cargo run --package nefrita-app` also loads the page from `http://localhost:5173`, so it only shows it while the Vite development server is running.

To open the app with the frontend inside the binary and the release content security policy, but still with the web inspector, run `pnpm build` and then, from the root of the repository:

```bash
cargo run --package nefrita-app --features tauri/custom-protocol
```

To open it as the `app-build` job compiles it, in release mode:

```bash
cargo run --release --package nefrita-app --features tauri/custom-protocol
```

`tauri/custom-protocol` is the feature that the Tauri CLI adds to its release builds, and cargo needs it explicitly. Both commands use the `frontend/dist` of the last `pnpm build`, so run it again after changing the frontend.

## Integration tests

The integration tests of `nefrita-device` talk to `libjade_daemon`, the Nefrita firmware compiled for a computer, built with XBT and listening on a TCP port. They also talk to two PIN servers: the development server, with the test key pair of the firmware repository, at `http://localhost:8096`, the address compiled into XBT development builds; and a custom server, with its own key pair, at `http://localhost:8097`.

The tests use the debug methods of the device, so they need the `debug-methods` feature: plain `cargo test` does not build them, and `cargo test --all-features` lists them as ignored. To run them locally you need Docker; the firmware commit and the builder image are the same as in the `device-integration` job.

Set these variables from the root of the repository, in the same shell as the next two blocks, and again in every new shell:

```bash
firmware_commit=e18deffd96fb293730a51c9874c73ab4ca6e841a
jade_builder_digest=cbf0aabee7513dc8cad8f1d69a5e00f5b73bcd0965a44254627c0de4bfc8c4f2
jade_builder_image="blockstream/jade_builder@sha256:$jade_builder_digest"
firmware_directory="$PWD/target/firmware"
custom_pin_server_key_directory="$PWD/target/custom-pin-server"
custom_pin_server_public_key_file="$custom_pin_server_key_directory/server_public_key.pub"
```

Prepare the firmware, `libjade_daemon`, the PIN server image and the key pair of the custom PIN server:

```bash
[ -d "$firmware_directory" ] || git clone https://github.com/Sharmaz/Jade.git "$firmware_directory"
git -C "$firmware_directory" fetch origin
git -C "$firmware_directory" checkout "$firmware_commit"
git -C "$firmware_directory" submodule update --init --recursive
docker run --rm --platform linux/amd64 --volume "$firmware_directory":/host/jade \
  --workdir /host/jade "$jade_builder_image" bash -c '
  git config --global --add safe.directory /host/jade &&
  pushd /opt/esp/idf >/dev/null && . ./export.sh >/dev/null && popd >/dev/null &&
  ./tools/switch_to.sh jade --dev --noradio &&
  ./libjade/make_libjade.sh Debug --xbt'
docker build --platform linux/amd64 --tag pin-server "$firmware_directory/pinserver"
mkdir -p "$custom_pin_server_key_directory"
docker run --rm --platform linux/amd64 --volume "$custom_pin_server_key_directory":/keys \
  --workdir /keys --env PYTHONPATH=/ pin-server python3 -m pinserver.generateserverkey
```

Then start the two PIN servers and `libjade_daemon`, run the tests and stop them:

```bash
docker run --rm --detach --name development-pin-server --platform linux/amd64 \
  --publish 8096:8096 --tmpfs /pins \
  --volume "$firmware_directory/server_private_key.key":/server_private_key.key:ro \
  pin-server
docker run --rm --detach --name custom-pin-server --platform linux/amd64 \
  --publish 8097:8096 --tmpfs /pins \
  --volume "$custom_pin_server_key_directory/server_private_key.key":/server_private_key.key:ro \
  pin-server
docker run --rm --detach --name libjade-daemon --platform linux/amd64 --publish 30121:30121 \
  --volume "$firmware_directory/build_linux/libjade":/libjade \
  "$jade_builder_image" /libjade/libjade_daemon --tcp 30121
for pin_server_port in 8096 8097; do
  curl --silent --fail --output /dev/null --retry 60 --retry-delay 1 --retry-all-errors \
    "http://localhost:$pin_server_port/"
done
NEFRITA_LIBJADE_ADDRESS=127.0.0.1:30121 \
NEFRITA_CUSTOM_PIN_SERVER_URL=http://localhost:8097 \
NEFRITA_CUSTOM_PIN_SERVER_PUBLIC_KEY_FILE="$custom_pin_server_public_key_file" \
  cargo test --package nefrita-device --test libjade --features debug-methods --locked \
  -- --ignored --test-threads=1
docker stop libjade-daemon development-pin-server custom-pin-server
```

The firmware and the key pair of the custom PIN server stay inside `target/`, which git ignores and `cargo clean` deletes. Run the preparation once, again after `cargo clean`, and again when the workflow changes `FIRMWARE_COMMIT`, after updating `firmware_commit` here. Building libjade takes several minutes, and longer on Apple silicon, where Docker emulates `linux/amd64`; while `target/firmware/build_linux/libjade/libjade_daemon` is built for the current commit, the `docker run` that builds it can be left out. `generateserverkey` keeps a key pair that already exists.

The PIN servers keep the PINs in memory (`--tmpfs /pins`), so every run starts empty. If the `local-pinserver` of the firmware guide is already running on port 8096, it works as the development PIN server: skip the `docker run` of `development-pin-server` and leave it out of `docker stop`. The tests wait up to two minutes for `libjade_daemon` to answer.

If a test fails, `docker logs` with the name of a container (`libjade-daemon`, `development-pin-server` or `custom-pin-server`) shows its output. Run it before `docker stop`, which removes the containers.

## Manual tests with a board

The example program `crates/nefrita-device/examples/unlock_over_usb.rs` talks to a real board over USB, the way the app will: it finds the board, checks that its firmware is XBT, prints its version and state, and unlocks it through the PIN server. Until the app connects to the board, it is the manual test of the USB connection.

### What you need

- A board from the [DIY list of Jade](https://github.com/Sharmaz/Jade/blob/master/diy/README.md) with the XBT development firmware, connected with a USB cable that carries data. Build the firmware with the "XBT firmware builds" block of the [firmware guide](https://github.com/Sharmaz/Jade/blob/master/CONTRIBUTING.md), with the profile and the chip of your board as the only entry of its loop, for example `display_ttgo_tdisplay:esp32`; the profiles are the `configs/sdkconfig_*.defaults` files of the firmware. The firmware CI only builds XBT for the T-Display and the M5StickC Plus2, so the other boards of the list have not been tried with XBT yet. Then flash it from the `build/` directory of the firmware repository with `esptool` 4.12.0, the version in the builder image. Install it in a Python virtual environment, because the Python of Homebrew does not let pip install packages outside one; the Tor test below uses the same environment:

  ```bash
  python3 -m venv ~/.venvs/nefrita-board-tools
  source ~/.venvs/nefrita-board-tools/bin/activate
  pip install esptool==4.12.0
  ```

  Set `board_port` to the port of your board: the example program lists it even before the board has the XBT firmware (see "Running the example"). Use `esp32s3` instead of `esp32` for an ESP32-S3 board. The first command erases the board, including any wallet stored on it, so that it starts without the data of other firmware:

  ```bash
  board_port=/dev/cu.usbserial-0001
  esptool.py --chip esp32 --port "$board_port" --baud 460800 erase_flash
  esptool.py --chip esp32 --port "$board_port" --baud 460800 \
    --before default_reset --after hard_reset write_flash "@flash_args"
  ```

- The development PIN server, keeping its data between runs. XBT development builds send their PIN requests to `http://localhost:8096`, and a board only unlocks with the server that holds the data of its PIN. Run the `local-pinserver` of the firmware guide ("Local PIN server"), which keeps that data in the `local-pinserver-pins` volume. It uses port 8096, like the development server of the integration tests: stop one before starting the other.

### Running the example

From the root of the repository:

```bash
cargo run --package nefrita-device --example unlock_over_usb
```

The program lists, without opening any, the USB serial ports whose chip is on the DIY list of Jade: CP210x, CH9102, FT232R, CH340 and the native USB of the ESP32-S3. For each one it shows the port, the chip with its USB ID, the product and the serial number. With one board it uses it, and with several it asks which one; the order can change between runs, so tell the boards apart by their serial number or port. Then it connects, stops if the firmware is not XBT, prints the firmware version, the board type and the state, and unlocks on `xbt-testnet4`:

| State | What happens |
| --- | --- |
| `Uninitialized` | The board has no wallet: it guides you through creating one and choosing its PIN, and the server receives `/set_pin` |
| `Locked` | Enter the PIN on the board; the server receives `/get_pin` |
| `Ready` | The board is already unlocked, and the program ends without asking for the PIN. A second run right after a successful one shows `Ready`: opening and closing the port does not restart the board. To lock it again, choose **Session** and then **Log Out** on the board; unplugging it is not enough on a board with a battery, such as the M5StickC Plus2 |

| Option | Use |
| --- | --- |
| `--port <port name>` | Connects to that port without searching: for a board with a chip that is not on the list, or to skip the question |
| `--network <network>` | The network of the unlock: `xbt`, `xbt-testnet4` (the default) or `xbt-regtest` |
| `--tor-proxy <IP address:port>` | Sends the PIN server requests through the Tor SOCKS proxy at that address, for example `127.0.0.1:9050`, and only to the onion URL of the board: see "Unlocking through Tor" |

Options go after `--`, for example `cargo run --package nefrita-device --example unlock_over_usb -- --port /dev/cu.usbserial-0001`.

The key pair of the development PIN server is public: a wallet created with it is only for testing, and must never hold real funds.

### Unlocking through Tor

With `--tor-proxy`, the example program sends the PIN server requests through the SOCKS proxy of a Tor that is already running. Then it only uses the onion URL that the board offers: if the board has none, or Tor does not answer, the unlock fails instead of using the normal URL. XBT development builds have no onion URL, so this test gives the board one, from an onion service of your own that forwards to the `local-pinserver`.

Besides the board and the `local-pinserver` of "What you need", the test needs:

- Tor. On macOS, install it with Homebrew: `brew install tor`. On Linux, follow the [installation guide of the Tor Project](https://support.torproject.org/little-t-tor/getting-started/installing/); this test has not been tried on Linux yet. Port 9050 must be free: if another Tor already uses it, stop it first, for example with `brew services stop tor`.
- jadepy, the Python client of the firmware repository, to give the board the onion URL: run `pip install .` from the root of that repository, in the virtual environment of "What you need".

Start Tor in a terminal of its own and leave it running. It keeps its configuration, its working data and the onion service in `~/.nefrita-tor-test`, and it is ready when it prints `Bootstrapped 100% (done): Done`:

```bash
tor_test_directory="$HOME/.nefrita-tor-test"
mkdir -p "$tor_test_directory"
chmod 700 "$tor_test_directory"
printf '%s\n' "SocksPort 9050" "DataDirectory $tor_test_directory/working-data" \
  "HiddenServiceDir $tor_test_directory/onion-service" \
  "HiddenServicePort 80 127.0.0.1:8096" > "$tor_test_directory/configuration"
tor -f "$tor_test_directory/configuration"
```

The first run creates the onion, and later runs keep the same one. Its private key stays in `onion-service/`, inside that directory: never share it or add it to a repository.

In another terminal, set `board_port` again as in "What you need", because a new terminal does not have it. Then, from the root of the firmware repository, activate the virtual environment and give the board the onion as its second PIN server URL:

```bash
source ~/.venvs/nefrita-board-tools/bin/activate
onion_host_name=$(cat "$HOME/.nefrita-tor-test/onion-service/hostname")
python set_jade_pinserver.py --serialport "$board_port" --set-url http://localhost:8096 \
  --set-url2 "http://$onion_host_name" --set-pubkey xbt_pinserver_public_key.pub
```

The board shows both URLs and the public key of the development PIN server; the onion URL may not fit on the screen. Confirm them within two minutes, the time the script waits for the board. The public key is the one the board already uses, so the firmware accepts the change and keeps the wallet. The warning of the script about Bluetooth does not matter over USB.

Then, from the root of this repository, unlock through Tor:

```bash
cargo run --package nefrita-device --example unlock_over_usb -- --port "$board_port" \
  --tor-proxy 127.0.0.1:9050
docker logs --tail 3 local-pinserver
```

The program prints `Sending the PIN server requests through the Tor SOCKS proxy at 127.0.0.1:9050` before the unlock, and `docker logs` shows the request. A new onion can take a while to be reachable: if the board asks `Failed communicating with Oracle - retry ?`, choose to retry. The server logs every request, but Tor and a direct connection reach it from the same computer, so its log does not tell them apart.

To check that the unlock never falls back to the normal URL, stop Tor with Ctrl+C in its terminal, lock the board again (see the `Ready` state above), and run the same two commands. When the board asks to retry, choose not to: the program ends with `the PIN server exchange failed: the Tor SOCKS proxy does not answer at 127.0.0.1:9050`, and `docker logs` shows no new request. Without `--tor-proxy`, the board still unlocks through `http://localhost:8096`: the program only uses the onion through Tor.

Tor keeps the same onion while `~/.nefrita-tor-test` exists, so `tor -f ~/.nefrita-tor-test/configuration` brings it back for later tests. When you no longer need it, stop Tor, reset the PIN server details of the board to those of its firmware, confirming on the board, and delete the directory. Otherwise the board keeps offering an onion that no longer exists, and an unlock through Tor fails without saying why. Run these commands from the root of the firmware repository, with `board_port` set as in "What you need":

```bash
source ~/.venvs/nefrita-board-tools/bin/activate
python set_jade_pinserver.py --serialport "$board_port" --reset-details
rm -rf "$HOME/.nefrita-tor-test"
```

### Ports on each system

| System | Ports | Notes |
| --- | --- | --- |
| macOS | `/dev/cu.*` | Each board also appears as `/dev/tty.*`, which waits for a carrier signal; the program leaves it out, and `--port` takes the `/dev/cu.*` name |
| Linux | `/dev/ttyUSB*` or `/dev/ttyACM*` | Opening them needs the `dialout` group (`uucp` on Arch): `sudo usermod --append --groups dialout "$USER"`, then log out and back in |
| Windows | `COM<number>` | Not tested yet |

### When it does not work

- **No board found while the board is on:** the cable or the hub port may only charge. On macOS, `ioreg -p IOUSB -w 0` shows whether the system sees the board at all.
- **The connection fails:** only one program can have the port open, so close serial monitors such as `idf.py monitor` first.
- **`the device firmware is not XBT`:** the board runs other firmware; flash the XBT development firmware.
- **The unlock fails at the PIN server:** check that the server is running and that it uses the volume where the PIN of that board was created.
- **`the device offers no onion PIN server URL to use through Tor`:** the board has no onion URL yet; give it one as in "Unlocking through Tor".
- **`the Tor SOCKS proxy does not answer at …`:** Tor is not running at that address; start it, or pass the address of its `SocksPort` to `--tor-proxy`.

## Continuous integration

`PR checks` (`.github/workflows/pr-checks.yml`) runs on every pull request to `main`, and on demand with "Run workflow". It has one job for the frontend checks above, tests included, `frontend`; one job per Rust check, `format`, `lint`, `test`, `app-build` and `dependencies`; and `device-integration` for the integration tests. `frontend` and `app-build` install Node in the version of `package.json` with `actions/setup-node`, and pnpm with npm, in the version of `packageManager`. On Linux, `lint`, `test` and `app-build` first install `libwebkit2gtk-4.1-dev`, without its recommended packages. After its tests, `test` fails if they changed the TypeScript types in `frontend/src/bindings`, that is, if the types in the repository are not the ones Rust generates. `app-build` builds the frontend and compiles the app in release mode on Linux x86_64, macOS ARM64 and Windows x86_64, one job per system, and a failure on one system does not cancel the others; it sets `AWS_LC_SYS_PREBUILT_NASM`, so that on Windows aws-lc-sys uses its prebuilt NASM objects. `device-integration` builds `libjade_daemon` with XBT and the PIN server image from the firmware commit in `FIRMWARE_COMMIT`, caches both per commit, generates a new key pair for the custom PIN server on every run, and runs the integration tests against `libjade_daemon` and the two PIN servers. If it fails, the job shows the output of `libjade_daemon` and of each PIN server.

The workflow only uses actions created by GitHub, and its token can only read the repository; `device-integration` logs in to Docker Hub with the `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` repository secrets to pull the builder image and the Debian base image of the PIN server.
