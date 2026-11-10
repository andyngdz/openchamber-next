# Rust + GPUI desktop spike

## Problem

OpenChamber desktop is Electron (`packages/electron`, about 24k lines) plus the React UI (`packages/ui`, about 600k lines without tests). Electron also runs the OpenChamber server in its own process: `packages/electron/main.mjs:1600` imports `startWebUiServer` from `@openchamber/web/server`. The goal is a desktop app in Rust with GPUI. The port is large, so one spike has to answer the risky questions first:

- Can a Rust app start, use and stop the existing Node server as a sidecar without leaving processes behind?
- Can it authenticate, read sessions and messages, send a prompt and follow the SSE stream?
- Does GPUI Kit render and take input on this Linux Wayland machine?

## Decision trace

All decisions below were made by the user in the planning session:

- Scope is the desktop app only. Web, VS Code and mobile stay on TypeScript.
- The first stage is Rust + GPUI for the shell and UI. The Node server stays as a sidecar; a Rust server is a later, optional stage.
- Code lives in `packages/desktop-rs` inside this repo.
- The spike covers reading sessions and sending a prompt (approach "read + send").
- Approach A: spawn the existing CLI, authenticate with the session cookie, hand-written serde types for the few endpoints used. The user asked for the approach that reuses the most existing code; approach A changes no TypeScript file.
- Layout matches the current OpenChamber desktop 100%: every region and component sits where it does today. Styling (colors, fonts, radii) may differ. UI text is English.
- The spike builds the full layout. Regions and controls outside the spike's features render in place and are inert.

## Goals

- One GPUI window on Linux (Wayland session) with the same layout as the current OpenChamber desktop: sidebar, header, chat column with composer, Work status panel and icon rail.
- The app starts the server, lists sessions of one directory, opens one, creates a new one, sends a prompt and renders the reply as it streams.
- Closing the app leaves no `node` or `opencode` process from this app running.
- Every failure (server start, request, stream) shows as a visible state, never as an empty list.

## Non-Goals

- macOS, Windows, packaging, auto-update, tray, menus, notifications.
- Matching the React app's exact styling. Colors come from OpenChamber's default dark tokens where GPUI Kit takes them; fonts, radii and spacing inside components may differ.
- Behavior of the inert regions: Work status panel content, icon rail panels, sidebar toolbar actions, session menus, project grouping actions, model, agent, permission, goal, attach and mic controls.
- Markdown rendering, syntax highlighting, diffs, tool call details beyond the tool name and status.
- UI password, remote hosts, SSH, LAN access, settings.
- Model, agent or permission pickers. Prompts use the server's default model and agent.
- Permission and form prompts from the agent: the spike shows that one is pending and does not answer it.
- Any change to TypeScript code or to the server API.
- An OpenAPI-generated client. No OpenAPI spec is available locally (`node_modules` is not installed), so the generated client is deferred to the foundation stage.

## Approaches considered

- **A. Sidecar through the existing CLI (chosen).** `openchamber serve --foreground --quiet` already prints the port as the first stdout line (`packages/web/bin/lib/commands-serve.js:225-232`) and stops the server cleanly on SIGTERM (`commands-serve.js:247-269`). No TypeScript change.
- **B. Attach to a server the user starts.** Smaller, but it does not test the sidecar lifecycle that Electron handles today.
- **C. A new Node entry file calling `startWebUiServer` with desktop options.** Closer to Electron (`OPENCHAMBER_RUNTIME=desktop`, notification hooks), but it changes `packages/web` during a spike.

## Design

### Workspace layout

```text
packages/desktop-rs/
  Cargo.toml            workspace, edition 2024, rust-version 1.92, Cargo.lock committed
  .gitignore            /target
  README.md             prerequisites and run command
  crates/oc-sidecar/    spawn, port handshake, shutdown
  crates/oc-client/     HTTP + SSE client, serde types
  crates/oc-state/      pure state reducer, no GPUI
  crates/oc-app/        GPUI binary
  fixtures/             JSON captured from a real server run
```

