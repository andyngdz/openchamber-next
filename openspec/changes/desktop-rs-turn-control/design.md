# Desktop-rs: permissions, forms, Stop and the message queue

## Problem

The Rust + GPUI app (`packages/desktop-rs`) keeps a pending permission or form only as a `PendingRequest` id on the sidebar row (`oc-state/src/session_entry.rs:23-28`, `oc-state/src/events.rs:50-60`). Nothing on screen answers it, the client has no interrupt call, and Enter does nothing while a turn runs (`openspec/specs/desktop-rs-selection/spec.md`, *Enter to send*). An agent that asks for a permission or asks a question leaves its session waiting forever, and a turn that goes wrong cannot be stopped.

## Decision trace

The user's answers in the planning session:

- Next stage: Stop, permissions and forms, picked over markdown rendering.
- Sending while a request is pending: as Electron, in this change. Sending dismisses the session tree's permissions and forms, and a send during a running turn goes through the server's message queue or steering. This brings the message queue into the change.
- Auto-accept: the composer's shield (`views/composer_slots.rs:39`, inert today) becomes the real control.
- Special forms: both the web search consent card and recovered questions, as Electron.
- Syntax highlighting: in this change. After measuring both gpui-kit backends (see Package decisions), the user picked tree-sitter.
- Architecture: approach A below.
- A form's free-text field uses `questionnaire`'s single-line input. Shift+Enter cannot insert a newline in a form answer, which React's `AutoGrowTextarea` allows (`packages/ui/src/components/chat/FormFieldControl.tsx:291-298`). Enter still advances, as in React.
- Icons and indicators (added after implementation started): the user found the Rust app missing icons and indicators React shows, such as the agent color dots, the selected check mark, the orange variant text, the colored agent and project icons, and different composer, send and chip icons. The parity gate compared only role, name, order and bounds, so it could not see them. The user chose to fix this in this change. The capture, the dump and the parity test now record and compare each control's icons and color indicators, and the existing surfaces are fixed until the gate passes. This adds the Remix icon set and the Electron theme's palette to the change.
- Markdown (added after implementation started): the parity gate compares the color of every text run, and React renders the agent's reply as markdown (698 colored runs in the `session` reference). The user chose to build markdown rendering in this change rather than exempt the transcript text from the comparison.
- Standing decisions from earlier sessions that apply here: behavior follows the Electron app and React is the spec for clicks, keys and server writes. UI is built from gpui-kit components, and a component's own geometry may differ from React's. The parity gate compares popup controls but not popup bounds, and behavior is gated by headless interaction tests through `gpui_kit::test`. No check opens a visible window. Default to Electron's full behavior.

## Goals

- Answer every pending permission of the open session and its descendants from a dock above the composer, with React's preview per tool, buttons, shortcuts and ordering.
- Answer every form, one field per step, with every `FormField` type, the web search consent card and recovered questions.
- Stop a running turn from the Stop button or Escape twice.
- Cycle the session's auto-accept mode and hide requests the server is answering.
- Send while a turn runs by queue, steer or plain prompt as React decides, dismiss the tree's pending requests on send, and show and edit the server's queue.
- Highlight the bash command and the diffs with the theme's syntax colors.
- Draw the icons React draws (the Remix set in `packages/ui/src/components/icon/sprite.ts`) and the window palette of the Electron default theme. Make the parity gate fail on a missing or wrong icon, color dot or text color on every surface, the existing ones included.

## Non-Goals

- Attachments, `@` mentions, slash commands, goals and context chips in the composer or in queued items. Queued items carry text and the send config only.
- Customizable shortcuts. The defaults from `packages/ui/src/lib/shortcuts/config.ts` are fixed.
- Settings screens. `followUpBehavior` and the auto-accept default are read from the settings document, not edited.
- BTW composers and scheduled-task queue items. Scheduled items show without edit and send now, as React does, but nothing creates them.
- The "Show details" JSON of unknown tools beyond a collapsed block of the request's `metadata`.
- Prompt-history search, which hides the docks in React (`ChatInput.tsx:5477-5482`). The Rust composer has no prompt history yet.

## Approaches considered

