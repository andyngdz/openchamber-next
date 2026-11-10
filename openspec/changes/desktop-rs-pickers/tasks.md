## 1. API fixtures   [depends-on: -] [writes: packages/desktop-rs/tools/capture-api.mjs, packages/desktop-rs/fixtures/api/] [parallel-safe: yes]

- [ ] 1.1 Extend `tools/capture-api.mjs`: the capture's OpenCode config registers the fixture provider under two provider ids with several models, one with variants, plus one hidden agent and one subagent; settings are seeded with two projects (one with `color` and `icon` set), favorites, recents and `lastUsedEfforts`.
- [ ] 1.2 Save raw responses to `fixtures/api/`: `GET /api/fs/home`; `POST /api/fs/mkdir`; `GET` and `PUT /api/config/settings?surface=desktop` (the PUT request body and the full response); `GET /api/config`, `/api/provider`, `/api/model?types=chat&types=image`, `/api/agent` for a project directory; `GET /api/session/:id`; `POST /api/session` with `agent` and `model` (with and without `variant`); `POST /api/session/:id/model` and `/agent`; the global `GET /api/session?limit=2` first page and its `cursor.next` page, with an archived session, a `/btw` fork, a pinned session and a Chat session among them.
- [ ] 1.3 Confirm every new route answers under `--api-only`; if one does not, stop and report.
- [ ] 1.4 Run the capture twice and confirm the output is identical apart from the fields the README lists as volatile; no personal path in any file.

## 2. Search port   [depends-on: -] [writes: packages/desktop-rs/tools/gen-fuse-fixture.mjs, packages/desktop-rs/fixtures/search/, packages/desktop-rs/crates/oc-state/src/search/, packages/desktop-rs/crates/oc-state/src/lib.rs] [parallel-safe: yes]

- [ ] 2.1 `tools/gen-fuse-fixture.mjs`: runs Fuse.js 7.1.0 from `packages/ui/node_modules` with React's options over a fixed corpus (project, model and agent-like strings, non-ASCII included) and a fixed query set (typos, short tokens, non-ASCII), and writes `fixtures/search/fuse.json` with indices and scores; also writes `rankByQuery` and `matchesRankQuery` results for the same corpus.
- [ ] 2.2 `oc-state/src/search/`: Fuse.js Bitap port (UTF-16 code units, norm via `sqrt`), `rank_by_query`, `matches_rank_query`, as design.md "Search".
- [ ] 2.3 Tests: the fixture's indices and order match exactly, scores within the derived 2-ULP bound; the applicable cases from `packages/ui/src/lib/search/fuzzySearch.test.ts`.
- [ ] 2.4 Gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; from the repo root `passgate-engine rust --scope packages/desktop-rs`.

## 3. Layout references   [depends-on: -] [writes: packages/desktop-rs/tools/capture-reference.mjs, packages/desktop-rs/fixtures/seed.json, packages/desktop-rs/fixtures/reference/v2/, packages/desktop-rs/fixtures/states/v2/] [parallel-safe: yes]

- [ ] 3.1 Seed and capture the `projects` state and the four popup states (`project-popover`, `model-popover`, `agent-menu`, `thinking-menu`) as design.md "Parity reference"; add the `popover` region rule to `REGIONS` with its comment.
- [ ] 3.2 Recapture empty, session, streaming and server-error references in the same run so every reference shares one fixture setup. Every reference of this run goes to `fixtures/reference/v2/`; the current references stay in place until unit 8.
- [ ] 3.3 Write every state, old and new, in the version 2 format to `fixtures/states/v2/<name>.json` from the responses the run received; the first-format files stay until unit 8 switches the loader.
- [ ] 3.4 Run the capture twice and confirm identical JSON; report any control with a missing or duplicate accessible name.

## 4. Client   [depends-on: 1] [writes: packages/desktop-rs/crates/oc-client/src/, packages/desktop-rs/crates/oc-app/src/live/, packages/desktop-rs/crates/oc-app/src/app/, packages/desktop-rs/crates/oc-app/tests/sidecar_it.rs] [parallel-safe: no]