Dependencies go from `oc-app` to the other three; `oc-state` depends on the types in `oc-client`; `oc-sidecar` depends on nothing internal. Bun ignores the directory because it has no `package.json`, even though the root `workspaces` glob is `packages/*`.

### Sidecar (`oc-sidecar`)

- Resolves the CLI path from env `OPENCHAMBER_CLI`, else `<repo>/packages/web/bin/cli.js`, where `<repo>` is found by walking up from the executable and the working directory until a directory has `packages/web/bin/cli.js`.
- Picks a free port by binding `127.0.0.1:0` and closing the socket, then runs `node <cli> serve --foreground --quiet --api-only --host 127.0.0.1 --port <port>`. The CLI rejects `--port 0` (`packages/web/bin/lib/cli-args.js:204-206`). An explicit port has no fallback: if another process takes it first, the CLI exits non-zero with `Port N is already in use by another process.` on stderr (`commands-serve.js:88-100`). The sidecar then picks a new port and retries, at most 3 attempts in total; any other failure is not retried.
- With `--quiet`, stdout carries only the port line. The CLI redirects stdout and stderr to `openchamber-<port>.log` in `$OPENCHAMBER_DATA_DIR/logs`, else `~/.config/openchamber/logs` (`commands-serve.js:107-108`, `:179-186`, `cli-paths.js:9-18`), before it starts the server, so most startup failures land in that file, not on stderr.
- Reads the first stdout line within 30 seconds and parses it as `<port>`. A line of `<port> pass:<pw>` is rejected as an error, because the spike runs without a UI password. Timeout, early exit, or an unparseable line is a typed error that carries the exit status, the last 20 lines of stderr and the last 20 lines of the log file written since the spawn.
- On Linux the child gets `PR_SET_PDEATHSIG(SIGTERM)` in `pre_exec` (crate `nix`), so a crashed app still stops the server. The signal fires when the spawning thread exits, so the spawn uses `std::process::Command` on the main thread before `Application::run` takes it over, never `tokio::process` on the runtime thread.
- `shutdown()` sends SIGTERM and waits up to 10 seconds. The CLI's SIGTERM handler calls `controller.stop()`, which stops the managed OpenCode process. `Drop` calls the same path.
- OpenCode runs in its own process group (`spawn` with `detached: true`, `packages/web/server/lib/opencode/lifecycle.js:428-434`), so killing the CLI does not reach it. Before SIGTERM, the sidecar records the pids of `opencode serve` processes whose parent is the CLI (from `/proc`). If the CLI is still alive after 10 seconds, it sends SIGKILL to the CLI and SIGTERM to each recorded OpenCode pid that is still running. Once the CLI is gone, by any path, including a SIGKILL from outside, shutdown sends SIGTERM to every recorded OpenCode pid still running. OpenCode starts about 100 ms after the port line, so the app calls `record_opencode_pids()` again once the server answers. When the app crashes, `PR_SET_PDEATHSIG` sends SIGTERM to the CLI, and the CLI's own shutdown stops OpenCode.
- Foreground serve writes PID and instance files and runs as `OPENCHAMBER_RUNTIME=web` (`commands-serve.js:165-169`, `:214-223`), so `openchamber status` lists the sidecar. The README says so.

Assumption checked in the first task: `--api-only` still serves `/auth/session`, `/api/*` and `/api/event`.

### Client (`oc-client`)

