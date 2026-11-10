## 1. Cargo workspace   [depends-on: -] [writes: packages/desktop-rs/Cargo.toml, packages/desktop-rs/Cargo.lock, packages/desktop-rs/.gitignore, packages/desktop-rs/README.md, packages/desktop-rs/crates/*/Cargo.toml, packages/desktop-rs/crates/*/src/lib.rs, packages/desktop-rs/crates/oc-app/src/main.rs] [parallel-safe: yes]

- [ ] 1.1 Create the workspace: edition 2024, `rust-version = "1.92"`, members `oc-sidecar`, `oc-client`, `oc-state`, `oc-app`, `[profile.dev.package]` `opt-level = 3` for the GPUI crates listed in the GPUI Kit installation guide, `.gitignore` with `/target`.
- [ ] 1.2 Add the crates with every dependency they will need, internal and external, as listed in design.md "Package decisions" and the crate edges (`oc-app` -> the other three, `oc-state` -> `oc-client`, `oc-sidecar` -> none), so this section writes the final `Cargo.lock`. Later sections add no dependency; one that needs a new dependency stops and reports it.
- [ ] 1.3 `oc-app/src/main.rs` opens one empty `gpui-kit` 0.9.1 window with a 1440x900 content size and exits on close.
- [ ] 1.4 README: prerequisites (`bun install` at the repo root, Node 24.14+, `opencode` on `PATH`, the Linux packages GPUI Kit needs), run command, test env flags `OC_DESKTOP_IT` and `OC_DESKTOP_UI`, and the note that `openchamber status` lists the sidecar.
- [ ] 1.5 Confirm Bun still resolves its workspace (`bun install` at the root succeeds and lists no `desktop-rs` package). Run the window on this Wayland session and record whether it renders; if not, record the XWayland result.
- [ ] 1.6 Gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

## 2. API and event fixtures   [depends-on: -] [writes: packages/desktop-rs/tools/capture-api.mjs, packages/desktop-rs/fixtures/api/, packages/desktop-rs/fixtures/events/] [parallel-safe: yes]

- [ ] 2.1 Write `tools/capture-api.mjs`. It reuses `startFixture`, `isolatedEnv`, `ensureProject` and `fixtureProviderConfig` from `scripts/perf/isolated-server.mjs` and `scripts/perf/fixture-provider.mjs`, and spawns `node packages/web/bin/cli.js serve --foreground --quiet --api-only --host 127.0.0.1 --port <port>` itself, since `startIsolatedServer` has no `--api-only` and needs a built `dist`. The OpenCode config sets `permission` for `glob` to `ask`.
- [ ] 2.2 Confirm `/auth/session`, `/api/session`, `/api/session/:id/message`, `/api/session/:id/prompt`, `/api/session/:id/interrupt` and `/api/event` answer under `--api-only`. If one does not, stop and report.
- [ ] 2.3 Save raw responses to `fixtures/api/`: auth status, session list (empty and non-empty), create session, message list of a finished turn, the prompt request body the React UI sends (`packages/ui/src/lib/opencode/client.ts:1610-1620`) and its response.
- [ ] 2.4 Save raw `/api/event` sequences to `fixtures/events/`: a plain text reply (`stream-300cps`); an agent turn with tool calls (`agent-20tools-300cps`); a turn the script interrupts with `POST /api/session/:id/interrupt`; a turn whose `glob` call asks for permission, which the script then answers; and events from a second session in the same stream.
- [ ] 2.5 Check each saved event type against its `case` in `packages/ui/src/lib/opencode/events.ts` and list every field that differs from design.md "State".

## 3. Layout reference   [depends-on: -] [writes: packages/desktop-rs/tools/capture-reference.mjs, packages/desktop-rs/fixtures/seed.json, packages/desktop-rs/fixtures/reference/] [parallel-safe: yes]

