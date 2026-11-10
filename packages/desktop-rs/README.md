# OpenChamber desktop (Rust + GPUI spike)

A GPUI desktop client that runs the OpenChamber server as a sidecar, lists the
sessions of one directory, sends a prompt and renders the streamed reply. It is
a spike: it does not replace `packages/electron`.

## Crates

| Crate | Role |
| --- | --- |
| `oc-sidecar` | Spawns `openchamber serve`, reads the port, stops the server |
| `oc-client` | HTTP and event-stream client for the server API |
| `oc-state` | Pure state reducer, no GPUI |
| `oc-app` | The GPUI binary |

## Prerequisites

- Rust 1.92 or later.
- `bun install` at the repository root, so `packages/web/bin/cli.js` can run.
- Node 24.14 or later.
- `opencode` on `PATH`.
- The Linux packages GPUI Kit needs. On Ubuntu 24.04:

  ```bash
  sudo apt install -y gcc g++ clang libfontconfig-dev libwayland-dev \
    libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
    libssl-dev libzstd-dev libasound2-dev vulkan-validationlayers libvulkan1
  ```

  Other distributions need the equivalent packages, plus a working Vulkan
  driver in a Wayland or X11 session.

## Run

```bash
cd packages/desktop-rs
cargo run -p oc-app -- --directory /path/to/project
```

`--directory` defaults to the current directory. The app starts
`openchamber serve` for that directory before it opens the window, lists its
sessions, and stops the server, and the `opencode serve` process it runs, when
the window closes. The CLI is `OPENCHAMBER_CLI` when set, otherwise
`packages/web/bin/cli.js` found by walking up from the executable and the
current directory.

The server inherits the app's environment, so `HOME`,
`OPENCHAMBER_DATA_DIR` and `OPENCODE_CONFIG_CONTENT` choose its data and its
model provider. To run against the deterministic fixture provider instead of a
real model, start `node scripts/perf/fixture-provider.mjs <port>` from the
repository root and pass the config `fixtureProviderConfig(<port>)` in that
file returns:

```bash
env -i PATH="$PATH" HOME=/tmp/oc-home OPENCHAMBER_DATA_DIR=/tmp/oc-data \
  OPENCODE_CONFIG_CONTENT="$(cat /tmp/oc-fixture-config.json)" \
  WAYLAND_DISPLAY="$WAYLAND_DISPLAY" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
  cargo run -p oc-app -- --directory /tmp/oc-project
```

When the server does not start, the window shows the error with the server's
stderr and log tails and a Retry button. When it exits while the app runs, the
header shows its exit code or signal and offers Retry, which starts a new
server.

The directory has no `package.json`, so Bun ignores it.

### Fixture states

`--state <file>` opens the window on a state fixture instead of a server: the
file's steps (session lists, events, connection changes) run through the same
reducer live data takes, and server-bound controls do nothing.

```bash
cargo run -p oc-app -- --state fixtures/states/streaming.json
```

With `--dump-layout <file>` the app renders that state at 1440x900, writes the
bounds of every region and the controls of each, in order, to `<file>` as JSON,
and exits. The parity test compares these dumps with `fixtures/reference/`.

```bash
cargo run -p oc-app -- --state fixtures/states/empty.json --dump-layout /tmp/empty.json
```

## Tests

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Two suites are off by default:

| Variable | Enables | Needs |
| --- | --- | --- |
| `OC_DESKTOP_IT=1` | Sidecar integration tests: start, shut down, no leftover processes | `bun install` and `opencode` |
| `OC_DESKTOP_UI=1` | Layout parity test against `fixtures/reference/` | A graphical session, because it opens a window |

## Notes

The sidecar runs the CLI in foreground mode, so `openchamber status` lists it
while the app is open.