- **A, picked: full records from events plus a second stream.** The reducer keeps whole `PermissionRequest` and `FormInfo` records from `permission.asked` and `form.created` and seeds them from the list routes at startup and after a reconnect. The queue and auto-accept state come from `/api/openchamber/events` snapshots, re-read on every ready frame. This is how React syncs (`packages/ui/src/sync/event-reducer.ts:697-751`, `sync/message-queue-sync.ts:26-35`, `sync/permission-mode-sync.ts:14-28`) and keeps the reducer pure and fixture-tested.
- B: ids only, fetching the details when the dock opens. The dock waits on a request and must reconcile events that arrive during the fetch.
- C: poll the queue and the pending lists. It needs no second stream, but its latency equals the poll period, and it breaks the repository rule that live state comes from live channels.

## Design

### Client (`oc-client`)

New calls. `Directory` is the existing newtype, and every scoped call takes it:

| Call | Route | Notes |
| --- | --- | --- |
| `permissions(dir)` | `GET /api/permission/request?location[directory]=` | `{ location, data: PermissionRequest[] }` |
| `reply_permission(session, id, decision)` | `POST /api/session/:s/permission/:id/reply` | body `{ decision }`, `once`/`always`/`reject` |
| `forms(dir)` | `GET /api/form?location[directory]=` | `{ location, data: FormInfo[] }`, includes `sessionID: "global"` forms |
| `reply_form(session, id, answer)` | `POST /api/session/:s/form/:id/reply` | body `{ answer }` |
| `cancel_form(session, id)` | `DELETE /api/session/:s/form/:id` | |
| `interrupt(session, dir)` | `POST /api/session/:s/interrupt` | no body |
| `prompt(..., delivery)` | existing prompt call | adds optional `delivery: "steer"` |
| `message_queue()` | `GET /api/message-queue` | `{ revision, sessions }` |
| `enqueue(session, dir, item)` | `POST /api/message-queue/sessions/:s/items` | body `{ directory, item: { content, text, sendConfig } }` |
| `take_all(session)` / `take_item(session, item)` | `POST .../take`, `POST .../items/:i/take` | |
| `reorder(session, ids)` | `PUT .../order` | complete permutation |
| `remove_item(session, item)` | `DELETE .../items/:i` | |
| `auto_accept()` / `set_auto_accept(session, dir, mode)` | `GET /api/permission-auto-accept`, `PUT .../sessions/:s` | body `{ mode, enabled: mode != ask, directory }`, as `stores/permissionStore.ts:57-65` |
| `routing()` | `GET /api/routing` | `available`, `jevAvailable` and `heldPermissions`; 404 reads as unavailable (`lib/routing/routingApi.ts:119-121`) |

A 404 from `reply_permission`, `reply_form` or `cancel_form` is returned as its own error variant, because the caller treats it as already settled.

Types: `PermissionRequest`, `FormInfo` and the `FormField` union exactly as the OpenCode contract defines them (`node_modules/@opencode/client/.../generated/types.d.ts:1104-1118, 3687-3693, 1519`), with `#[serde(tag = "type")]`. The bound fields `min`/`max` accept a number or the strings `"Infinity"`, `"-Infinity"` and `"NaN"`. Queue snapshots follow `packages/web/server/lib/message-queue/runtime.js:399-415`: `{ revision, session: { sessionId, directory, items, sendingId } }`, with items as `toPublicItem` (`runtime.js:191-215`) builds them. The strip reads `id`, `text`, `content`, `sendConfig.agent` and `scheduledTask`. The other item fields are kept but not shown. Unknown fields are ignored.

Control stream: `control_events()` opens `GET /api/openchamber/events`. Frames are `data:` lines only, each a JSON `{ type, properties }` (`packages/web/server/lib/notifications/emitter-runtime.js:26-28`). Decoded types: `openchamber:event-stream-ready`, `openchamber:heartbeat`, `openchamber:message-queue.updated`, `openchamber:permission-auto-accept.updated`, `openchamber:permission-auto-accept.left-for-user`, `openchamber:routing.updated`, `openchamber:routing.permission-held` (`{ permissionId, sessionId, directory, score, kind }`, `lib/openchamberEvents.ts:217-224`). Every other type becomes `Other` and is dropped. The stream uses the same auth as `/api/event`. Silence for 45 seconds counts as a drop, as `lib/openchamberEvents.ts:473-499` does.

### State (`oc-state`)