- [ ] 3.1 Write `fixtures/seed.json`: one project directory and the fixed titles of the sessions the capture creates.
- [ ] 3.2 Write `tools/capture-reference.mjs`: start the Electron dev window with `OPENCHAMBER_ELECTRON_REMOTE_DEBUGGING_PORT=9222`, a temporary `OPENCHAMBER_DESKTOP_USER_DATA_DIR`, and an isolated `HOME`, `OPENCHAMBER_DATA_DIR` and `OPENCODE_CONFIG_CONTENT` with the fixture provider; seed the project and sessions from `fixtures/seed.json`; drive it with `scripts/perf/cdp.mjs`; set the content area to 1440x900 with `Emulation.setDeviceMetricsOverride`; reset the sidebar to 280 px and the Work status panel to its default width; stop the window by the pid of `electron-dev.mjs` and remove the temporary folders afterwards.
- [ ] 3.3 For the states empty, session, streaming, write `fixtures/reference/<state>.json` (bounds of sidebar, header, chat column, composer, Work status panel, icon rail; per region the interactive controls in DOM order with role and accessible name; the widths it set) and `fixtures/reference/<state>.png`. Streaming uses model `agent-20tools-300cps`.
- [ ] 3.4 Report controls with a missing or duplicate accessible name and record the fallback key (`data-*` attribute or DOM path) used for each.
- [ ] 3.5 Run the script twice and confirm the JSON output is identical.

## 4. Sidecar   [depends-on: 1] [writes: packages/desktop-rs/crates/oc-sidecar/src/, packages/desktop-rs/crates/oc-sidecar/examples/] [parallel-safe: yes]

- [ ] 4.1 Spawn API taking the CLI path, working directory and extra env vars: CLI path resolution (`OPENCHAMBER_CLI`, else walk up to `packages/web/bin/cli.js`), free-port pick, spawn with `std::process::Command` and `PR_SET_PDEATHSIG(SIGTERM)` in `pre_exec`, first-line read with a 30 s timeout, port-line parsing that rejects `<port> pass:<pw>`.
- [ ] 4.2 Retry with a new port on the "already in use" stderr message, at most 3 attempts; typed errors carrying exit status, stderr tail and the tail of `openchamber-<port>.log` in the data dir's `logs` folder.
- [ ] 4.3 Shutdown: record `opencode serve` children of the CLI from `/proc`, SIGTERM, wait 10 s, then SIGKILL the CLI and SIGTERM each recorded OpenCode pid still alive; `Drop` runs the same path. Expose the CLI pid and recorded OpenCode pids for tests.
- [ ] 4.4 Example binary `examples/hold.rs`: starts the sidecar from env settings, prints the CLI pid, port and OpenCode pids as one JSON line, then waits forever.
- [ ] 4.5 Unit tests: port-line parsing (`<port>`, `<port> pass:x`, garbage, empty), "already in use" detection, log path for a given port and data dir.
- [ ] 4.6 Gate as in 1.6.

## 5. Client   [depends-on: 1, 2] [writes: packages/desktop-rs/crates/oc-client/src/] [parallel-safe: yes]

- [ ] 5.1 Serde types for the fields the spike uses, shaped from `fixtures/api/` and `fixtures/events/`, with `#[serde(default)]` for optional fields; the wire event envelope with unknown types kept as a raw variant.
- [ ] 5.2 `reqwest` client with cookie store and `x-opencode-directory`: auth check requiring `authenticated: true`, list sessions, create session, list messages, send prompt, and the `/api/event` stream through `eventsource-stream`.
- [ ] 5.3 `thiserror` error enum (transport, HTTP status with body, decode, auth); a non-2xx or undecodable body is an error, never an empty list.
- [ ] 5.4 Unit tests: every fixture in `fixtures/api/` and every event in `fixtures/events/` decodes.
- [ ] 5.5 Gate as in 1.6.

## 6. State reducer   [depends-on: 2, 5] [writes: packages/desktop-rs/crates/oc-state/src/] [parallel-safe: no]

- [ ] 6.1 State struct: session list, open session, messages with ordered text and tool items, session status, pending-permission flag, connection status, per-region error, composer draft.
- [ ] 6.2 Reducer for the wire events in design.md "State", with part identity `(assistantMessageID, ordinal)` for text and call id for tools, ported from the matching `case` in `packages/ui/src/lib/opencode/events.ts`; filter by known session; unknown and reasoning events dropped with a debug log.
- [ ] 6.3 Actions for app-side changes: connection status changes, region errors and retries, prompt sent, prompt failed (draft kept), loaders that turn the session list and message list responses into state.
- [ ] 6.4 Unit tests from `fixtures/events/`: full text reply, agent turn with tools, interrupted turn, permission asked then replied, a `session.text.delta` before its `session.text.started`, events from another session, unknown events. Unit tests for actions: reconnecting and connected status, a region error then retry, the draft kept after a failed prompt.
- [ ] 6.5 Gate as in 1.6.

