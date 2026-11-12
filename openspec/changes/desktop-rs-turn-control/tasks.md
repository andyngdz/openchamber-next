## 1. API fixtures   [depends-on: -] [writes: scripts/perf/fixture-provider.mjs, scripts/perf/DOCUMENTATION.md, packages/desktop-rs/tools/capture-api.mjs, packages/desktop-rs/fixtures/api/, packages/desktop-rs/fixtures/events/, packages/desktop-rs/crates/oc-client/src/calls_test.rs, packages/desktop-rs/crates/oc-client/src/event_test.rs, packages/desktop-rs/crates/oc-client/src/session_page_test.rs, packages/desktop-rs/crates/oc-client/src/types_test.rs, packages/desktop-rs/crates/oc-client/src/message_id_test.rs, packages/desktop-rs/crates/oc-state/src/message_events_test.rs, packages/desktop-rs/crates/oc-state/src/selection/catalog_test.rs, packages/desktop-rs/crates/oc-state/src/selection/pickers_test.rs] [parallel-safe: yes]

- [ ] 1.1 `scripts/perf/fixture-provider.mjs`: add models that call one tool and then answer: `bash` (a fixed `echo` command), `edit` (a fixed patch to a seeded file), `write` (new file content), `question` (three options, one `multiselect` with `custom`, one free-text field) and one that calls `webfetch`. Existing models and their output stay byte for byte the same.
- [ ] 1.2 `tools/capture-api.mjs`: the capture config sets `permission` to `ask` for `bash`, `edit` and `webfetch`, and seeds `followUpBehavior` unset and one session with an explicit auto-accept mode. Save the raw responses of every route in design.md's client table:
  - `GET /api/permission/request` and `GET /api/form` with a pending request, a pending form and a `global` form;
  - each reply, the cancel and a 404 reply;
  - `interrupt` during a running turn;
  - a steer prompt;
  - every message-queue route in design.md's client table: enqueue, reorder, take one, take all, remove one, and the 409 for an item in flight;
  - the auto-accept GET and PUT, and `GET /api/routing` (or its 404).
- [ ] 1.3 Save the parsed frames of `/api/openchamber/events` for the same run to `fixtures/events/control.json`: ready, a heartbeat, `message-queue.updated`, `permission-auto-accept.updated`, `left-for-user` when the server emits it. Save the matching `/api/event` frames: `permission.asked`/`replied`, `form.created`/`replied`/`cancelled`, `session.execution.interrupted`, the question tool part's error after an interrupt, and `session.inbox.*` for a plain prompt during a turn.
- [ ] 1.4 Run the capture twice and confirm the output is identical apart from the fields the README lists as volatile. Confirm no file holds a personal path. `bun run check:changed` passes.

## 2. Highlighter and theme   [depends-on: -] [writes: packages/desktop-rs/Cargo.toml, packages/desktop-rs/Cargo.lock, packages/desktop-rs/crates/oc-app/Cargo.toml, packages/desktop-rs/crates/oc-app/src/theme/, packages/desktop-rs/crates/oc-app/assets/themes/, packages/desktop-rs/tools/gen-theme-fixture.mjs, packages/desktop-rs/fixtures/theme/] [parallel-safe: yes]

