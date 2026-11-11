# Desktop-rs: projects, model and agent pickers, Enter to send

## Problem

The spike in `openspec/specs/desktop-rs-spike/spec.md` opens one directory given by `--directory`, shows a hard-coded model and agent label, and sends only from the Send button. The user cannot pick a project, a model or an agent, so the app is not usable day to day.

## Decision trace

Answers the user gave in this session, each settling one question:

- Scope: two changes. This one covers choosing among projects that already exist, the multi-project sidebar, the model picker, the agent picker and Enter to send. The Add project dialog is the next change.
- Settings: Next writes to the shared settings exactly as React does (`PUT /api/config/settings`), so Next and the Electron app see the same active project, recents and favorites.
- Enter while a turn runs: sending stays blocked and the draft is kept. Queue, steer and Stop belong to stage 2.
- Model picker: search, Favorites, Recent, one section per provider, star toggle, Thinking (variant) menu, arrow keys, Enter, Escape, `mod+shift+m`. Left out: drag reordering, Auto model, Run in parallel, Add provider, collapsing provider sections.
- Sidebar: every project with its sessions, the Chats section, collapse per project, open any session, New session for a project or for Chat. The project menu keeps its place and offers only New session. The branch and worktree picker stays inert.
- Settings that change what React shows (`hiddenModels`, `providerOrder`, the sidebar section settings, pinned sessions) are honored, because the user requires the layout to match and Next reads the same settings. This adds a read-only Pinned section, which the user accepted. Provider sections do not collapse: the composer picker keeps its collapse state in the Electron renderer's `localStorage` (`model-picker-collapsed-sections`, `packages/ui/src/stores/useModelPickerSectionsStore.ts:25-50`), not in `collapsedModelProviders`, so Next shows React's default, every section open, as the model picker answer chose.
- Popover bounds (decided during unit 8, option A): the `popover` region is compared on the device-pixel grid within 1 device pixel, the rule the composer already uses, because GPUI snaps edges to that grid and React's edges (e.g. 318 at scale 1.25) are off it. The composer triggers and the popover rows are fixed until the popovers match; the capture records each control's bounds as the measurement for that work. Comparing every control's bounds in the gate is a later change.
- Popups use gpui-kit components (decided 2026-11-11 after the user found that hand-built popovers neither closed on an outside press nor kept the wheel to themselves): `Popover` with `List` or `PopupMenu`. Their placement and size follow the components, not Base UI: `Popover::offset` is vertical only (`gpui-component-0.9.1/src/popover.rs:169`), so React's horizontal `alignOffset` cannot be expressed, and `PopupMenu` has its own row metrics. The user accepted that deviation, since the two frameworks behave differently. The parity gate therefore compares a popup's controls (keys and order, with `List`'s virtualized rows matched like the chat column's tool rows) but not the `popover` region's bounds; every other region keeps its rules.
- Interaction gate (same decision): bounds and order cannot catch behavior, so every interactive surface has interaction tests through gpui-kit's UI testing (`gpui_kit::test`): each popup kind opens from its trigger, closes on an outside press and on Escape, keeps the wheel inside, and picks with Enter; the composer's Enter follows `shouldSubmitEnter` through real key dispatch. The cases run over every popup kind generically, so a new popup is covered without a new test. They block the gate like parity.
- Approach: one client with the directory passed per call and one global event stream (below).
- Fuzzy search: `fuse-rust` 0.5.0 was measured against Fuse.js 7.1.0 on 184 typo queries over 25 picker-like strings with React's options; it agreed on the result order in 152 and on the result set in 180, and it has no `ignoreLocation` or `minMatchCharLength`. The Fuse.js algorithm is ported instead (see Package decisions).

## Goals

- The sidebar lists the projects from settings with their sessions, a Chats section and the In work section, ordered and limited as React does.
- Choose project popover on the empty state with search; picking a project makes it the active project.
- Model picker and Thinking menu in the composer, backed by the server's catalog for the draft's or session's directory.
- Agent picker in the composer with search and favorites.
- A new session carries the picked agent and model; an existing session is switched to a changed model or agent before the prompt.
- Enter sends, following the `enterToSend` setting, and never while an IME composition is open.
- Layout parity for every new surface, checked by the existing gate against new references.