- `reqwest` with the `cookies`, `json` and `stream` features. One `Client` per server, base URL `http://127.0.0.1:<port>`.
- Auth: `GET /auth/session` first and require `authenticated: true` in the body. With no UI password and client auth off, the server answers `{authenticated: true, disabled: true}` and lets every request through (`packages/web/server/lib/ui-auth/ui-auth.js:568-571`, `:606-619`). Any other answer is an auth error shown to the user. The cookie store stays on so a later stage can add password login. The spike mints no `oc_client_` token.
- Every OpenCode call sends `x-opencode-directory: <directory>`.
- Calls: list sessions (`GET /api/session`), create session (`POST /api/session`), list messages (`GET /api/session/:id/message`), send prompt (`POST /api/session/:id/prompt`), event stream (`GET /api/event`, parsed with `eventsource-stream`).
- Types are hand-written with serde and hold only the fields the spike uses, with `#[serde(default)]` for optional fields. Request and response shapes come from fixtures captured from the real server and from what the React UI sends (`packages/ui/src/lib/opencode/client.ts:1119-1131`, `:1610-1620`). A test parses each fixture.
- Errors are a `thiserror` enum: transport, HTTP status with body, decode. A non-2xx response or decode failure is an error, never an empty list.

### State (`oc-state`)

A pure reducer with no GPUI or I/O, so it is unit-tested with fixtures.