- [ ] 2.1 Turn on gpui-kit's `tree-sitter-languages` feature. No other dependency changes, and `Cargo.lock` gains exactly the 37 crates design.md lists.
- [ ] 2.2 Copy `packages/ui/src/lib/theme/themes/openchamber-dark.json`, the Electron default dark theme (`DEFAULT_DARK_THEME_ID`, `themes/index.ts:16`), to `oc-app/assets/themes/openchamber-dark.json` and embed it with `include_str!` from the crate's own directory. Code holds no `../` path. In `oc-app/src/theme/`, port `TOKEN_RULES`, the dotted-name parent fallback, bold and italic, and the `diff.plus`/`diff.minus` roles of `packages/ui/src/lib/shiki/zedThemeFromAppTheme.ts`, together with `resolveSyntaxTokens`. Build a gpui-kit `HighlightTheme` from that JSON's `colors.syntax` and install it on `Theme::highlight_theme` inside `theme::apply`. The window palette stays as it is.
- [ ] 2.3 Tests:
  - The embedded copy is byte for byte the file in `packages/ui`. The test finds the repository by walking up from `CARGO_MANIFEST_DIR`, as the CLI lookup does.
  - For `openchamber-dark.json` and `openchamber-light.json`, every token name resolves to the same color, bold and italic as the TypeScript function. `tools/gen-theme-fixture.mjs` runs the TypeScript function from `packages/ui` and writes the expected table to `fixtures/theme/zed-tokens.json`, as `tools/gen-fuse-fixture.mjs` does for search.
  - A headless render of a Rust `Diff` paints a keyword with that theme's `syntax.base.keyword`.
- [ ] 2.4 Gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; from the repo root `passgate-engine rust --scope packages/desktop-rs`.

## 3. Electron references   [depends-on: 1] [writes: scripts/perf/fixture-provider.mjs, scripts/perf/DOCUMENTATION.md, packages/desktop-rs/tools/capture-reference.mjs, packages/desktop-rs/fixtures/seed.json, packages/desktop-rs/fixtures/reference/, packages/desktop-rs/fixtures/states/, packages/desktop-rs/crates/oc-app/src/views/messages.rs, packages/desktop-rs/crates/oc-app/src/views/messages_test.rs, packages/desktop-rs/crates/oc-app/src/views/turns.rs, packages/desktop-rs/crates/oc-app/src/views/turns_test.rs, packages/desktop-rs/crates/oc-app/src/views/turn_stats_test.rs, packages/desktop-rs/crates/oc-app/tests/parity.rs, packages/desktop-rs/crates/oc-app/tests/parity_support/] [parallel-safe: yes]

- [ ] 3.1 Seed and capture the states that design.md's Parity reference lists: `permission-bash`, `permission-edit`, `permission-write`, `permission-two`, `form-choice`, `form-multiselect`, `form-text`, `form-external`, `websearch-consent`, `recovered-question`, `queue-one`, `queue-three-expanded` and `busy-with-text`. Add the `dock` region rule to `REGIONS` with its comment. A state that needs a form OpenCode will not produce from the fixture models, such as `websearch.provider` or `external`, is seeded through the server's form route, and the capture script's header comment says so.
- [ ] 3.2 Write every new state as a state fixture that the same reducer replays, to `fixtures/states/turn/<name>.json`, and the references to `fixtures/reference/turn/`. Existing states stay where they are.
- [ ] 3.3 Run the capture twice and confirm identical JSON. Report any control with a missing or duplicate accessible name.
- [ ] 3.4 The capture runs with no window on the user's desktop: Electron on Chromium's Ozone `headless` backend, with no display variables passed to it. A run of the existing `empty` state matches `fixtures/reference/empty.json` in region bounds and in control keys, roles, names and order before any other change lands.
- [ ] 3.5 Every control records `icons`, `marks` and `texts` as design.md's Parity reference says. Recapture every existing state's reference with these fields. Their region bounds, control keys, roles, names and order must pass the existing comparison rules against the committed files, and control bounds may move with headless font metrics. Any other difference is reported before it is written.
- [ ] 3.6 Turn state fixtures carry the `ui` block for local view choices, as design.md says. Every dock control has an accessible name, and any control whose name falls back to its role is reported.

## 4. Client   [depends-on: 1] [writes: packages/desktop-rs/crates/oc-client/src/] [parallel-safe: no]