- [ ] 4.1 `Directory` newtype; `Client::new(port)` without a directory; every scoped call takes `&Directory`; `events()` unscoped (global stream).
- [ ] 4.2 New calls and request fields from design.md "Client", including the global session page with cursor and the stop rules.
- [ ] 4.3 Settings types: typed keys plus `projects` as raw JSON objects with typed accessors and a field patch that keeps unknown fields; model refs keyed by catalog `id`.
- [ ] 4.4 Adapt `oc-app`'s existing call sites so the workspace compiles and behaves as before for one directory.
- [ ] 4.5 Tests: every new fixture decodes; a patched project keeps `color` and `icon` byte for byte; the page loop stops on each of its three stop conditions.
- [ ] 4.6 Gate as in 2.4, plus `OC_DESKTOP_IT=1 cargo test -p oc-app --test sidecar_it`.

## 5. Sidebar state   [depends-on: 2, 4] [writes: packages/desktop-rs/crates/oc-state/src/, packages/desktop-rs/crates/oc-client/src/types.rs, packages/desktop-rs/crates/oc-client/src/types_test.rs, packages/desktop-rs/crates/oc-client/src/metadata.rs, packages/desktop-rs/crates/oc-client/src/metadata_test.rs] [parallel-safe: no]

- [ ] 5.1 Settings and home info in state; global list pages merged as design.md "App" (first page early, complete list replaces, incomplete state).
- [ ] 5.2 Session split: archived, `/btw`, the four chat roots, project membership by path, other sessions hidden; event routing by directory and session id.
- [ ] 5.3 Sections and their settings gates and order; Pinned and In work membership with their exclusions; project order for every `sidebarProjectSortOrder`; session order with the empty rank table; Show more limits; collapse state.
- [ ] 5.4 Unit tests for each rule above, from the unit 1 fixtures and small hand-built inputs.
- [ ] 5.5 Gate as in 2.4.

## 6. Selection state   [depends-on: 2, 4, 5] [writes: packages/desktop-rs/crates/oc-state/src/, packages/desktop-rs/crates/oc-client/src/catalog.rs, packages/desktop-rs/crates/oc-client/src/catalog_test.rs, packages/desktop-rs/crates/oc-client/src/settings.rs, packages/desktop-rs/crates/oc-client/src/settings_test.rs, packages/desktop-rs/crates/oc-client/src/settings_patch.rs, packages/desktop-rs/crates/oc-client/src/settings_patch_test.rs, packages/desktop-rs/crates/oc-client/src/lib.rs] [parallel-safe: no]

- [ ] 6.1 Catalog per directory with the provider, model, `hiddenModels` and `providerOrder` rules (every provider section open); agents filtered and sorted.
- [ ] 6.2 Draft target (Chat or project) and selection per draft and per session, the open session's record as its start.
- [ ] 6.3 Default agent and model cascades including catalog absence, and `setAgent` with its per-session agent memory, ported from the cited lines.
- [ ] 6.4 Picker rules: Recent, `lastSelectedModel`, `recentAgents`, favorites, `lastUsedEfforts`, project pick (`activeProjectId`, `lastOpenedAt`); each produces the settings patch to write.
- [ ] 6.5 `resolveSendSelection` port (variant counts, Default sends no variant) and the Enter policy port (`shouldSubmitEnter`, desktop not expanded).
- [ ] 6.6 Unit tests: each cascade step taken; catalog absence; agent switch and back; every picker rule; the desktop cases of `keyboardPolicy.test.ts:25-52`.
- [ ] 6.7 Gate as in 2.4.

## 7. Live wiring   [depends-on: 6] [writes: packages/desktop-rs/crates/oc-app/src/app/, packages/desktop-rs/crates/oc-app/src/live/, packages/desktop-rs/crates/oc-app/src/root/, packages/desktop-rs/crates/oc-app/src/main.rs, packages/desktop-rs/crates/oc-app/Cargo.toml, packages/desktop-rs/Cargo.toml, packages/desktop-rs/Cargo.lock, packages/desktop-rs/README.md, packages/desktop-rs/crates/oc-client/src/client.rs, packages/desktop-rs/crates/oc-client/src/client_test.rs] [parallel-safe: no]