## 7. Window layout and parity test   [depends-on: 3, 6] [writes: packages/desktop-rs/crates/oc-app/src/, packages/desktop-rs/crates/oc-app/tests/parity.rs, packages/desktop-rs/fixtures/states/] [parallel-safe: no]

- [ ] 7.1 Theme from the default dark tokens in `packages/ui/src/styles/design-system.css:164-195`.
- [ ] 7.2 Every region and control from design.md "Layout" rendered from `oc-state` state, in the reference order, with each control's `ElementId` set to the reference name; inert controls do nothing on click; UI text in English.
- [ ] 7.3 Layout registry filled during prepaint, and `--dump-layout <file> --state <name>` that loads `fixtures/states/<name>.json`, renders one frame at a 1440x900 content size, writes the reference JSON shape and exits.
- [ ] 7.4 Write `fixtures/states/` for empty, session, streaming (from `fixtures/events/`, with the titles from `fixtures/seed.json`) and server-error.
- [ ] 7.5 Parity test `tests/parity.rs`, run only with `OC_DESKTOP_UI=1`: for empty, session and streaming, region bounds equal the reference, no reference control is missing, order matches; server-error region bounds equal the empty state's.
- [ ] 7.6 Gate as in 1.6, plus `OC_DESKTOP_UI=1 cargo test -p oc-app --test parity` on this machine. Save GPUI screenshots next to `fixtures/reference/*.png` for the user to compare.

## 8. Live wiring   [depends-on: 4, 5, 7] [writes: packages/desktop-rs/crates/oc-app/src/] [parallel-safe: no]

- [ ] 8.1 Spawn the sidecar on the main thread before `Application::run`, and call `record_opencode_pids()` once the auth check succeeds; start a tokio runtime on a background thread; send results and events over `futures::channel::mpsc` to a `cx.spawn` task that applies them to the state `Entity`.
- [ ] 8.2 Startup flow: auth check, session list, header server state; New session, open session, send prompt; send disabled while busy or disconnected.
- [ ] 8.3 Reconnect loop that takes its connect function as a parameter: backoff 500 ms doubling to 10 s, then refetch the session list and open messages and call `record_opencode_pids()` again, since the server may have restarted OpenCode under a new pid. Unit test with a stream that fails twice, checking the delays and the refetch.
- [ ] 8.4 Failure states from design.md "Failure behavior", including server exit while running, each with its retry action.
- [ ] 8.5 Window close runs the sidecar shutdown before the process exits.
- [ ] 8.6 Gate as in 1.6, plus the parity test from 7.6.

## 9. Verification   [depends-on: 8] [writes: packages/desktop-rs/crates/oc-app/tests/sidecar_it.rs] [parallel-safe: no]

- [ ] 9.1 Integration test `tests/sidecar_it.rs`, run only with `OC_DESKTOP_IT=1`, with an isolated `HOME`, data dir and fixture-provider config passed through the sidecar's env parameter: start, auth, list sessions, open the event stream, send a prompt and see text deltas arrive, shut down, and check that the CLI pid and the recorded `opencode serve` pids are gone.
- [ ] 9.2 Case with SIGSTOP on the CLI so the SIGKILL path runs; the same pid check.
- [ ] 9.3 Crash case: run `oc-sidecar`'s `hold` example, SIGKILL it, and check that the CLI and OpenCode pids it printed are gone within 15 s.
- [ ] 9.4 Manual check on this Wayland machine: create a session, send a short prompt to a real model and watch the reply stream; send SIGKILL to the CLI and see the server-exit state; restart the app, close the window, then `pgrep -f "openchamber serve"` and `pgrep -f "opencode serve"` show nothing this app started.
- [ ] 9.5 Run the gate from 1.6 and both env-gated test suites; report what passed, what did not, and the Wayland result from 1.5.