- [ ] 4.1 Types: `PermissionRequest`, `FormInfo`, the tagged `FormField` union with number-or-string bounds, `FormValue`, queue snapshots and items, the auto-accept snapshot, the routing state (`available`, `jevAvailable`, `heldPermissions`), and the decision and delivery enums. `followUpBehavior` in `settings.rs`, with the `queueModeEnabled` legacy mapping (`lib/settings/parsers.ts:421-428`) and the `queue` default. The `PermissionAsked` and `FormCreated` event payloads gain the full records, keeping the field names `oc-state` reads today, so the workspace compiles unchanged.
- [ ] 4.2 Every call in design.md's client table. The steer is a new method next to `send_prompt`, whose signature stays, so `oc-app` and its tests compile unchanged. A 404 from a reply or a cancel comes back as its own error variant.
- [ ] 4.3 `control_events()`: the `data:`-only SSE reader on `/api/openchamber/events` with the decoded types and `Other`, sharing the existing event stream's reader. Silence for 45 seconds is an error.
- [ ] 4.4 Tests: every unit 1 fixture decodes; `FormField` bounds accept numbers and the three strings; a request body matches its saved `.request.json`; an unknown control event decodes to `Other`; the silence timeout fires.
- [ ] 4.5 Gate as in 2.4.

## 5. Pending requests state   [depends-on: 4] [writes: packages/desktop-rs/crates/oc-state/src/, packages/desktop-rs/crates/oc-client/src/] [parallel-safe: no]

- [ ] 5.1 The `Requests` store: upsert and remove from events, list results replace one directory's entries including `global` forms, and a failed list keeps the old entries and marks the directory stale. Sidebar `PendingRequest` ids are derived from it.
- [ ] 5.2 Scope: `scoped_permissions` and `scoped_forms` with design.md's order and the `from_subagent` mark.
- [ ] 5.3 Auto-accept modes with revision, `effective_mode` through the lineage, and the cycle with safety present and missing. Held-back fresh requests, early `left-for-user` reports with their 100-entry limit, and the clearing on reply. Safety holds from routing and `routing.permission-held`, released on reply.
- [ ] 5.4 The local `Interrupted` mark on the trailing turn's open tool parts and on a loaded history that records an interruption. The `classifyQuestionToolError` port, and seen questions, outcomes and the offer rule with its 100-entry limit.
- [ ] 5.5 Form logic, pure: `initialFormValues`, `when` and `hidden` evaluation over earlier active answers, `missingRequiredKeys`, the answer builder, `serializeFormAnswerAsMarkdown`, web search consent detection, and the in-memory drafts with their 50-entry limit and field signature.
- [ ] 5.6 Tests for each rule above, from the unit 1 fixtures and hand-built inputs. Port the applicable cases from `formCardState`, `recoverable-questions`, `scoped-blocking-requests`, `permissionAutoAccept` and `permissionSummary` tests.
- [ ] 5.7 Gate as in 2.4.

## 6. Queue and send state   [depends-on: 4, 5] [writes: packages/desktop-rs/crates/oc-state/src/] [parallel-safe: no]

- [ ] 6.1 Queues by session with the per-session not-older revision rule, and a full `GET` replacing every session.
- [ ] 6.2 `followUpBehavior` from the settings document into the state.
- [ ] 6.3 `decide_send` with design.md's inputs and branches, the three ordered steps after it as a pure plan the app runs, the queue text join, and the Steer downgrade when idle.
- [ ] 6.4 The abort window: `arm_abort` and `confirm_abort` over 3000 ms, per session.
- [ ] 6.5 Tests: every branch of `decide_send`, ported from `ChatInput.keystroke.test.tsx` where it applies. Include the pending-permission, empty-composer, two-queued-items case, both `enterToSend` settings, the revision rule, and the abort window edges.
- [ ] 6.6 Gate as in 2.4.

## 7. Live wiring   [depends-on: 5, 6, 12] [writes: packages/desktop-rs/crates/oc-app/src/live/, packages/desktop-rs/crates/oc-app/src/app/, packages/desktop-rs/crates/oc-app/src/root/live.rs, packages/desktop-rs/crates/oc-app/src/root/requests.rs, packages/desktop-rs/crates/oc-app/src/root/updates.rs, packages/desktop-rs/crates/oc-app/tests/sidecar_it.rs, packages/desktop-rs/README.md] [parallel-safe: no]

