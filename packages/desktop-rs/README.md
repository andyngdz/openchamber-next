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
cargo run -p oc-app
```

The directory has no `package.json`, so Bun ignores it.

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