- **Pending requests.** `PendingRequest` keeps its ids for the sidebar. A new `Requests` store holds `PermissionRequest` and `FormInfo` by session id, sorted by `id` (`event-reducer.ts:700-707`). `permission.asked` and `form.created` upsert, `permission.replied`, `form.replied` and `form.cancelled` remove. A list result replaces exactly that directory's entries, sessions and `global` forms included. A failed list leaves the previous entries in place and marks the directory stale. It never empties it.
- **Scope.** `scoped_permissions(open)` returns the open session's requests, then its descendants in breadth-first order, each session's sorted by `id` (`sync/scoped-blocking-requests.ts:5-61`). The parent links come from the global session list. `scoped_forms(open)` is the first form of that tree, then the `global` forms of the open session's directory, plus a count of the rest (`FormDock.tsx:67-71,134`). A request carries a `from_subagent` mark only when its session's parent is the open session (`usePermissionResponse.ts:40-48`).
- **Auto-accept.** It holds `modes` by session id and the snapshot `revision`. A newer revision replaces the map. `effective_mode(session)` walks to the nearest explicit ancestor, and unknown lineage is `ask` (`packages/web/server/lib/permission-auto-accept/runtime.js:224-255`, `stores/utils/permissionAutoAccept.ts:80-115`). With `safety` unavailable (`available && jevAvailable` false), a `safety` session reads as `ask`, and the cycle is ask → auto → ask. Otherwise it is ask → safety → auto → ask (`permissionAutoAccept.ts:16-40`).
- **Held-back requests.** Only a fresh `permission.asked` is held back, and only when the session's effective mode at that moment answers without the user (`safety` or `auto`). It waits outside the store until `left-for-user` names it, which adds it, or `permission.replied` drops it. A reply clears both the waiting entry and any early report for that request (`forgetAutoAnswerWait`, `sync/sync-context.tsx:1986-1990`). A `left-for-user` that arrives before its `permission.asked` is remembered, at most 100 entries, and that request is shown at once. Requests loaded from a list are always shown, and a mode switch never hides a request already shown (`sync/sync-context.tsx:1929-1990`).
- **Safety holds.** `held` by permission id, seeded from `GET /api/routing`'s `heldPermissions`, added by `routing.permission-held`, and released by `permission.replied` (`stores/useRoutingStore.ts:77-115`, `sync/sync-context.tsx:2377`).
- **Recovered questions.** A form is seen when it has `metadata.kind == "question"`, `sessionID != "global"` and a `metadata.tool { messageID, id }`. It is offered after it leaves the pending store if all of these hold: the linked tool part exists in the loaded messages, its state is `error`, and its error classifies as a turn death by `classifyQuestionToolError` (`sync/recoverable-questions.ts:52-58`):
  - with `errorSource: "local"`, `Interrupted` is a turn death and anything else is unresolved;
  - otherwise an `errorType` other than `aborted` dismisses the question;
  - `Tool execution interrupted` is a turn death, `Interaction cancelled because the location shut down` is a shutdown, and anything else is unresolved.

  A completed part counts as answered. The local mark comes from the reducer, as `interruptedTurnToolParts` does (`sync/sync-context.tsx:2700-2770`). When `session.execution.interrupted` (not the shutdown reason) or a failure ends a turn, every pending or running tool part of the trailing assistant message becomes `error` with `Interrupted` and `errorSource: "local"`. A loaded history whose turn records an interruption gets the same mark on an `Interrupted` part that has no source or type. A later authoritative tool transition replaces the mark. Submit and Dismiss record an outcome, and the seen map keeps at most 100 entries. Nothing persists.
- **Queue.** `queues` by session id from snapshots, each with the revision it was applied at. A snapshot is skipped only when its revision is older than that session's held one, so an action's response and its broadcast at the same revision both apply (`stores/messageQueueStore.ts:524-552`). A full `GET` replaces every session at its revision.
- **Send decision.** `decide_send(input)` is pure. Its inputs are:
  - the composer text;
  - whether the session is idle;
  - `followUpBehavior`;
  - whether the key is the plain send key or Ctrl/Cmd+Enter, after the Enter policy of `shouldSubmitEnter`;
  - whether the draft's first send is pending;
  - whether the queue holds items.

  It returns `Prompt`, `Steer`, `Enqueue` or `None`, following `ChatInput.tsx:2101-2111, 2883-2896, 3185-3213`:
  - First send pending: `None`.
  - Busy with text and the plain send key: `Enqueue` under `queue` (the default, `stores/messageQueueStore.ts:19`), `Steer` under `steer`.
  - Everything else is `Prompt`: idle with text, any Ctrl/Cmd+Enter, and an empty composer with queued items, busy or idle. With `enterToSend` false, Ctrl/Cmd+Enter is the only send key, so a busy session neither enqueues nor steers from the keyboard, as in React. The follow-up button above Stop still applies `followUpBehavior`.

  A `Prompt` or `Steer` from the composer runs in this order:
  1. Dismiss the tree's pending requests (`ChatInput.tsx:2296-2317`).
  2. When anything was dismissed and the decision is not `Steer`, stop sending. The composer text is enqueued, because a plain prompt would race the turn that just got unblocked. An empty composer enqueues nothing (`ChatInput.tsx:1794`), and the queue is left for the server to send.
  3. Otherwise take the whole queue (`POST .../take`) and send the queued texts, oldest first, then the composer text, joined by `\n\n` (`ChatInput.tsx:2445-2460`, `composer/submit/buildOutgoingMessage.ts:155-180`). A failed take shows the error and sends nothing. `Enqueue` dismisses nothing (`ChatInput.tsx:3199-3205`). `Steer` is downgraded to a plain prompt when the session is idle by the time it is sent (`ChatInput.tsx:2111`). The legacy `queueModeEnabled` maps onto `followUpBehavior` as `lib/settings/parsers.ts:421-428` does.
