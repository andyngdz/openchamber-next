# OpenChamber desktop (Rust + GPUI spike)

A GPUI desktop client that runs the OpenChamber server as a sidecar, lists the
projects from the shared settings with their sessions, sends a prompt and renders
the streamed reply. It is a spike: it does not replace `packages/electron`.

## Crates

| Crate | Role |
| --- | --- |
| `oc-sidecar` | Spawns `openchamber serve`, reads the port, stops the server |
| `oc-client` | HTTP and event-stream client for the server API |
| `oc-state` | Pure state reducer, no GPUI |
| `oc-app` | The GPUI binary |

`oc-app` also uses `uuid` 1 (feature `v4`) to mint a Chat session's folder and
`chrono` 0.4 (feature `clock`) for its local date. Both are already in
`Cargo.lock` through GPUI, so the workspace gains no second version of either.
It uses `image` 0.25 (feature `png`) to write `--screenshot`, which GPUI already
depends on, and GPUI Kit's `test-support` feature for the headless renderer.

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
  driver. The parity test and `--screenshot` render through GPUI's headless
  renderer, so they need no Wayland or X11 session.

## Run

```bash
cd packages/desktop-rs
cargo run -p oc-app
```

There is no `--directory`: the app loads the shared settings, so the sidebar
comes from the same projects the Electron app shows, and the window opens on a
new Chat draft. The sidecar works in the user's home directory, and a new Chat
session gets a folder under the server's chats root
(`<chatsRoot>/YYYY-MM-DD/session-<uuid v4>`). The app starts `openchamber serve`
before it opens the window, and stops the server, and the `opencode serve`
process it runs, when the window closes. The CLI is `OPENCHAMBER_CLI` when set,
otherwise `packages/web/bin/cli.js` found by walking up from the executable and
the current directory.

### Shared settings

Next reads and writes the same `GET`/`PUT /api/config/settings?surface=desktop`
document as the React UI, so the active project, recents, favorites, the sidebar
settings and each project's collapse state are shared with the Electron app.
Writes are partial: each `PUT` carries only the keys the change touched, the
server merges them, and the answer replaces the settings the window holds. A
write is debounced 200 ms (700 ms for a collapse) and a failed write keeps the
choice locally and is sent again with the next one; closing the window flushes
what is pending before the sidecar stops. Next reads the settings at startup and
after an event-stream reconnect, not while the Electron app writes in between.

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
  cargo run -p oc-app
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

With `--dump-layout <file>` the app renders that state at 1440x900 in a headless
GPUI window, writes the bounds of every region and the controls of each, in
order, to `<file>` as JSON, and exits. The parity test compares these dumps with
`fixtures/reference/`. No display server is involved.

With `--screenshot <file>` the app renders the same headless window, using the
real text system, fonts and theme, and writes the frame as a PNG. No display
server or Vulkan window is involved: GPUI's headless renderer rasterizes the
frame. The test window renders at its own 2x scale, so the PNG is 2880x1800 for
the same 1440x900 content, and the dump records that scale factor.

```bash
cargo run -p oc-app -- --state fixtures/states/empty.json --screenshot /tmp/empty.png
```

`tools/screenshots.sh` renders every `fixtures/states/*.json`, popup states with
their popup open, to `fixtures/screenshots/<state>.gpui.png`, and pairs each with
its Electron reference at `fixtures/reference/<state>.png` in
`fixtures/screenshots/<state>.side-by-side.png` (React on the left, GPUI on the
right, both at the reference's 1440x900). It needs ImageMagick 7 (`magick`).

```bash
tools/screenshots.sh            # every state
tools/screenshots.sh empty.json # named states only
```

```bash
cargo run -p oc-app -- --state fixtures/states/empty.json --dump-layout /tmp/empty.json
```

## Tests

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The sidecar suite is off by default:

| Variable | Enables | Needs |
| --- | --- | --- |
| `OC_DESKTOP_IT=1` | Sidecar integration tests: start, shut down, no leftover processes | `bun install` and `opencode` |

The layout parity test is part of `cargo test --workspace`: it renders each
state in a headless window, so it needs no graphical session.

## Notes

The sidecar runs the CLI in foreground mode, so `openchamber status` lists it
while the app is open.