- [ ] 7.1 The control stream task next to the session task, with backoff from 1 to 30 seconds. Every ready frame re-reads the queue, the auto-accept snapshot and the routing state. A server without the stream (404) logs once and turns the queue strip and the shield modes off.
- [ ] 7.2 Startup and an `/api/event` reconnect list permissions and forms for every directory with an open session or a loaded catalog.
- [ ] 7.3 A `Command` for every client call, with its failure returned as an `Update`. The dismiss-on-send runs in parallel with optimistic removal and a restore on non-404 failure. The send plan from 6.3 runs in order.
- [ ] 7.4 README: the control stream, what Enter does while a turn runs, and the highlighter feature with its binary cost and its 38 languages.
- [ ] 7.5 Gate as in 2.4, plus `OC_DESKTOP_IT=1 cargo test -p oc-app --test sidecar_it`.

## 8. Docks   [depends-on: 2, 7, 20] [writes: packages/desktop-rs/crates/oc-app/src/live/protocol.rs, packages/desktop-rs/crates/oc-app/src/live/execution.rs, packages/desktop-rs/crates/oc-app/src/icons/, packages/desktop-rs/crates/oc-app/src/views/dock/, packages/desktop-rs/crates/oc-app/src/views/mod.rs, packages/desktop-rs/crates/oc-app/src/root/mod.rs, packages/desktop-rs/crates/oc-app/src/root/keys.rs, packages/desktop-rs/crates/oc-app/src/layout/] [parallel-safe: no]

- [ ] 8.1 Permission dock: the header with `Stepper` dots and the subagent badge, the safety-hold banner, the summary and message, the per-tool bodies (read-only `Editor` for the command, `Diff` with a height cap for edit and write, the webfetch block, a collapsed `metadata` block), the buttons with the Always label rules, the three Alt shortcuts caught before the composer, and the in-flight, 404 and failure behavior.
- [ ] 8.2 Form dock on `QuestionnaireState`: every field type mapped as design.md says, `set_item_disabled` for `when` and `hidden`, the external `Link` flag, the header with count, dots, `more waiting` and the collapse toggle, Dismiss, Back, Next and Submit, the error line, and draft restore. Hidden while the permission scope is not empty.
- [ ] 8.3 Web search consent card and recovered question, as design.md says. A recovered submit sends through 7.3's send sequence, which switches `/agent` and `/model` to the linked message's picks before the prompt.
- [ ] 8.4 The `dock` layout region with its two roots: the floating panel above the composer that holds the permission or form dock, and the queue strip's row group inside the composer box, which unit 9 builds in `composer.rs`. The region's bounds are their union.
- [ ] 8.4a Every dock control registers `icons`, `marks` and `texts` (14.1), and the `dock` region is compared from the start.
- [ ] 8.5 Interaction tests through `gpui_kit::test`, generic over every dock kind. For each dock: open from state, buttons and shortcuts send the right command, in-flight disables, a 404 removes, and a failure keeps. For forms: navigation, every field type's answer, `when` skipping, external refusal and the consent buttons.
- [ ] 8.6 Gate as in 2.4.

## 9. Composer turn controls   [depends-on: 7, 8] [writes: packages/desktop-rs/crates/oc-app/src/live/protocol.rs, packages/desktop-rs/crates/oc-app/src/live/execution.rs, packages/desktop-rs/crates/oc-app/src/icons/, packages/desktop-rs/crates/oc-app/src/layout/, packages/desktop-rs/crates/oc-app/src/views/composer.rs, packages/desktop-rs/crates/oc-app/src/views/composer_controls.rs, packages/desktop-rs/crates/oc-app/src/views/composer_slots.rs, packages/desktop-rs/crates/oc-app/src/views/queue_strip.rs, packages/desktop-rs/crates/oc-app/src/views/mod.rs, packages/desktop-rs/crates/oc-app/src/root/keys.rs, packages/desktop-rs/crates/oc-app/src/root/mod.rs] [parallel-safe: no]