- **Abort prompt.** `arm_abort(session, now)` and `confirm_abort(session, now) -> bool` keep the 3000 ms window of `hooks/useKeyboardShortcuts.ts:452-499`.

### Highlighting and theme

`gpui-kit` gets the `tree-sitter-languages` feature. A port of `packages/ui/src/lib/shiki/zedThemeFromAppTheme.ts` (`TOKEN_RULES`, the parent fallback and the `diff.plus`/`diff.minus` roles) builds a gpui-kit `HighlightTheme` from the `colors.syntax` of `openchamber-dark.json`, the Electron default dark theme (`packages/ui/src/lib/theme/themes/index.ts:16`). The app has no theme switching yet. The window palette comes from the same JSON, as the Icons and palette section says. The JSON is copied into the crate and embedded, and a test keeps the copy equal to the source. `theme::apply` installs the result on `Theme::highlight_theme`. `Diff` picks the language from the file path, and the bash preview passes `bash`. A language without a grammar shows unhighlighted.

### Icons and palette

- **Remix icons.** `tools/gen-remix-icons.mjs` writes every symbol of `sprite.ts` (337 today) as `oc-app/assets/icons/remix/<name>.svg`. A test checks that the folder matches `sprite.ts`, file for file. `RemixIcon` names each one and implements gpui-kit's `IconNamed` (`gpui-component/src/icon.rs:30-148`). The app's `AssetSource` serves these files. It also answers the `icons/<name>.svg` paths that gpui-kit components load for their own icons, such as the search icon of `List`, the check of `PopupMenu` and `Checkbox`, and the arrows of `Select` and `Diff`, with the Remix file React draws in the same place. That substitution comes from one named table in `src/icons/`, and every other path goes to gpui-kit's `Assets`. A test renders every state headless and records each `icons/` path the asset source is asked for. It fails when a gpui-kit path has no Remix entry in the table, so no gpui-kit icon reaches the screen. Views name icons only through `RemixIcon`, with the same names React's `Icon` uses, and `gpui_kit::assets::IconName` leaves the views. The test in `views/icons_test.rs` checks that every name the views use has a file.
- **Provider logos.** `ProviderLogo` (`packages/ui/src/components/ui/ProviderLogo.tsx`, `hooks/useProviderLogo.ts`) is ported: the logos bundled in `packages/ui/src/assets/provider-logos/` are copied into the crate like the Remix icons, the candidate ids follow `providerLogoIds`, a provider without a bundled logo loads `https://models.dev/logos/<id>.svg` as React does, and a failed load falls back to `getProviderLogoFallbackIcon`. The dump reports a logo as `img:<id>`.
- **React's own SVGs.** Icons React draws as inline SVG components rather than sprite symbols (`StopIcon`, `ContextRing`, `DiffViewIcon`'s base) are copied from their components into `assets/icons/custom/`, drawn through the same `IconNamed` path, and reported as `other`.
- **Project icons.** A project's configured `icon` and `color` in the settings (`Project.icon`, `Project.color`) travel from `oc-client` through the sidebar state to the view, which draws the Remix icon and color React draws for it.
- **Palette.** The window palette comes from `openchamber-dark.json` through the variable mapping of `CSSVariableGenerator` (`packages/ui/src/lib/theme/cssGenerator.ts:39-147`), after `requireTheme` fills the defaults as the highlight port does, for every token the `Palette` holds and every token a view needs for an indicator: the primary, status and agent colors. The fixed `DARK` set copied from `design-system.css` goes away, because Electron overrides those variables at runtime with the theme's values.
- **What the dump records.** The views report, for each control they register, new controls included, the same three fields the capture records (Parity reference). A gpui-kit component's own icon is reported by the view that uses the component, from the same table that substitutes it. A component's own marks and text colors, such as `Stepper` dots or a `List` row label, are reported from the theme field the component paints with, through a second table in `src/layout/` that cites the gpui-kit source line for each entry. The dump also records the bounds of every icon, mark and text run.
- **Frame check.** The dump's claims are checked against the painted frame, so the gate cannot pass while the screen differs. A test renders every fixture state headless at 2x, reads the frame, and requires two things of what the dump reports:
  - every icon and text run has at least one pixel inside its bounds equal to its expected color within 1 per channel. At 2x, a glyph or icon stroke covers whole pixels, so such a pixel exists whenever the color is right;
  - every mark's center pixel equals its expected color within 1 per channel.

  The expected color is the claimed `#rrggbbaa` composited over the backdrop with source-over. For a draw with bounds of its own, the backdrop is the most frequent color of the one-pixel ring just outside them, which is the surface it sits on, also for a filled icon that covers most of its box. An opaque claim composites to itself, so translucent text such as `text-foreground/80` and the `opacity-60` hints is checked by the same rule. An icon or text run whose own bounds the view does not know, such as one inside a gpui-kit component, uses the control's bounds. Its backdrop is then the most frequent color inside the control, the control's own surface, because the ring outside a filled button is the page. There a same-colored icon or border can also satisfy the check, which makes it weaker, and the design accepts that.

  Only pixels the screen shows are read. A draw is clipped to its region's bounds, so a list row scrolled out of the popup is not checked, and a draw outside the popover region that the open popup overlaps is skipped. The check's own proof: painting the agent icon in the label color while the dump keeps the agent color fails it in every state.

