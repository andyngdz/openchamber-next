## Why

OpenChamber desktop is Electron plus the React UI in `packages/ui`. The long-term goal is a desktop app written in Rust with GPUI that replaces both. Before committing to a port of about 600k lines of UI code, one spike has to show that a GPUI app can run the existing OpenChamber server as a sidecar, read sessions from it, send a prompt and render the streamed reply on this Linux Wayland machine.

## What Changes

- Add a Cargo workspace at `packages/desktop-rs` with four crates: `oc-sidecar`, `oc-client`, `oc-state`, `oc-app`.
- The app spawns `openchamber serve --foreground --quiet --api-only` from the existing CLI, reads the port, and stops the server on exit.
- The window has the same layout as the current OpenChamber desktop: sidebar, header, chat column, Work status panel and icon rail. Regions outside the spike's features render in place and are inert.
- The app lists the sessions of one directory, opens a session, shows its messages, creates a new session, sends a prompt and renders the reply live from `/api/event`.
- No TypeScript file changes. Web, VS Code, mobile and the Electron desktop keep working as they do now.

## Capabilities

### New Capabilities

- `desktop-rs-spike`: a GPUI desktop client that runs the OpenChamber server as a sidecar and covers listing sessions, reading messages, sending a prompt and streaming the reply.

### Modified Capabilities

None.

## Impact

- New directory `packages/desktop-rs`, ignored by Bun because it has no `package.json`.
- Running the app needs `bun install` at the repo root, Node 24.14 or later, and `opencode` on `PATH`.
- Sending a prompt calls a real model and spends tokens.