- [ ] 9.1 Stop instead of Send while the session is not idle, its tooltip with two `Kbd` Esc, and the follow-up button labeled `Queue message` or `Send message`. The button runs `decide_send` as the plain send key, so it applies `followUpBehavior` even when `enterToSend` is off, and shares 7.3's send plan.
- [ ] 9.2 Escape twice in the composer editor through the abort window, ignored with a popup open, during IME composition and while idle.
- [ ] 9.3 Enter and Ctrl+Enter while busy through `decide_send`, the composer clearing on enqueue, and no send while the first send is pending.
- [ ] 9.4 The shield's three icons and tooltips, the click cycle, and inert on a draft.
- [ ] 9.5 The queue strip: one row or `Collapsible` with `Badge` (at most 4 rows, then scrolling), the agent badge, edit, send now and remove, drag to reorder with `ListItem` drag and drop, and no edit or remove on an item that is sending.
- [ ] 9.6 Interaction tests through `gpui_kit::test`: Stop click and Escape Escape send one interrupt; a single Escape and a late second Escape send nothing; Enter while busy under `queue` and `steer` and with `enterToSend` off; shield cycling with safety present and missing; strip edit, send now, remove and a drag that reorders.
- [ ] 9.7 Gate as in 2.4.

## 10. Parity   [depends-on: 3, 8, 9] [writes: packages/desktop-rs/crates/oc-app/tests/parity.rs, packages/desktop-rs/crates/oc-app/tests/parity_support/, packages/desktop-rs/crates/oc-app/src/fixture/, packages/desktop-rs/tools/screenshots.sh] [parallel-safe: no]

- [ ] 10.1 `--state` loads every `fixtures/states/turn/` state, and `tools/screenshots.sh` renders them next to their references.
- [ ] 10.2 `tests/parity.rs` covers every new state. The `dock` region compares its controls in order, and its left, right and bottom edges by the composer's device-pixel rule, without its top edge or height. Every other region keeps its rules.
- [ ] 10.3 Gate as in 2.4, plus `cargo test -p oc-app --test parity` over all states, rendered headless.

## 12. Remix icons   [depends-on: -] [writes: packages/desktop-rs/tools/gen-remix-icons.mjs, packages/desktop-rs/crates/oc-app/assets/icons/, packages/desktop-rs/crates/oc-app/src/icons/, packages/desktop-rs/crates/oc-app/src/app/, packages/desktop-rs/crates/oc-app/src/views/, packages/desktop-rs/crates/oc-app/src/lib.rs, packages/desktop-rs/crates/oc-app/tests/] [parallel-safe: yes]