### Markdown, telemetry and remote logos

- **Markdown.** Assistant text renders as markdown through gpui-kit's `TextView` (`gpui-component/src/text/`), with fenced code highlighted by this change's tree-sitter highlighter. Colors come from the theme's `colors.markdown` tokens (`link`, `inlineCode`, `inlineCodeBackground`, `blockquote`, `blockquoteBorder`, `listMarker`, `bold`, `italic`) through `CSSVariableGenerator` and the typography of React's `MarkdownRenderer` (`packages/ui/src/components/chat/MarkdownRenderer*.tsx`, `markdown/`). While a reply streams, only finished blocks are shown, as `commitStreamedText` decides (`packages/ui/src/components/chat/lib/streamTextCommit.ts`).
- **Telemetry.** `oc-client` keeps the assistant message's `tokens`, `cost` and part timings, `oc-state` carries them, and the work-status turn stats port `packages/ui/src/components/chat/work-status/telemetry.ts` and `sections.ts`, so the panel shows React's figures.
- **Remote provider logos.** A provider without a bundled logo loads `https://models.dev/logos/<id>.svg` through `oc-client`'s existing HTTP client on the live runtime, caches the bytes in memory, and serves them through the app's asset source, where GPUI renders the SVG. A failed load falls back to `getProviderLogoFallbackIcon`, as React does.
- **Rollout of the comparison.** The parity test keeps a list of regions whose `icons`, `marks` and `texts` it compares. A unit that fixes a region adds it, so every commit stays green, and the last surface unit removes the list, after which every region is compared. The frame check covers every region from the start.

### App (`oc-app`)

