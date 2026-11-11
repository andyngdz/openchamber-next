## Why

The Rust + GPUI desktop app records a pending permission or form only as an id for the sidebar badge, has no way to stop a running turn, and blocks every send while a turn runs. A session hangs as soon as an agent asks for a permission or asks a question, so the app cannot be used for agent work.

## What Changes

- A permission dock above the composer answers every pending request of the open session and its subagents with Allow once, Always or Deny, with a per-tool preview: the command highlighted, the diff of an edit, the content of a write, the request of a web fetch.
- A form dock answers agent questions and other forms one field per step, with every field type OpenCode sends, the web search consent card, and a recovered question that is offered again after its turn died.
- Stop replaces Send while a turn runs; Escape twice within 3 seconds stops it too.
- The composer's shield button cycles the session's permission auto-accept mode (ask, safety, auto) through the server's policy.
- Sending while a turn runs follows `followUpBehavior`: queue the message in the server's message queue, or steer it into the running turn; Ctrl+Enter sends a plain prompt. Sending dismisses the session tree's pending permissions and forms first. A strip above the composer lists queued messages with edit, send now, remove and drag to reorder.
- The app reads a second server stream, `/api/openchamber/events`, for the queue, auto-accept and routing updates, and reconnects it on its own.
- Code in the permission preview is highlighted by gpui-kit's tree-sitter highlighter, colored from the app theme as React's `zedThemeFromAppTheme` does.

## Capabilities

### New Capabilities

- `desktop-rs-turn-control`: answering permissions and forms, stopping a turn, the auto-accept mode, and sending while a turn runs through the message queue or steering, in the Rust desktop app.

### Modified Capabilities

- `desktop-rs-selection`: Enter while a turn runs queues, steers or sends instead of doing nothing.
- `desktop-rs-spike`: an event stream reconnect also refetches the pending permissions and forms.

## Impact

- `packages/desktop-rs` (`oc-client`, `oc-state`, `oc-app`, the capture tools and fixtures) and `scripts/perf/fixture-provider.mjs`, which gains fixture models that ask for a permission, edit a file and ask a question. No other TypeScript changes.
- `gpui-kit` gains the `tree-sitter-languages` feature: 37 crates in `Cargo.lock`, and the stripped release binary grows from 49.3 MB to 100.0 MB.