## Non-Goals

- The Add project dialog, cloning, and `POST /api/openchamber/directory` (next change).
- Queue, steer, Stop and the follow-up button (stage 2).
- Drag reordering, Auto model, Run in parallel, Add provider, provider logo images (a placeholder icon of the logo's size and place is drawn). Collapsing provider sections: the composer picker's collapse state lives in renderer `localStorage`, so Next shows every section open, React's default; see Decision trace.
- Project menu items other than New session, the branch and worktree picker, worktrees, spaces, pinning and unpinning, custom sidebar sections (`section:<id>` entries in `sidebarSections`, which have their own title conditions).
- The settings UI; Next only reads and writes the keys listed under Settings.
- The `manual` project order, whose list lives in the Electron renderer's `localStorage` (`projects:<namespace>:manualOrder`, `packages/ui/src/stores/useProjectsStore.ts:90`) where Next cannot read it. Under `manual`, Next keeps the order of `projects` in settings.

## Approaches considered

1. One client, directory per call, one global event stream. Chosen. React does the same: scoped calls through `getScopedSdkClient(directory)` (`packages/ui/src/lib/opencode/client.ts:790`) and one unscoped event subscription. The server treats a stream with no `directory` query and no `x-opencode-directory` header as global (`packages/web/server/lib/opencode/proxy.js:840`), and every event names its directory in `location.directory`.
2. One client and one event stream per project. Rejected: N streams, N reconnect loops, and the global events would arrive N times.
3. One project at a time, switching the client's directory. Rejected: React shows every project at once.

## Design

### Client (`oc-client`)

- `Directory` newtype (absolute path string). `Client::new(port)` keeps no directory; every scoped call takes `&Directory` and sends `x-opencode-directory` (URI-encoded, as today) plus the `directory` or `location` query React sends for that route.
- `Client::events()` sends no directory header and no query, so the stream is global.
- New calls, shapes taken from captured fixtures:
  - `GET /api/fs/home` -> `{ home, chatsRoot, canonicalChatsRoot?, canonicalLegacyChatsRoot? }`.
  - `GET /api/session?limit=500[&cursor=<next>]` with no directory: the global session list, one page per call, `cursor.next` for the next page. Sessions are deduped by id, and loading stops when `cursor.next` is missing, equals the current cursor, or a page adds no new id (`packages/ui/src/stores/globalSessions.ts:51-120`).
  - `POST /api/fs/mkdir` `{ path }` for a Chat session's folder.
  - `GET /api/config/settings?surface=desktop` -> the flat settings document; `PUT` the same URL with a partial object, answered with the full document (`packages/web/server/lib/opencode/routes.js:97-106, 290-299`).
  - `GET /api/config` (OpenCode config for a directory): `model` and `default_agent`.
  - `GET /api/provider`, `GET /api/model?types=chat&types=image`, `GET /api/agent`, each for a directory.
  - `GET /api/session/:id` for the session record's `agent` and `model` (`{ providerID, id, variant? }`), which the list does not carry.
  - `POST /api/session/:id/model` `{ model: { providerID, id, variant? } }` and `POST /api/session/:id/agent` `{ agent }`, both 204.
- `POST /api/session` gains `agent` and `model` (`{ providerID, id, variant? }`, `variant` left out when none), as `packages/ui/src/lib/opencode/client.ts:1146-1165` sends.
- Model identity: a catalog model has `id` and `modelID`, which differ for derived entries (`packages/ui/src/lib/opencode/model.ts:102-108`). Everything Next stores or compares uses the catalog `id`: session refs (`id`), settings refs (their `modelID` field holds the catalog `id`), and the `"provider/model"` keys of `lastUsedEfforts`, `defaultModel` and `lastSelectedModel`.
- Settings types decode the keys Next uses:
  - Typed: `activeProjectId`, `favoriteModels`, `recentModels`, `hiddenModels` (`{ providerID, modelID }`), `favoriteAgents`, `recentAgents`, `providerOrder`, `lastUsedEfforts` (`"provider/model"` -> variant or `null`), `lastSelectedModel`, `defaultModel`, `defaultVariant`, `defaultAgent`, `enterToSend`, `enterToSendConfigured`, `sidebarProjectSortOrder`, `sidebarKeepWorkInPlace`, `sidebarShowChatsSection`, `sidebarShowWorkSection`, `sidebarShowPinnedSection`, `sidebarSections`, `sidebarHiddenSections`, `sessionWorkEnabled`.
  - `projects[]` entries are held as raw JSON objects with typed read accessors for `id`, `path`, `label`, `defaultAgent`, `defaultModel`, `defaultVariant`, `addedAt`, `lastOpenedAt`, `sidebarCollapsed`. React writes `projects` as a whole array (`useSessionProjectViewState.ts:85-90`, `useProjectsStore.ts:844-849`) and the server keeps the array it receives, so Next patches only the fields it changes on the raw objects and every other field (`color`, `icon`, `iconBackground` and the rest) survives the round-trip.

### State (`oc-state`)

- Sessions are held with their directory. Every event is routed by its `location.directory` and session id; a new session appears in the group its directory belongs to by the split below, and an event for a session no group shows changes nothing on screen.
- Projects come from settings. The global session list is split on the client as React does (`partitionSidebarSessions` and `isKnownActiveSessionDirectory`, `packages/ui/src/components/session/sidebar/list/sessionCollection.ts:41-75`; `splitGlobalSessionsByArchived`, `packages/ui/src/stores/globalSessions.ts:29-39`):
  - Archived sessions (`time.archived` set) show nowhere.
  - `/btw` forks (`openchamber.kind` `btw` with an original session id, `packages/ui/src/lib/sessionBtwMetadata.ts:55-57`) show nowhere.
  - Chats: the session's directory lies under one of the four chat roots: configured (`chatsRoot`, else the legacy root), canonical configured (`canonicalChatsRoot`, else configured), legacy (`<home>/.config/openchamber/chats`), canonical legacy (`canonicalLegacyChatsRoot`, else legacy) (`packages/ui/src/lib/chatDirectories.ts:16-19, 30-49, 70-79`).
  - A project: the directory equals the project path or lies under it.
  - Any other session is not shown.
- Sidebar membership, order and limits port React. Every sidebar setting below is read from the shared settings, so a user's Electron choices show the same way in Next:
  - Sections in the order of `sidebarSections` (default Chats, Pinned, In work, Projects, `packages/ui/src/lib/sidebarSections.ts:17`), minus `sidebarHiddenSections` and minus Chats, Pinned or In work when `sidebarShowChatsSection`, `sidebarShowPinnedSection` or `sidebarShowWorkSection` is false. Custom sections (`section:<id>`) are not rendered (Non-Goals).
  - Pinned (`SessionProjectCollection.tsx:541-549`): project sessions, not Chat and not archived, whose metadata has `openchamber.pinned.at` and no `openchamber.pinned.list` (`packages/ui/src/lib/sessionPins.ts:26-31`), in the same lifecycle order as a project group. They leave their project group. A pin filed in a list belongs to a custom section (Non-Goals) and stays in its project group. Pinning and unpinning are not offered.
  - In work (`sidebarWorks.ts:177-181`): sessions with metadata `openchamber.work.state` equal to `open` (`packages/ui/src/lib/sessionWorkMetadata.ts:28-33`), leaving out subagent sessions (`parentID` set), archived sessions and Chat sessions. The section exists only while `sessionWorkEnabled` is true (default true, `SessionProjectCollection.tsx:446`). Its sessions leave their project group unless `sidebarKeepWorkInPlace` is true (`sectionMembership.ts:56`).
  - Project order by `sidebarProjectSortOrder`: `a-z`, `z-a` (label or path, lowercased), `date-added`, and `recent` with its re-rank by newest session activity, as `packages/ui/src/components/session/sidebar/list/projectSort.ts:14-74`; `manual` keeps settings order (Non-Goals).
  - Sessions inside a group use `compareSessionsByLifecycleOrder` with the empty rank table the sidebar passes (`sessionSidebarRowModel.ts:352`): pinned first, then `time.updated` newest first, then `time.created`, then id (`packages/ui/src/sync/session-ordering.ts:205-228`). Rows do not move while a turn streams unless `time.updated` changes.
  - Show more: 7 rows per project and 7 more per click; Chats 3, then 7 more (`sessionSidebarRowModel.ts:312-316`).
  - Collapse per project from `sidebarCollapsed`; clicking a project header toggles it and does not change the active project.
- Catalog per directory: providers whose `activation` is not `disabled` and that have at least one enabled chat model; models with `enabled` and a type other than `image`, in the order the server returns (`useConfigStore.ts:2405-2442`). The model list leaves out `hiddenModels`, and orders provider sections by `providerOrder` then the server's order (`ModelPickerList.tsx:594-638`); every section is open. Agents whose mode is `primary`, `all` or missing and that are not hidden, sorted by lowercased name, then name.
- Selection per draft and per session: `{ agent, model: { providerID, id }, variant }`. An open session starts from its record's `agent` and `model` (`GET /api/session/:id`); a field the record lacks comes from the cascade below.
- Default cascades port `packages/ui/src/stores/useConfigStore.ts:436-581`:
  - Agent: project `defaultAgent`, settings `defaultAgent`, the OpenCode config's `default_agent` when it names a visible primary agent, `build`, the first primary agent, the first agent.
  - Model: project `defaultModel` (with project `defaultVariant`) or settings `defaultModel` (with settings `defaultVariant`); then the agent's pinned model; then the OpenCode config `model`; then `lastSelectedModel`; then `opencode/big-pickle`; then the first model not in `hiddenModels`; then `opencode/big-pickle` even if hidden; then the first model.
  - A configured default, an agent pin and `lastSelectedModel` are kept even when the catalog lacks that model, and their variant is then kept unchecked (`useConfigStore.ts:343-346, 373-376, 531-535, 549-552`). A variant is checked against the model's variants only when the catalog has the model.
  - Variant, when still unset: the agent's pin for that exact model, then `lastUsedEfforts`, kept only if the model has that variant.
- Picking an agent ports `setAgent` exactly as `useConfigStore.ts:3475-3620` does, including its per-session memory of the model and variant last used with each agent, so switching agents in an open session and back restores that agent's model. The variant order there is the agent's variant for the model, then the configured default variant, then the remembered effort.
- A draft's target is Chat or a project. A new draft starts on Chat (`session-ui-store.ts:882-887`). Picking a project sets the target, makes it the active project, sets its `lastOpenedAt` to now (`useProjectsStore.ts:844-849`), and re-applies the defaults for that project. Picking Chat leaves `activeProjectId` alone.
- Picker rules port `packages/ui/src/stores/useUIStore.ts` and `ModelControls.tsx`:
  - On a model pick: Recent gets it (dedupe, prepend, keep 5, `useUIStore.ts:3200-3211`), and `lastSelectedModel` becomes `provider/id` (`ModelControls.tsx:1641-1645`). Sending adds nothing to Recent.
  - On an agent pick: an agent already in `recentAgents` leaves the list unchanged; a new one is prepended and the list keeps 5 (`useUIStore.ts:3229-3243`).
  - Favorite model: toggle, new ones prepended (`:3028-3048`). Favorite agent: toggle, new ones appended (`:3219-3227`).
  - `lastUsedEfforts`: written on a variant pick and on a successful send, key `provider/id`, `null` for Default, oldest dropped past 512 (`:3268-3286`).
- Search ports `packages/ui/src/lib/search/fuzzySearch.ts`:
  - `rank_by_query` (`rankByQuery`, `:114-170`), including its Fuse.js fallback for one token of three or more characters. The agent list ranks name, display name and description (`ModelControls.tsx:666`); the project list ranks label and path (`DraftTargetSelectors.tsx:243`).
  - `matches_rank_query` (`matchesRankQuery`, `:177-187`), no fuzzy fallback, which keeps the model list's sections and order and filters on model name, model id and provider name (`ModelPickerList.tsx:604`).
  - Lowercasing, positions (`idx / 1000`) and the Fuse Bitap search work on UTF-16 code units, as JavaScript strings do, so non-ASCII labels such as Vietnamese project names rank the same as in React.
- Enter policy ports `shouldSubmitEnter` (`packages/ui/src/components/chat/composer/keyboardPolicy.ts:13-25`) with `isMobile` and `isDesktopExpanded` false; Ctrl and Meta count alike. Sending also needs a non-empty draft and an idle, connected session, as the Send button does today.

### App (`oc-app`)

- Startup loads settings and `/api/fs/home`, then the global session list page by page (`useGlobalSessionsStore.ts:790-815`). The first page is shown as soon as it arrives; the complete list replaces it. Each page request is tried 3 times, 500 ms apart (`globalSessions.ts:77-88`). When a later page still fails, the sessions already shown stay, the sidebar shows "Session list incomplete" with Retry, and nothing is presented as the full list.
- `--directory` is removed from the live app: projects come from settings, and the app opens on a new Chat draft, React's default target (`session-ui-store.ts:882-887`). The sidecar's working directory becomes the user's home directory. `--state` and `--dump-layout` stay. The local launcher (outside the repo) drops the flag.
- Catalog and agents load per directory when a draft or session needs them, cached per directory. After a reconnect the app refetches settings, the global session list, every loaded catalog and the open session's messages.
- Settings writes are partial `PUT`s, debounced 200 ms (`SETTINGS_DEBOUNCE_MS`, `packages/ui/src/lib/persistence.ts:399`); collapse writes debounce 700 ms (`useSessionProjectViewState.ts:79-103`). The response replaces the state's settings. A failed write keeps the local choice and logs it; the next write carries it again. Closing the window flushes pending writes before the sidecar stops, as React flushes on `pagehide` (`persistence.ts:511-521`). Next reads settings at startup and after each reconnect; it does not watch for the Electron app's writes in between.
- Popovers use gpui-component's `popover`, `list` and `menu`. Each popover is a layout region of its own, registered with its controls like the existing regions. A popover's width, row heights and paddings come from the CSS React applies to it, cited by file and line; a width React sizes to its text is measured with GPUI's text system in the same font. The model rows' names carry the context size React shows (`formatModelContextTokens`, `ModelPickerList.tsx:74-79, 1055-1058`), so the catalog keeps `limit.context`.
- Model trigger content: provider logo placeholder and the model's display name; a separate Thinking trigger shows `/` and the variant or `default`, hidden when the model has no variants (`ModelControls.tsx:2355-2412, 2658-2750`).
- Keys: `mod+shift+m` opens the model picker (React's default binding; a user override in Electron is not read); inside every popover arrows and Ctrl+N/P move, Enter picks, Escape closes and returns focus to the composer.
- Sending in a new session: Chat target creates `<chatsRoot>/YYYY-MM-DD/session-<uuid v4>` with `POST /api/fs/mkdir` (`packages/ui/src/lib/chatDirectories.ts:109-118`), then `POST /api/session` with `agent`, `model` and that location. In an existing session, `resolveSendSelection` is ported (`packages/ui/src/sync/session-selection.ts:21-50`):
  - The agent is compared with a fresh `GET /api/session/:id`; when it differs, `/agent` runs.
  - The model is compared with the session record in state in wire form: provider, `id`, and variant with no variant equal to `"default"` (`packages/ui/src/lib/opencode/model.ts:96-100`). A change of the variant alone is a change. Switching to Default sends `/model` without `variant`.
  - Then the prompt. A failure of any step keeps the draft and shows the error in the composer.
- IME: Enter while gpui's input has marked (composing) text goes to the input method and never sends.

### Parity reference

`tools/capture-reference.mjs` gains states, captured the same way as today:

- `projects`: two projects, two Chat sessions and one In work session seeded through settings and the API, so the sidebar shows Chats, In work and two project groups, one collapsed.
- `project-popover`, `model-popover`, `agent-menu`, `thinking-menu`: the empty state (the session state for `thinking-menu`) with that popup open, opened by clicking its trigger through CDP.
- The `popover` region is the open Base UI popup: the element with role `dialog` or `menu` that is inside none of the other regions (both pickers use `@base-ui/react`, `DraftTargetSelectors.tsx:21`, `components/ui/dropdown-menu.tsx:2`). Its bounds are that element's box; its controls are keyed by accessibility path like the other regions. The REGIONS list documents this rule next to the selectors.

The capture's own OpenCode config (not `scripts/perf`) registers the fixture provider twice under two provider ids with several models, one of them with variants, so sections, Recent and the Thinking menu have content.

State fixtures move to a second format, `fixtures/states/<name>.json`, and the four existing states are rewritten into it:

```json
{
  "version": 2,
  "home": { "home": "...", "chatsRoot": "..." },
  "settings": { "...": "the GET /api/config/settings document" },
  "directories": {
    "<path>": { "config": {}, "providers": [], "models": [], "agents": [], "sessions": [] }
  },
  "draft": { "target": "chat" },
  "steps": [],
  "open": "model-popover"
}
```

`settings` holds the settings document, each `directories` entry the raw responses for that directory, and `steps` the reducer steps of the first format.

`--state` loads every block through the same reducer actions the live app uses, so the dump draws what the live app would draw. `open` names the popup to show, or is absent. The capture tool writes the blocks from the responses it received, so the fixture and the reference come from one run. The parity test compares the new states with the same rules as the old ones; for the `popover` region it compares controls only (Decision trace). The capture also writes each control's bounds; the test does not compare them yet (Decision trace).

### Failure behavior

| Failure | Shown |
| --- | --- |
| Settings or `/api/fs/home` fails at startup | Sidebar error with Retry; no project list is guessed |
| The first session page fails | Sidebar error with Retry; no empty list is shown |
| A later session page fails | Sessions already shown stay; "Session list incomplete" with Retry |
| Catalog or agents fail for a directory | Model or agent trigger shows Unavailable; its popover shows the error with Retry |
| Settings write fails | Choice kept locally, logged, carried by the next write |
| `mkdir`, create, `/model`, `/agent` or prompt fails | Composer error, draft kept, Send enabled again |

## Package decisions

| Need | Decision | Reason |
| --- | --- | --- |
| Popovers, lists, menus, search input | reuse gpui-component (`popover`, `list`, `menu`, `input`) | already a dependency through gpui-kit |
| Keys | reuse GPUI actions and key bindings | built in |
| Bounded concurrent loads | reuse `futures::stream::buffer_unordered` | already a dependency |
| HTTP and JSON | reuse `reqwest`, `serde` | already dependencies |
| UUID v4 for Chat folders | add `uuid` 1 (feature `v4`) to `oc-app` | already in `Cargo.lock` 1.29.2 through gpui |
| Local date for Chat folders | add `chrono` 0.4 (feature `clock`) to `oc-app` | already in `Cargo.lock` 0.4.45 through gpui |
| Token ranking | custom port of `rankByQuery`, about 60 lines | React's own rules, no crate matches them |
| Fuse.js fallback | custom port of Fuse.js 7.1.0 Bitap search with React's options (`threshold` 0.35, `ignoreLocation`, `distance` 100, `minMatchCharLength` 2, `includeScore`) and Fuse's defaults that matter (`isCaseSensitive` false, 32-unit pattern chunks, field-length norm, results sorted by score then index), about 150 lines in `oc-state` | `fuse-rust` 0.5.0 measured 152/184 same order, 180/184 same set, and lacks the options; `nucleo-matcher` and `fuzzy-matcher` use other scoring |

## Testing

- Fixtures: `tools/capture-api.mjs` captures every new endpoint (settings GET and PUT, fs home, fs mkdir, config, provider, model, agent, session create with agent and model, session model and agent switch).
- Fuse fixture: a Node script runs Fuse.js 7.1.0 from `packages/ui/node_modules` over a fixed corpus, which includes non-ASCII strings, and a fixed query set, and writes `fixtures/search/fuse.json`. The Rust port must return the same indices in the same order. The field-length norm (`1 / Math.pow(numTokens, 0.5)` rounded to 3 decimals, `fuse.mjs:298-301`) is ported with `sqrt`, which is correctly rounded in both languages, so the norm is identical. Scores must then be equal except for the one remaining `pow` call (`score ^ norm`): every other step is IEEE-754 double arithmetic that Rust reproduces exactly, while V8's `pow` (fdlibm) and Rust's `powf` (the platform libm) are each within 1 ULP of the exact result. So two scores may differ by at most 2 ULP: `|a - b| <= 2 * f64::EPSILON * max(|a|, |b|)`. The bound follows from that one call, not from the data.
- Unit tests: client decoding of every new fixture; event routing by directory; sidebar order, limits and In work membership; both default cascades with each step taken; recents, favorites and `lastUsedEfforts` rules; `rank_by_query` cases from `packages/ui/src/lib/search/fuzzySearch.test.ts` where present; the Enter policy cases from `keyboardPolicy.test.ts:25-52` that apply to the desktop composer.
- Parity: `cargo test -p oc-app --test parity` over the old and the new states. `--dump-layout` and `--screenshot` render through GPUI's headless context (real text system, headless wgpu renderer), so no gate opens a window on the user's desktop, and parity runs inside `cargo test --workspace`.
- Integration (`OC_DESKTOP_IT=1`): create a session with an agent and model, switch the model, send, and see the reply come from the picked fixture model; a Chat session gets its folder under `chatsRoot`.
- Manual: type Vietnamese with the IME and press Enter while composing; pick a model in Next and see it under Recent in the Electron app.

## Risks

- Next and the Electron app write the same settings. Both send partial `PUT`s that the server merges, so a key one app does not send is never touched by it. A whole-array key (`projects`, `favoriteModels`, `recentModels`) written from a stale copy overwrites the other app's newer change to that key, as two Electron windows can today; Next narrows the window by re-reading settings after each reconnect.
- gpui-component's popover may not place itself as Radix does; parity on the new states shows it, and the placement is then computed in `oc-app`.
- GPUI 0.3.12 does not instance a variable font by weight: `FontKey` (`gpui-pre-wgpu` `cosmic_text_system.rs:27-31`) has no weight and a variable family has one face, so 500-weight text such as the composer trigger labels shapes as 400, 0.4-3 px narrower than Chromium. Widths are measured at runtime from that shaping, never fixed; the trigger boxes are therefore up to about 2.4 px narrower than React until GPUI applies the weight axis or a static Medium face is bundled.
- The popup x edges pass today partly because the weight error moves the right-aligned triggers' left edges more than the right edge the popups are anchored to. Trigger width error / left-edge error / right-edge error: model 2.39 / +3.79 / +1.40 px; Thinking 0.49 / +1.40 / +0.91 px; agent 0.76 / +0.91 / +0.15 px. The model popup takes that +1.40 px (left and right edges 512.0 vs 511.0 and 892.0 vs 891.0 CSS px), which is +1 device pixel at scale 1.25 and passes at the test's limit. The Thinking popup's own text-measured width is 0.09 px narrower, and snapping leaves left/right +0.2/+0.11 px (703.2 vs 703.0, 930.4 vs 930.29); the agent popup's edges are snapped to −0.4 px (713.6 vs 714.0, 993.6 vs 994.0). The project popup now anchors to the trigger button itself, so no offset compensates anything, and its x/right edges are −0.4 px (317.6 vs 318.0, 605.6 vs 606.0). Per edge (actual − reference, CSS px): project x −0.4, y −0.4, right −0.4, bottom +0.2; model +1.0, −0.2, +1.0, 0.0; agent −0.4, −0.2, −0.4, −0.2; Thinking +0.2, +0.2, +0.11, +0.6. Heights: project 135.2 vs 134.6, model 392.8 vs 392.6, agent 100.8 = 100.8, Thinking 143.2 vs 142.8, from row-line rounding and the 1 px borders. If GPUI later applies the `wght` axis, these popover edges will move by these amounts; that is the expected correction, not a regression.
- The model order inside a provider comes from OpenCode and may change between versions; the fixtures pin it for the tests.