- [ ] 7.1 Add `uuid` (feature `v4`) and `chrono` (feature `clock`) as design.md "Package decisions" lists; no other dependency.
- [ ] 7.2 Remove `--directory` from the live app; sidecar working directory is the home directory; startup loads settings, home info and the global list with retries.
- [ ] 7.3 Catalog and agent loading per directory with Retry; refetch after reconnect as the MODIFIED requirement says.
- [ ] 7.4 Settings writer: partial PUTs, 200 ms and 700 ms debounces, response replaces state, failure keeps the choice and logs, flush on window close before the sidecar stops.
- [ ] 7.5 Send sequence: Chat folder via `mkdir`, create with agent and model, or `/agent` and `/model` then prompt in an open session; every failure keeps the draft and shows the composer error.
- [ ] 7.6 README: launch without `--directory`, the shared settings note.
- [ ] 7.7 Gate as in 4.6.

## 8. Views and parity   [depends-on: 3, 7] [writes: packages/desktop-rs/crates/oc-app/src/views/, packages/desktop-rs/crates/oc-app/src/layout/, packages/desktop-rs/crates/oc-app/src/fixture/, packages/desktop-rs/crates/oc-app/src/root/, packages/desktop-rs/crates/oc-app/tests/, packages/desktop-rs/fixtures/states/, packages/desktop-rs/fixtures/reference/, packages/desktop-rs/crates/oc-app/src/app/mod.rs, packages/desktop-rs/crates/oc-client/src/catalog.rs, packages/desktop-rs/crates/oc-client/src/catalog_test.rs, packages/desktop-rs/crates/oc-state/src/selection/pickers.rs, packages/desktop-rs/crates/oc-state/src/selection/pickers_test.rs, packages/desktop-rs/crates/oc-state/src/fixture_test.rs, packages/desktop-rs/crates/oc-app/src/app/live_start.rs, packages/desktop-rs/tools/capture-reference.mjs] [parallel-safe: no]

- [ ] 8.1 Sidebar view: sections, project groups with collapse, Show more, error and incomplete rows, project menu with New session only.
- [ ] 8.2 Choose project popover, model popover (search, Favorites, Recent, provider sections, star), Thinking menu, agent menu (search, favorites), each a `popover` layout region; arrow keys, Ctrl+N/P, Enter, Escape returning focus; `mod+shift+m`.
- [ ] 8.3 Composer: model and Thinking triggers from state, agent trigger, Enter by the policy, no send while IME text is marked.
- [ ] 8.4 `--state` loads the version 2 format; move `fixtures/states/v2/*` into `fixtures/states/` and `fixtures/reference/v2/*` into `fixtures/reference/`, delete the first-format state files, and extend `tests/parity.rs` to every new state with the existing rules.
- [ ] 8.6 Popover bounds: the capture records each control's bounds; the composer triggers' positions and widths and the popover rows' heights follow those measurements and React's CSS until every popover region matches under the composer's device-pixel rule, which the parity test applies to `popover`.
- [ ] 8.5 Gate as in 2.4, plus `OC_DESKTOP_UI=1 cargo test -p oc-app --test parity` over all states.

## 9. Verification   [depends-on: 8] [writes: packages/desktop-rs/crates/oc-app/tests/] [parallel-safe: no]

- [ ] 9.1 `sidecar_it`: create a session with an agent and model and see the reply from that fixture model; switch the model, and the variant alone, and send; a Chat session gets its folder under the chats root; a settings PUT from the app keeps a project's unknown fields.
- [ ] 9.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `OC_DESKTOP_UI=1 cargo test -p oc-app --test parity`, `OC_DESKTOP_IT=1 cargo test -p oc-app --test sidecar_it`; from the repo root `passgate-engine rust` and `passgate-engine logging` on `packages/desktop-rs`; `openspec validate desktop-rs-pickers --strict`.
- [ ] 9.3 Manual check on this machine (the user runs it): pick a project, model, variant and agent and send with Enter; type Vietnamese with the IME and press Enter while composing; pick a model in Next and see it under Recent in the Electron app; collapse a project and see its color and icon unchanged in Electron.