- **Live.** A second task runs the control stream next to the session task, with backoff from 1 second to 30 seconds. Every ready frame re-reads `GET /api/message-queue`, `GET /api/permission-auto-accept` and `GET /api/routing`. A reconnect of `/api/event` also re-lists permissions and forms for every directory that has an open session or a loaded catalog. New `Command`s cover each call in the client table. Their failures return as `Update`s, so the state can restore an optimistic change.
- **Permission dock.** It floats above the composer and shows nothing when the scope is empty. The header has a warning icon, `Permission required`, `{current} of {total}` with a gpui-kit `Stepper` of clickable dots, the tool's name and icon, and `from a subagent` when the mark applies. A request the safety net holds opens with `The safety net held this action for you to decide`, followed by ` · {kind}` when the hold names one (`PermissionCard.tsx:68-74, 299-309`, `routing.i18n.ts:5-12`). The summary line and `permission.message` follow `permissionSummary.ts:54-108` and `PermissionCard.tsx:313-317`. The body depends on the tool:
  - shell and bash: description, working directory, timeout, then the command (`metadata.command|cmd|script`, otherwise `resources.join("\n")`) in a read-only gpui-kit `Editor` with language `bash`, plus any patterns that are not lines of the command.
  - edit, write and patch: one `Diff` per `metadata.files[].patch`, falling back to `metadata.path/changes/diff`. A write with `metadata.content` gets a diff whose lines are all additions. Each diff sits in a box with a height cap and scrolls inside it.
  - webfetch: method, URL, headers and body.
  - any other tool: the description and a collapsed `metadata` block.

  The buttons are Deny, then Always (only when `save` is not empty: `Always: {up to 2 patterns}` with `...` after the second, or `Always allow` when every pattern is `*`; for `external_directory` the directory glob is stripped, `permissionSummary.ts:116-125`), then Allow once as the primary (`PermissionCard.tsx:350-402`). The keys are Alt+Enter, Alt+Shift+Enter and Alt+Backspace, caught before the composer and taken by the newest card. While a reply is in flight every button is disabled and Allow once shows a spinner. On success the card waits for `permission.replied`. A 404 removes the card, and any other failure logs and keeps it.
- **Form dock.** Hidden while the permission scope is not empty (`ChatInput.tsx:5479-5482`). Each form becomes a gpui-kit `QuestionnaireState`, one item per answerable field, built as follows:
  - `string` with options, and `multiselect`: choices. An "Other" input appears when `custom` is set, and a validator accepts free text only while Other is chosen.
  - `string` without options: the item's input, with its placeholder and default.
  - `boolean`: Yes/No choices, defaulting to `default ?? false`.
  - `number` and `integer`: the input, with a validator that accepts only text that parses as a number, or as an integer for `integer`.
  - `external`: a gpui-kit `Link`. Opening it sets an acknowledged flag, and a validator fails until the flag is set.

  As in React, the dock enforces only required fields, acknowledged links and parseable numbers. Bounds, `minItems`, `maxItems`, `pattern` and `format` are the server's to reject (`formCardState.ts:185-200`, `FormFieldControl.tsx:169-187`).

  `hidden` fields and `when` clauses are applied by `set_item_disabled` on every `AnswerChanged`, reading only earlier active answers, where an unanswered field fails both `eq` and `neq` (`formCardState.ts:139-174`). Defaults are seeded as `formCardState.ts:43-102` does. The header shows the title (or `Input needed`), `{current} of {total}`, dots, `{count} more waiting`, and a collapse toggle (`Collapse the question` / `Expand the question`), which starts expanded (`FormDock.tsx:130, 306-333`). Answers and the step are kept per form in memory while the window is open, at most 50 forms, and restored only when the field signature is unchanged (`formDrafts.ts:20-45`, `FormDock.tsx:120-129`). The actions are Dismiss (ghost), Back, Next and Submit, using `questionnaire`'s own Enter, Ctrl+Enter and arrow keys. On submit the dock sends `answer` built from the active fields (`formCardState.ts:107-129`) and removes the form when the reply succeeds. A 404 drops the draft. A failure shows `Could not submit the answer` with `Try again.`. Dismiss sends the `DELETE`.
