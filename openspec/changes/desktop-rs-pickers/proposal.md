## Why

The Rust + GPUI desktop spike opens one directory and shows a fixed model and agent, so it cannot be used for daily work. Picking a project, a model and an agent, and sending with Enter, are the first things a user reaches for.

## What Changes

- The sidebar lists every project from the shared settings with its sessions, plus the Chats, Pinned (read-only) and In work sections, split from one global session list and ordered, limited and shown or hidden by the same settings as the React sidebar.
- The empty state's Choose project popover picks Chat or a project and sets the active project.
- The composer gets a working model picker (search, Favorites, Recent, provider sections that follow `providerOrder`, `hiddenModels` and collapse, star, Thinking variant menu) and agent picker (search, favorites).
- A new session is created with the picked agent and model; an existing session is switched before the prompt when the pick changed.
- Enter sends according to the `enterToSend` setting and never during IME composition. Sending stays blocked while a turn runs.
- Picks are written to the shared settings (`PUT /api/config/settings`), so the Electron app sees the same active project, recents and favorites.
- New layout references for the multi-project sidebar and each popover keep the parity gate covering the new surfaces.

## Capabilities

### New Capabilities

- `desktop-rs-selection`: choosing the project, model, agent and variant in the Rust desktop app, sending with Enter, and keeping those choices in the shared settings.

### Modified Capabilities

- `desktop-rs-spike`: the session list covers every project and the Chats section from the global list instead of one directory, and a reconnect refetches settings, that list and the loaded catalogs.

## Impact

- `packages/desktop-rs` only: `oc-client`, `oc-state`, `oc-app`, the capture tools and fixtures. No TypeScript changes.
- `oc-app` adds direct dependencies on `uuid` and `chrono`, both already in `Cargo.lock`.
- Next and the Electron app share `settings.json` and `preferences.json` through the server, as two Electron windows do today.