- Holds the session list, the open session id, its messages and parts in order, the session status (idle, busy, error), the pending-permission flag and the connection status.
- `/api/event` carries OpenCode v2 wire events. The React UI translates them into its own message and part vocabulary in `packages/ui/src/lib/opencode/events.ts`; `oc-state` ports the subset of that translation the spike needs, with the same part identity: text and reasoning items by `(assistantMessageID, ordinal)`, tool calls by call id.
- Wire events handled, as captured in `packages/desktop-rs/fixtures/events/` (differences from `events.ts` are listed in that folder's `README.md`):
  - Session list: `session.created` (no title), `session.renamed` (sets the title), `session.deleted`.
  - User message: `session.inbox.enqueued` adds it, `session.inbox.delivered` marks it delivered. The stream has no other user-message event.
  - Status: `session.execution.started` sets busy; `session.execution.succeeded` and `session.execution.interrupted` set idle; `session.execution.failed` sets error. `session.status` and `session.idle` are handled the same way when they arrive, but the captured server never sent them.
  - Assistant messages: each `session.step.started` opens a new assistant message (one per step, so a 20-tool turn has 21); `session.step.ended` closes it and feeds the meta row. `data.model` is an object.
  - Text: `session.text.started`, `session.text.delta` appends, `session.text.ended` replaces the item with its full text.
  - Tools: `session.tool.input.started` creates the row and carries the tool name; `session.tool.called`, `session.tool.success`, `session.tool.failed` update its status by call id.
  - Pending answer: `permission.asked` (request id in `data.id`) sets it; `permission.replied` (id in `data.requestID`) clears it. `form.created`, `form.replied`, `form.cancelled` do the same; the fixture provider cannot produce them, so their tests use hand-built payloads shaped from `events.ts`.
- The stream has no directory filter, so the reducer keeps events whose session is in the directory's session list or is the open session, and drops the rest.
- Unknown event types are dropped and logged at debug level. Reasoning events are dropped in the spike.
- A text delta for an item not yet seen creates the item, so a missed `session.text.started` does not lose text.

### App (`oc-app`)

- `gpui-kit = "0.9.1"` with default features and the default gpui-pre backend. Root `Cargo.toml` sets `[profile.dev.package]` `opt-level = 3` for the GPUI crates listed in the GPUI Kit installation guide.
- Usage: `oc-app [--directory <path>] [--dump-layout <file> --state <name>]`, default the current directory.
- Theme: a GPUI Kit theme built from the default dark tokens in `packages/ui/src/styles/design-system.css:164-195`.

### Layout

The reference is the running React app at a 1440x900 window, measured in the planning session through Chrome DevTools:

| Region | Bounds today | Spike content |
| --- | --- | --- |
| Sidebar | x 0, width 280, full height | Live: New session row, Chats with sessions of the directory, active session highlight, busy indicator. Inert: sidebar toggle, toolbar (search, add project, filter, more), In work grouping, Projects section, row menus, settings button |
| Header | x 280, height 48, to the right edge | Live: session title. Inert: title menu, context percentage, goal icon, panel toggle |
| Chat column | x 280, from y 48, between sidebar and Work status panel | Live: messages. Inert: prompt navigation, message action icons |
| Composer | bottom of the chat column, bounds captured by the reference task | Live: text area and send. Inert: attach, permission, goal, model / variant, agent, mic |
| Work status panel | width 324, right of the chat column | Inert: section headers Session, Project, Usage, Context sources, Turn stats with placeholder rows |
| Icon rail | width 44, right edge | Inert: the same icons in the same order |
| Empty state | chat column when no session is open | Live: heading and composer. Inert: Choose project, quick-action chips |

Inside the message list the order matches today: user message aligned right; assistant text; tool calls grouped under an Activity row; a meta row with model, agent and duration under each assistant turn. Text parts render as plain text; tool rows show `tool name: status`.

The composer send button is disabled while the session is busy or the server is not connected. Server state shows in the header area where the context percentage sits today, as starting, connected, reconnecting, or the error text.

### Parity reference

Screenshots alone do not catch a missing button, so the reference is structured data plus screenshots:

- The reference surface is the Electron dev window, the desktop app this port replaces. A Node script, `packages/desktop-rs/tools/capture-reference.mjs`, starts it with `OPENCHAMBER_ELECTRON_REMOTE_DEBUGGING_PORT=9222` and a fresh `OPENCHAMBER_DESKTOP_USER_DATA_DIR`, as `.agents/skills/desktop-shell/SKILL.md` describes, and drives it over CDP with `scripts/perf/cdp.mjs` at a 1440x900 window. Before capturing it resets the sidebar to 280 px and the Work status panel to its default width, since both are resizable (`packages/ui/src/components/layout/Sidebar.tsx:11-14`), and records the widths it set.
- 1440x900 is the content area, not the outer window frame: the script sets it with the CDP `Emulation.setDeviceMetricsOverride` call, and `oc-app` opens its window with a 1440x900 content size.
- The Electron window runs against an isolated `HOME`, `OPENCHAMBER_DATA_DIR` and `OPENCODE_CONFIG_CONTENT`, built with the helpers in `scripts/perf/isolated-server.mjs`, and a project seeded from `fixtures/seed.json` with fixed session titles. The control lists then hold the seed's names, never the user's own sessions, and `fixtures/states/` uses the same seed.
- Streaming uses the deterministic model provider in `scripts/perf/fixture-provider.mjs` (model `agent-20tools-300cps`), so captures spend no tokens and every run streams the same text and tool calls.
- It captures three states: empty state, open session at rest, open session while a reply streams. For each state it writes `fixtures/reference/<state>.json` with the bounds of every region in the table above and, per region, the list of interactive controls in DOM order, each with its role and accessible name. It also saves `fixtures/reference/<state>.png`. The server-error state exists only in the Rust app, since Electron runs the server in its own process; it has no reference.
- `oc-app --dump-layout <file> --state <name>` builds the window from a fixture event sequence in `fixtures/states/<name>.json` fed to the `oc-state` reducer, with no server, renders one frame, writes the same JSON shape and exits. GPUI does not expose final element bounds for walking, so `oc-app` keeps its own registry: each region and control registers its `ElementId` and bounds during prepaint. Each control's id is the React control's accessible name, so the two lists compare by name.
- The parity test fails when a region's bounds differ, when a reference control is missing from the GPUI dump, or when the order inside a region differs. For the server-error state it checks only that the regions have the same bounds as the empty state. The PNGs are for a person comparing the two side by side; no pixel diff runs, because styling may differ.

### Runtime

- Async: one multi-thread tokio runtime on a background thread does all I/O. It sends results and events over a `futures::channel::mpsc` channel; a GPUI task started with `cx.spawn` receives them, applies them to the `oc-state` reducer held in an `Entity`, and calls `cx.notify()`.
- SSE reconnect: backoff starting at 500 ms, doubling up to 10 s. After a reconnect the app refetches the session list and the open session's messages, because events during the gap are lost.
- Closing the window calls the sidecar shutdown before the process exits.

### Failure behavior

| Case | What the user sees |
| --- | --- |
| `node` missing, CLI not found, port line never arrives | Header server state shows the error; the chat column shows the error with the stderr tail and the log file tail and a retry button; the sidebar session list shows "Server not running" |
| List or message fetch fails | Error text in that region with a retry button |
| Prompt fails | Error above the composer; the typed text stays in the composer |
| SSE drops | Header server state shows "Reconnecting"; existing messages stay on screen |
| Server exits while running | Header server state shows the exit status; composer disabled |

## Package decisions

| Need | Package | Why |
| --- | --- | --- |
| UI | `gpui-kit` 0.9.1 | Chosen framework; ships list, input, button and scroll components |
| HTTP, cookies, streaming | `reqwest` | Standard client with a built-in cookie store |
| SSE parsing | `eventsource-stream` | Parses an SSE byte stream from reqwest |
| Async runtime | `tokio`, `futures` | reqwest needs tokio; `futures` mpsc works across both executors |
| JSON | `serde`, `serde_json` | Standard |
| Errors | `thiserror` | Typed errors per crate |
| Logging | `tracing`, `tracing-subscriber` | Standard |
| Process signals, PDEATHSIG | `nix` | Safe wrappers for `prctl` and `kill` |
| CLI args | `clap` | One flag today, more later |

Nothing in the spike is hand-written where one of these crates does the job.

## Testing

- `oc-state`: unit tests feed captured event sequences from `fixtures/` and check the resulting messages, parts and status, including a `session.text.delta` before its `session.text.started`, events for another session, and unknown events.
- `oc-client`: unit tests decode every fixture into the types.
- Layout parity, run only with `OC_DESKTOP_UI=1` because it opens a window: a test runs `oc-app --dump-layout` at 1440x900 for each reference state and compares it with `fixtures/reference/<state>.json`. Region x, y, width and height match exactly, because with the widths reset the React app sets them as fixed CSS values. Every reference control is present and the order inside each region matches. Content heights inside regions are not compared, since fonts differ.
- `oc-sidecar`: unit tests for port-line parsing (`<port>`, `<port> pass:x`, garbage, empty).
- Integration test, run only with `OC_DESKTOP_IT=1` because it needs `bun install` and `opencode`: start the sidecar, authenticate, list sessions, open the event stream, record the pids of the CLI and of `opencode serve` processes whose parent is the CLI, shut down, then check that none of those pids is alive. A second case blocks the CLI's SIGTERM handling by sending SIGSTOP first, so the SIGKILL path runs, and checks the same pids. A third case starts the sidecar from a small example binary, kills that binary with SIGKILL, and checks the same pids, which tests the parent-death signal. These tests live in `crates/oc-app/tests/`, which already depends on both crates, and start the server with an isolated `HOME`, data dir and the fixture provider through an env parameter of the sidecar spawn.
- Reconnect: the reconnect loop takes its connect function as a parameter; a unit test feeds a stream that fails twice and checks the backoff delays and that a refetch runs after the reconnect.
- Failure states: reducer tests cover connection status, per-region errors and the composer draft kept after a failed prompt.
- Gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Manual check on this machine (Wayland): run the app in a directory, create a session, send a short prompt to a real model, watch the reply stream, close the window, then `pgrep -f "openchamber serve"` and `pgrep -f "opencode serve"` print nothing that this app started.

## Risks

- GPUI Kit's gpui-pre backend may not render on this Wayland setup. The first app task finds out; X11 through XWayland is the fallback to note in the result.
- Wire event fields may differ from what `events.ts` suggests. Fixtures captured from a real run settle this before types are written.
- Accessible names in the React app may be missing or duplicated for some controls. The capture script reports those, and the reference falls back to a stable `data-*` attribute or DOM path for them.
- `--api-only` may drop a route the spike needs. The sidecar task checks it before the client is built.