- [ ] 12.1 `tools/gen-remix-icons.mjs` writes every symbol of `packages/ui/src/components/icon/sprite.ts` as `assets/icons/remix/<name>.svg`, a 24x24 viewBox around the symbol's content.
- [ ] 12.2 `src/icons/`: `RemixIcon` with one name per file, implementing gpui-kit's `IconNamed`, and the app's `AssetSource`. It serves `icons/remix/` from the embedded files, answers the gpui-kit `icons/<name>.svg` paths in the substitution table with the Remix file React draws in the same place, and passes every other path to gpui-kit's `Assets`. It is registered for the window and the headless renderer.
- [ ] 12.3 Every view draws its icons through `RemixIcon`, with the icon React draws for the same control (`packages/ui/src/components/icon/icons.ts` names, the React component cited in each view's doc comment). `gpui_kit::assets::IconName` leaves `src/views/`.
- [ ] 12.4 Tests: the folder matches `sprite.ts` file for file (the test parses `sprite.ts`, found by walking up from `CARGO_MANIFEST_DIR`); every `RemixIcon` the views name has a file; a headless render draws a named icon; rendering every fixture state headless asks the asset source for no gpui-kit `icons/` path missing from the substitution table.
- [ ] 12.5 Gate as in 2.4, plus `node --check` on the script.

## 13. Theme palette   [depends-on: 2, 12] [writes: packages/desktop-rs/crates/oc-app/src/theme/, packages/desktop-rs/crates/oc-app/src/app/mod.rs, packages/desktop-rs/crates/oc-app/src/app/headless.rs, packages/desktop-rs/crates/oc-app/src/views/work_status.rs, packages/desktop-rs/crates/oc-app/src/views/composer.rs, packages/desktop-rs/tools/gen-theme-fixture.mjs, packages/desktop-rs/fixtures/theme/] [parallel-safe: no]

- [ ] 13.1 Build the window palette from the embedded `openchamber-dark.json` after `requireTheme`'s defaults, through `CSSVariableGenerator`'s mapping (`packages/ui/src/lib/theme/cssGenerator.ts:39-147`), for every token `Palette` holds plus the primary, status and agent colors views use for indicators. Remove the fixed `DARK` set.
- [ ] 13.2 Tests: `gen-theme-fixture.mjs` also writes the CSS variables `CSSVariableGenerator` produces for `openchamber-dark`, and every palette token equals its variable.
- [ ] 13.3 Gate as in 2.4.

## 14. Draw records and frame check   [depends-on: 3, 12, 13] [writes: packages/desktop-rs/crates/oc-client/src/, packages/desktop-rs/crates/oc-state/src/, packages/desktop-rs/crates/oc-app/assets/, packages/desktop-rs/crates/oc-app/src/icons/, packages/desktop-rs/tools/gen-remix-icons.mjs, packages/desktop-rs/crates/oc-app/src/layout/, packages/desktop-rs/crates/oc-app/src/theme/, packages/desktop-rs/crates/oc-app/src/views/, packages/desktop-rs/crates/oc-app/src/root/, packages/desktop-rs/crates/oc-app/tests/parity.rs, packages/desktop-rs/crates/oc-app/tests/parity_support/] [parallel-safe: no]

- [x] 14.1 The layout registry and `--dump-layout` record each control's `icons`, `marks` and `texts` with their bounds, as design.md says, from what the view draws. A gpui-kit component's own icons come from the substitution table, and its own marks and text colors from the component table in `src/layout/`. Every control registers them, the controls units 8 and 9 add included.
- [x] 14.2a The frame check of design.md runs over every fixture state headless and fails on any icon, text run or mark whose pixels do not show its reported color.
- [x] 14.2 `tests/parity.rs` compares `icons`, `marks` and `texts`, colors within 1 per channel, for every shown control of the regions in its comparison list, naming the control, the expected value and the drawn one on failure. The list holds the regions that already pass.
- [x] 14.2b Bundled provider logos, `providerLogoIds` candidates and the fallback icon, React's own SVG icons (`StopIcon`, `ContextRing`, `DiffViewIcon`), and project icons and colors from the settings, as design.md's Icons and palette says.
- [x] 14.4 Gate as in 2.4, plus `cargo test -p oc-app --test parity`, headless.

## 15. Telemetry   [depends-on: 14] [writes: packages/desktop-rs/crates/oc-client/src/, packages/desktop-rs/crates/oc-state/src/, packages/desktop-rs/crates/oc-app/src/views/turn_stats.rs, packages/desktop-rs/crates/oc-app/src/views/turn_stats_test.rs, packages/desktop-rs/crates/oc-app/src/views/work_status.rs, packages/desktop-rs/crates/oc-app/src/views/work_status_test.rs] [parallel-safe: no]

- [ ] 15.1 `oc-client` keeps the assistant message's `tokens`, `cost` and part timings, `oc-state` carries them, and the turn stats port `work-status/telemetry.ts` and `sections.ts`.
- [ ] 15.2 Tests ported from `telemetry.test.ts` and `sections.test.ts`, values from fixtures.
- [ ] 15.3 Gate as in 2.4.

## 16. Remote provider logos   [depends-on: 15] [writes: packages/desktop-rs/crates/oc-client/src/, packages/desktop-rs/crates/oc-app/src/icons/, packages/desktop-rs/crates/oc-app/src/live/, packages/desktop-rs/crates/oc-app/src/root/updates.rs] [parallel-safe: no]

- [ ] 16.1 A provider without a bundled logo loads the models.dev SVG through `oc-client`'s HTTP client on the live runtime, cached in memory, served through the asset source; a failed load falls back to the fallback icon. Fixture states use a logo stored in the fixture, so tests make no network call.
- [ ] 16.2 Tests: the candidate order, the cache, the fallback on failure.
- [ ] 16.3 Gate as in 2.4.

## 17. Markdown   [depends-on: 16] [writes: packages/desktop-rs/crates/oc-app/src/views/messages.rs, packages/desktop-rs/crates/oc-app/src/views/messages_test.rs, packages/desktop-rs/crates/oc-app/src/views/markdown/, packages/desktop-rs/crates/oc-app/src/views/mod.rs, packages/desktop-rs/crates/oc-app/src/theme/, packages/desktop-rs/crates/oc-state/src/] [parallel-safe: no]

- [ ] 17.1 Assistant text renders through gpui-kit `TextView` markdown with the tree-sitter highlighter, the `colors.markdown` tokens and React's markdown typography.
- [ ] 17.2 While a reply streams, only finished blocks show, as `commitStreamedText` decides; port its tests.
- [ ] 17.3 Gate as in 2.4.

## 18. Shell surfaces   [depends-on: 17] [writes: packages/desktop-rs/crates/oc-app/src/views/, packages/desktop-rs/crates/oc-app/src/layout/, packages/desktop-rs/crates/oc-app/src/theme/, packages/desktop-rs/crates/oc-app/tests/parity.rs, packages/desktop-rs/crates/oc-app/tests/parity_support/] [parallel-safe: no]

- [ ] 18.1 Sidebar, header, icon rail and the empty state match the references' `icons`, `marks` and `texts`, following React for each control; their regions join the comparison list.
- [ ] 18.2 Gate as in 14.4.

## 19. Composer and popup surfaces   [depends-on: 18] [writes: packages/desktop-rs/crates/oc-app/src/views/, packages/desktop-rs/crates/oc-app/src/layout/, packages/desktop-rs/crates/oc-app/src/theme/, packages/desktop-rs/crates/oc-app/tests/parity.rs, packages/desktop-rs/crates/oc-app/tests/parity_support/] [parallel-safe: no]

- [ ] 19.1 The composer and its triggers, and the project, model, agent and Thinking popups match: agent color dots, the selected check, the variant color, the agent icon, provider logos, the popover surface. Their regions join the comparison list.
- [ ] 19.2 Gate as in 14.4.

## 20. Chat and work-status surfaces   [depends-on: 19] [writes: packages/desktop-rs/crates/oc-app/src/views/, packages/desktop-rs/crates/oc-app/src/layout/, packages/desktop-rs/crates/oc-app/src/theme/, packages/desktop-rs/crates/oc-app/tests/parity.rs, packages/desktop-rs/crates/oc-app/tests/parity_support/] [parallel-safe: no]

- [ ] 20.1 The chat column (markdown included) and the work-status panel match. The comparison list is removed, so every region of every state is compared.
- [ ] 20.2 Gate as in 14.4.

## 11. Verification   [depends-on: 10, 20] [writes: packages/desktop-rs/crates/oc-app/tests/] [parallel-safe: no]

- [ ] 11.1 `sidecar_it` with the fixture provider: a bash permission answered once and the turn finishing; an edit denied; a question answered and the reply reaching the model; a message queued during a turn and sent by the server after it; Stop of a running turn and the session going idle.
- [ ] 11.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (parity included, headless), `OC_DESKTOP_IT=1 cargo test -p oc-app --test sidecar_it`; from the repo root `passgate-engine rust` and `passgate-engine logging` on `packages/desktop-rs`, `bun run check:changed`, and `openspec validate desktop-rs-turn-control --strict`.
- [ ] 11.3 Rebuild the release app (`cargo build --release -p oc-app`), then the user checks on this machine: answer a real bash permission and a question, stop a turn with Escape twice, queue a message during a turn and watch it send, and cycle the shield and see the mode in the Electron app.