- **Web search consent.** A form with `metadata.kind == "websearch.provider"` and exactly one string field keyed `choice` or `provider` that has options renders the card of `WebSearchConsent.tsx:91-154` instead. For the `choice` step: Allow, Choose provider, Turn off web search. For the `provider` step: one button per option label. Not now cancels the form. A button replies `{ [fieldKey]: value }`.
- **Recovered question.** It shows only when no live form does, as the stepper with `The turn stopped while waiting. Submit starts a new paid turn.`. Submit sends `serializeFormAnswerAsMarkdown` (`formSerializers.ts:63-79`) as a new prompt with the linked assistant message's model, agent and variant, falling back to the session record and then the composer's selection (`session-actions.ts:2765-2819`). Dismiss only records the outcome.
- **Stop.** While the open session is not idle and its first send is not pending, Stop replaces Send. It is an error-colored button with a stop icon, labeled `Stop generating`, with two `Kbd` Esc in its tooltip. While the composer holds text, a second button above it reads `Queue message` or `Send message` by `followUpBehavior`. Escape inside the composer arms the abort, and a second Escape within 3000 ms for the same session sends `interrupt`. Escape counts only while the composer editor has focus, as `[data-chat-input]` limits it (`ComposerEditor.tsx:690`), and is ignored while a popup is open, during IME composition, or when the session is idle. Nothing changes on screen until `session.execution.interrupted` arrives. A failed interrupt only logs, as React does.
- **Shield.** It shows `shield-user`, `shield-star` or `shield-check`, with the tooltips `Permissions: ask every time`, `Permissions: safety net, ask only before risky actions` and `Permissions: accept everything` (`routing.i18n.ts:59-61`). A click sets the next mode in the cycle. A draft without a session shows ask and is inert.
- **Queue strip.** Above the editor, under the docks, and hidden on a draft. One queued item is one row. Two or more collapse into `Queued messages {n}` (gpui-kit `Collapsible` with a `Badge`), at most 4 rows tall, then the rows scroll. The expanded state is local to the window. A row shows the first line or `(empty)`, plus an agent badge `Sends as {agent}.` when the item's agent differs from the composer's. Its actions:
  - Edit: `take_item`, then put the text back in the composer.
  - Send now: takes that item only and sends it as a steer, or as a plain prompt when the session is idle. It dismisses nothing (`ChatInput.tsx:2065-2068, 2301`).
  - Remove.
  - Drag to reorder: gpui-kit `ListItem` with `on_drag`/`drag_over`/`on_drop`, as `tab_panel.rs:417-616` does, sending `PUT .../order`.

  An item that is sending shows no edit or remove. The app never dispatches queued items itself, since the server owns dispatch (`messageQueueStore.ts:51-57`).
- **Dismiss on send.** For a `Prompt` or `Steer` decision from the composer, the app removes the tree's permissions and forms from the dock immediately, then rejects and cancels them in parallel. A 404 clears the entry, and any other failure restores it. The send goes ahead either way: as decided for a `Steer`, and enqueued otherwise when anything was dismissed (`session-actions.ts:2531-2571, 2844-2889`).

### Parity reference

The capture tool seeds the new fixture models below and records these Electron states as references:
- `permission-bash`, `permission-edit`, `permission-write`, `permission-two`: two pending requests, showing the dots and `2 of 2`.
- `form-choice`, `form-multiselect`, `form-text`, `form-external`.
- `websearch-consent`, `recovered-question`.
- `queue-one`, `queue-three-expanded`, `busy-with-text`: Stop plus the follow-up button.

Every control of every reference, existing states included, also records what it draws:
- `icons`: in DOM order, each `svg` and `img` inside the control. For an `svg`, `name` is its `<use href="#oc-NAME">` symbol, or `other` for an svg React draws itself (such as `StopIcon`, `ContextRing` and the base of `DiffViewIcon`), and `color` is its color. For an `img`, such as a provider logo, `name` is `img:<id>`, where `<id>` is the file name of its `src` without extension (the bundled logo's id or the models.dev provider id), and `color` is absent.
- `marks`: in DOM order, each element inside the control with no text and no svg, whose own background fill is not transparent, and whose larger side is at most 12 px, with that fill's color. Borders, outlines, focus rings, box shadows and scrollbars are not marks. On the GPUI side a mark is a filled shape the view draws with no text or icon in it, under the same size rule.
- `texts`: in DOM order, each run of visible text with its color, where adjacent text nodes of the same color merge into one run. A control such as the model trigger, whose name and Thinking level have different colors, has two runs. The text is recorded too, so a run is matched by its text.

A color is recorded as `#rrggbbaa` in sRGB, in the resting state without hover or focus. The capture converts the computed value, `oklch` included, by painting it on a canvas and reading the pixel back. The element's own `opacity` and its ancestors' are multiplied into the alpha. The GPUI side converts the `Hsla` it paints, multiplied by any opacity the view applies, to the same form. The parity test compares `icons`, `marks` and `texts` for every shown control in every state, allowing a difference of 1 per channel for 8-bit rounding and nothing more. The dump reports the same three fields.

The `dock` region has two roots: the floating panel above the composer that holds the permission or form dock, and the queue strip's row group inside the composer box. Its bounds are the union of the two. A state fixture can carry a `ui` block of local view choices that no server state holds, such as the shown permission, the form step and the expanded queue. The fixture loader applies it after the replay. References are captured with Electron on Chromium's Ozone `headless` backend, with no window on the user's desktop. Their control bounds follow the headless font metrics and may differ by a few pixels from earlier captures. The gate does not compare control bounds. The dock and the strip together form the `dock` region described above. The parity test compares their controls in order, as it compares popups. It compares the region's left edge, right edge and bottom edge by the composer's device-pixel rule. The top edge and the height are left out, because a dock grows with its gpui-kit content.

### Failure behavior

- A list call that fails keeps the directory's previous requests, logs, and is retried on the next reconnect. It never shows an empty dock as fact.
- A control stream that drops keeps the last queue and modes until the next ready frame replaces them. While it is down the strip stays as it was, and a queue action still runs against the server.
- A queue action that fails logs and leaves the strip to the next snapshot. A `409 in flight` on edit or remove leaves the row unchanged.
- A failed `set_auto_accept` keeps the previous mode, because the state changes only from the snapshot.

## Package decisions

- **Highlighter: gpui-kit `tree-sitter-languages`.** Measured in a detached worktree on the release build, with headless renders through gpui-kit's real `Diff` and `TextView` paths, median of 5 runs:

  | | none | syntect | tree-sitter |
  | --- | --- | --- | --- |
  | Stripped binary | 49.3 MB | 50.6 MB | 100.0 MB |
  | New crates | 0 | 6 | 37 |
  | Rust diff, 2000 lines | 7 ms | 142 ms | 39 ms |
  | TSX diff, 2000 lines | 7 ms | 606 ms | 22 ms |
  | Markdown, 1000-line block | 5 ms | 76 ms | 13 ms |
  | Languages | 0 | 199 | 38 |

  Tree-sitter is 4 to 27 times faster, and speed is the reason for the port. Streaming markdown in the next change re-highlights code blocks as they grow. The user accepted the larger binary and the 38 languages.
- **Form stepper:** gpui-kit `questionnaire` (`gpui-base/src/questionnaire`), not a hand-built stepper. The field types it lacks map onto its choices and validators as the Form dock section says.
- **Diff and code:** gpui-kit `Diff` and the read-only `Editor`.
- **Drag to reorder:** gpui-kit has no sortable list. Rows use GPUI's `on_drag`, `drag_over` and `on_drop` the way gpui-kit's own tab panel does. The index math is the app's own, because no maintained crate offers reorder for GPUI rows.
- **No other new dependency.** The control stream reuses the event stream's SSE reader.

## Testing

- **Fixtures.** `scripts/perf/fixture-provider.mjs` gains models that call `bash`, `edit`, `write` and `question` once, then answer. `tools/capture-api.mjs` runs them under a config that sets `permission` to `ask` and saves:
  - the list routes, a reply, a cancel and an interrupt;
  - every queue route, the auto-accept and routing routes;
  - the frames of `/api/openchamber/events` and the matching `/api/event` frames.
- **Unit tests** in `oc-client` (decode every fixture, `FormField` bounds, the 404 variant) and `oc-state`:
  - scope order and descendants;
  - list replace per directory and the failed-list rule;
  - the mode lineage and the cycle with safety present and missing;
  - every recovered-question classification;
  - revision ordering of queue snapshots;
  - every branch of `decide_send`, ported from `ChatInput.keystroke.test.tsx`, plus the send order with a permission pending, an empty composer and two queued items (nothing is taken or sent, the queue stays);
  - the abort window;
  - the answer builder per field type, ported from `formCardState` tests;
  - the theme mapping against `zedThemeFromAppTheme` for one theme.
- **Interaction tests** headless through `gpui_kit::test`:
  - each dock's buttons and keys;
  - Stop and Escape Escape;
  - form navigation, validation and submit;
  - consent buttons;
  - queue strip edit, send now, remove and drag;
  - Enter while busy under both behaviors;
  - shield cycling.

  The tests run generically over every dock kind, as the popup tests do.
- **Parity** over the new states, headless.
- **`sidecar_it`** with the fixture provider:
  - a bash permission answered once;
  - an edit denied;
  - a question answered;
  - a message queued during a turn and sent by the server after it;
  - Stop of a running turn.

## Risks

- The binary doubles. The cost is accepted, and it is measured above.
- Comparing colors exactly will fail many existing controls at first, because the window palette and most icons differ from Electron today. Unit 14 fixes the existing surfaces before the new ones are built, so the dock and composer units start from a green gate.
- `questionnaire` covers the field types only through validators and disabled items. A form whose `when` clauses chain across many fields re-evaluates on every answer. That is cheap at the sizes OpenCode sends, but it is not measured.
- The control stream is new to the app. A server without it, such as an older OpenChamber, answers 404. The app then logs once, shows no queue strip and no shield modes, and keeps sending as plain prompts.
- Recovered questions depend on the loaded tool parts. A session whose messages are not loaded shows no recovered question until it is opened, which matches React, whose store also holds only loaded parts.
