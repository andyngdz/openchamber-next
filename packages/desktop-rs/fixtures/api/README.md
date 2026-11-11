# API fixtures

Raw responses of a real OpenChamber server started the way the sidecar starts it
(`node packages/web/bin/cli.js serve --foreground --quiet --api-only --host 127.0.0.1 --port <port>`),
captured by `packages/desktop-rs/tools/capture-api.mjs` against the deterministic fixture provider
(`scripts/perf/fixture-provider.mjs`). Regenerate with:

```bash
node packages/desktop-rs/tools/capture-api.mjs
```

The script replaces the `*.json` files of `fixtures/api/` and `fixtures/events/` (the READMEs stay), runs in
`<os tmpdir>/oc-desktop-rs-capture/` (isolated `HOME`, `OPENCHAMBER_DATA_DIR` and `OPENCODE_CONFIG_CONTENT`),
stops everything it started and removes that folder. It holds all output in memory and exits non-zero,
writing nothing, if any file would contain the home directory, the repository root or the user name as a
path segment.

The fixture provider listens on the fixed port 4611, so the base URL carried in `config.json`,
`providers.json` and `models.json` is the same in every run. A process already on that port stops the
capture instead of being reused. The capture's OpenCode config registers the fixture provider twice
(`perf` and `perf-second`) with several models each; only `perf/stream-600cps` and
`perf-second/alt-thinking` keep reasoning levels (`low`/`high` and `low`/`medium`/`high`), and the
config adds `capture-primary`, `capture-secondary`, the hidden `capture-hidden` and the
`capture-subagent` agent. The server starts on settings seeded with two projects (the first with a
colour and icon), favourites, recents, `lastUsedEfforts` and the sidebar sections before it first reads
them.

## What is normalized

No response field is rewritten. Files are the response body, re-indented with `JSON.stringify(_, null, 2)`.
Every run mints new ids (`ses_…`, `msg_…`, `evt_…`, `per_…`, `frm_…`, `queued-<ms>-<random>`,
`sh_…`), random session slugs (`slug`), timestamps (`time.*`, `created`, `archivedAt`, `data.time.active`,
the archived stamp, `metadata.openchamber.work.openedAt`, `metadata.openchamber.turn.endedAt`), the measured
`metadata.openchamber.turn.durationMs`, pagination cursors, message ids in prompt bodies, a different count of
streamed text chunks, and a different server instance id in `config.json`'s managed-config paths
(`.../run/instances/<pid>/...`). The project directory is `/tmp/oc-desktop-rs-capture/project` and OpenCode's
home `/tmp/oc-desktop-rs-capture/state/home` in every file (`os.tmpdir()` on the capturing machine). One
`/api/event` connection carries every session of the directory, so two runs can interleave events of
independent sessions differently; a replay must key on type and session, not on array index.

Three timing effects follow from running real turns rather than a replay:
- An interrupted turn's last `session.text.*` part holds only the prefix that streamed before the interrupt, so
  its length varies with when the interrupt lands.
- The intermediate `session.usage.updated` totals can differ when OpenCode's own requests overlap (title
  generation next to the last step); the final total is the same.
- A stored event's `durable.seq` can differ by the position its event reached in the aggregate when two writers
  raced; the event set and its order within one session are the same.

Run-to-run the same files exist with the same keys; only those values, that interleaving and those timing
effects differ. Decoders must not depend on them.

The Chat folder is pinned as `<chatsRoot>/2026-01-02/session-00000000-0000-4000-8000-000000000000` so
`fs-mkdir.request.json` and the Chat session's `location.directory` do not change between days.

## Index

`index.json` lists every saved call: `name`, HTTP method, path, status, content type, whether a
request body (`<name>.request.json`) and a response body (`<name>.json`) exist. `routes.json` is the
`--api-only` route check: every route answered, none with 404 or the HTML fallback.

| Route | Result under `--api-only` |
| --- | --- |
| `GET /auth/session` | 200 `{"authenticated": true, "disabled": true}` |
| `GET /api/fs/home` | 200 |
| `GET /api/config/settings?surface=desktop` | 200 |
| `PUT /api/config/settings?surface=desktop` | 200, the full document |
| `GET /api/config` | 200 |
| `GET /api/provider` | 200 |
| `GET /api/model?types=chat&types=image` | 200 |
| `GET /api/agent` | 200 |
| `GET /api/session` | 200 |
| `GET /api/session?limit=2` (no directory) | 200, one global page with `cursor.next` |
| `GET /api/session/:id` | 200 |
| `GET /api/session/:id/message` | 200 |
| `POST /api/session` | 200 |
| `POST /api/session/:id/prompt` | 200 |
| `POST /api/session/:id/model` | 204 |
| `POST /api/session/:id/agent` | 204 |
| `POST /api/session/:id/interrupt` | 200 |
| `POST /api/fs/mkdir` | 200 |
| `POST /api/openchamber/sessions/archive` | 200 |
| `POST /api/openchamber/sessions/:id/metadata` | 200 |
| `GET /api/permission/request?location[directory]=` | 200, the directory's pending requests |
| `GET /api/form?location[directory]=` | 200, the pending forms, `global` included |
| `POST /api/session/global/form` | 200, a location-scoped form |
| `GET /api/permission-auto-accept` | 200, `{sessions, modes, revision}` |
| `PUT /api/permission-auto-accept/sessions/:id` | 200, the whole snapshot |
| `GET /api/routing` | 200, `available`, `jevAvailable` and `heldPermissions` |
| `GET /api/event` | 200 `text/event-stream` |
| `GET /api/openchamber/events` | 200 `text/event-stream` |

Also used and answering: `DELETE /api/session/:id`,
`POST /api/session/:id/permission/:requestID/reply`,
`POST /api/session/:id/form/:formID/reply`,
`DELETE /api/session/:id/form/:formID`, `POST /api/message-queue/sessions/:id/items`,
`POST /api/message-queue/sessions/:id/items/:itemID/take`,
`POST /api/message-queue/sessions/:id/take`,
`PUT /api/message-queue/sessions/:id/order`,
`DELETE /api/message-queue/sessions/:id/items/:itemID` and `GET /api/message-queue`.

## Files

| File | Call |
| --- | --- |
| `auth-status.json` | `GET /auth/session`, no `x-opencode-directory` header |
| `fs-home.json` | `GET /api/fs/home`, no header: `home`, `chatsRoot` and the two canonical aliases |
| `settings.json` | `GET /api/config/settings?surface=desktop` after the seeded settings file is read: two projects, favourites, recents, `lastUsedEfforts`, sidebar sections |
| `settings-update.request.json`, `settings-update.json` | partial `PUT /api/config/settings?surface=desktop` and the full document it answers |
| `config.json` | `GET /api/config`: the config sources, folded by `mergeConfigDocuments` in the UI; the last document carries `model`, `default_agent: "capture-secondary"`, `providers` and `agents` |
| `providers.json` | `GET /api/provider`: `{location, data}`, three providers (`opencode`, `perf`, `perf-second`) |
| `models.json` | `GET /api/model?types=chat&types=image`: `{location, data}`; `stream-600cps` has `low`/`high`, `alt-thinking` has `low`/`medium`/`high`, every other fixture model has an empty `variants` |
| `agents.json` | `GET /api/agent`: `{location, data}`; built-ins plus the four capture agents, one hidden and one subagent |
| `fs-mkdir.request.json`, `fs-mkdir.json` | `POST /api/fs/mkdir` for the Chat folder `<chatsRoot>/2026-01-02/session-<uuid>` |
| `session-list-empty.json` | `GET /api/session` before any session exists |
| `session-list-empty-query.json` | the same with `?limit=50&directory=<encoded dir>`, the query the React UI adds |
| `session-create.json`, `.request.json` | `POST /api/session` with `{"location": {"directory": …}}` |
| `session-create-agent-model*.request.json`, `.json` | `POST /api/session` with `agent` and `model`, once with a `variant` and once without |
| `session-list-nonempty.json` | list right after the create: the item has no `title` |
| `session-list-after-turn.json` | list after a finished turn: `title`, `outcome`, `metadata`, `time.busy` appear (`subpath` only when the project is a subfolder of a repository, not here) |
| `session-get.json` | `GET /api/session/:id` for the session created with a variant: its `agent` and `model` |
| `switch-model.request.json`, `switch-model-variant.request.json`, `switch-model-default.request.json` | `POST /api/session/:id/model` with no variant, with `variant: "high"`, and with Default (no `variant`) |
| `switch-agent.request.json` | `POST /api/session/:id/agent` |
| `session-list-global-page-1.json`, `session-list-global-page-2.json` | `GET /api/session?limit=2` with no header and no query, then the same with `cursor=<page 1's cursor.next>`: a Chat session, a pinned session, a `/btw` fork and an archived session |
| `seed-archive.request.json`, `seed-archive.json` | `POST /api/openchamber/sessions/archive` for the archived session |
| `seed-metadata-pinned.request.json`, `seed-metadata-pinned.json` | `POST /api/openchamber/sessions/:id/metadata` with `openchamber.pinned.at` |
| `seed-metadata-btw.request.json`, `seed-metadata-btw.json` | the same with `openchamber.kind: "btw"` and `originalSessionID` |
| `prompt-text.request.json`, `prompt-text.json` | the prompt body and its response |
| `messages-empty.json`, `messages-text-turn.json`, `messages-agent-turn.json`, `messages-interrupted-turn.json`, `messages-permission-turn.json` | `GET /api/session/:id/message` |
| `interrupt-idle.json`, `interrupt-running.json` | `POST /api/session/:id/interrupt` with nothing running (`interrupted: false`) and during a turn (`true`) |
| `session-create-*.json`, `prompt-*.json` | the remaining sessions and prompts of the event scenarios |
| `permission-reply.request.json` | `POST /api/session/:id/permission/:requestID/reply` (no response body) |
| `permission-request-list.json` | `GET /api/permission/request`: the pending bash request, `action: "shell"`, `resources: ["echo fixture-permission"]` and `save: ["echo *"]` |
| `permission-reply-once.request.json`, `permission-reply-reject.request.json`, `permission-reply-always.request.json` | the three decisions, one per tool |
| `permission-reply-404.json`, `.request.json` | a reply to a request that does not exist: 404 |
| `form-list.json` | `GET /api/form`: the question form and the `global` form |
| `form-create-global.json`, `.request.json` | `POST /api/session/global/form`: the location-scoped form, `sessionID: "global"` |
| `form-reply.request.json` | `POST /api/session/:id/form/:formID/reply` with one answer per question field (a list for the multiselect) |
| `form-cancel.json` | `DELETE /api/session/global/form/:formID` (no body) |
| `form-reply-404.json`, `.request.json`, `form-cancel-404.json` | a reply and a cancel for a form that does not exist: 404 |
| `interrupt-question-turn.json`, `interrupt-queue-turn.json` | `POST /api/session/:id/interrupt` during the question and queue turns |
| `prompt-steer.request.json`, `prompt-steer.json` | `POST /api/session/:id/prompt` with `delivery: "steer"` while the session is busy |
| `message-queue-enqueue-1.json`, `-2`, `-3`, `-4` and their `.request.json` | `POST /api/message-queue/sessions/:id/items`: `{revision, session, itemId}` |
| `message-queue-list.json` | `GET /api/message-queue`: three queued items at revision 3 |
| `message-queue-reorder.request.json`, `.json` | `PUT /api/message-queue/sessions/:id/order` with all three ids in the new order |
| `message-queue-take-one.json` | `POST .../items/:itemID/take`: the snapshot plus the full item |
| `message-queue-take-all.json` | `POST .../take`: the empty snapshot plus the items in queue order |
| `message-queue-remove.json` | `DELETE .../items/:itemID` (no body) |
| `message-queue-remove-in-flight.json` | the same remove while the item is being sent: 409 `{"error": "message is being sent"}` |
| `permission-auto-accept.json` | `GET /api/permission-auto-accept` before any mode is set |
| `permission-auto-accept-set.json`, `.request.json` | the safety mode for one session, answered with `{sessions, modes, revision}` |
| `permission-reply-safety.request.json` | the reply that ends the held safety turn |
| `routing.json` | `GET /api/routing`: `available: true`, `jevAvailable: false`, an empty `heldPermissions` |
| `messages-*-turn.json` | `GET /api/session/:id/message` after each turn-control turn |

## Request shapes

Taken from the `@opencode/client` 2.2.6 SDK, which `packages/ui/src/lib/opencode/client.ts` calls
(checked by running the SDK against a recording `fetch`; the UI's base URL is the server origin, so
paths begin with `/api/…`):

- Every directory-scoped OpenCode call carries `x-opencode-directory: encodeURIComponent(<directory>)`. The
  global session list, `/api/fs/home` and `/api/config/settings` carry neither the header nor a directory
  query.
- Prompt (`client.ts:1610-1620`): `POST /api/session/:id/prompt`, `content-type: application/json`,
  body `{"id": "<client-minted msg_…>", "text": "…"}`. `files`, `agents`, `skills`, `metadata` and
  `delivery` are left out when empty, and the composer's plain send passes none of them. The id is
  `ascendingId("msg")` (`packages/ui/src/lib/opencode/ids.ts`): `msg_` + 12 hex digits of
  `Date.now() * 4096 + counter` + 14 random base-62 characters; the capture script mints it the same way.
  The response carries `delivery: "steer"`, the server default.
- Create session: body `{"location": {"directory": "…"}}`; a session rule is `permissions: [{action, resource, effect}]`.
  With a pick, it also carries `"agent": "<name>"` and `"model": {"providerID", "id"}` plus `"variant"`
  only when one is picked (`toModelRef`, `packages/ui/src/lib/opencode/model.ts:87-89`).
- Model and agent switches: `POST /api/session/:id/model` body `{"model": {"providerID", "id"[, "variant"]}}`,
  `POST /api/session/:id/agent` body `{"agent": "<name>"}`; both answer 204 with no body.
- Settings: `PUT /api/config/settings?surface=desktop` takes a partial document and answers with the whole
  one, so a client patches only the keys it changed.
- Global session list: no directory; `?limit=<n>` and then `?limit=<n>&cursor=<cursor.next>`. A response is
  `{"data": [...], "cursor": {"previous", "next"}}`, and the last page still carries `next`; callers stop
  when a page is short, `next` is missing or `next` repeats the cursor.
- Chat folder: `POST /api/fs/mkdir` body `{"path": "<chatsRoot>/<YYYY-MM-DD>/session-<uuid v4>"}`
  (`packages/ui/src/lib/chatDirectories.ts:109-118`), then a session whose `location.directory` is that path.
- Archive: `POST /api/openchamber/sessions/archive` body `{"directory", "ids", "archivedAt"}`, answered
  with `{"archived": [{"id", "archivedAt"}], "failedIds": []}`.
- Metadata: `POST /api/openchamber/sessions/:id/metadata` body `{"patch": {…}}`, a JSON Merge Patch
  answered with `{"metadata": {…}}`. Pins write `openchamber.pinned.at`
  (`packages/ui/src/lib/sessionPins.ts:47-60`); `/btw` forks write `openchamber.kind: "btw"` and
  `originalSessionID` (`packages/ui/src/lib/sessionBtwMetadata.ts:55-57`).
- Interrupt: `POST /api/session/:id/interrupt`, no body, no content type.
- Permission reply: `POST /api/session/:id/permission/:requestID/reply`, body `{"decision": "once" | "always" | "reject"}`.
- Permission and form lists take the location in the query, `?location[directory]=<encoded dir>`, as well as the
  directory header; the response is `{"location": {"directory"}, "data": [...]}`.
- Form create: `POST /api/session/global/form`, body `{"id", "title", "metadata", "fields"}`. The `id` must start
  with `frm_`; `sessionID` is the path segment, and `global` is the location-scoped sentinel. Form reply:
  `POST /api/session/:id/form/:formID/reply`, body `{"answer": {<key>: <value>}}`, 204 with no body. Form cancel:
  `DELETE /api/session/:id/form/:formID`, 204 with no body. A missing form answers 404 with
  `{"_tag": "FormNotFoundError", "id", "message"}`.
- Interrupt: `POST /api/session/:id/interrupt`, no body, answered `{"interrupted": true | false}`.
- Steer: the prompt body gains `"delivery": "steer"`; the response is the admitted user message with the same
  `delivery`.
- Message queue: `POST /api/message-queue/sessions/:id/items` with `{"directory", "item": {"content", "text",
  "sendConfig": {"providerID", "modelID", "agent"}}}`, answered with `{revision, session, itemId}`. `GET
  /api/message-queue` is `{revision, sessions: [{sessionId, directory, items, sendingId}]}`; an item is
  `{id, createdAt, content, text, attachments, sendConfig}`. Reorder takes `{"itemIds": [...]}` with the complete
  order; take one and take all answer the snapshot plus the taken items; remove answers the snapshot. A remove or
  take of the item being sent answers 409 `{"error": "message is being sent"}`.
- Auto-accept: `PUT /api/permission-auto-accept/sessions/:id` with `{"mode": "ask" | "safety" | "auto",
  "enabled": <mode != "ask">, "directory"}`. The answer, like the GET, is `{sessions, modes, revision}`, where
  `sessions` is the legacy boolean map and `modes` holds the mode names.

## Shapes worth knowing for the Rust types

- Responses are wrapped: create is `{"data": {…}}`, lists are `{"data": […], "cursor": {"previous", "next"}}`,
  the prompt response is `{"data": {…}}`. The auth status and the interrupt answer are bare objects.
- `GET /api/provider`, `GET /api/model` and `GET /api/agent` are `{"location": {"directory"}, "data": [...]}`.
  A model carries `id`, `modelID` (equal here), `providerID`, `name`, `variants` (records with `id`),
  `capabilities`, `limit` and `enabled`; a provider carries `id`, `name`, `activation`, `package` and
  `settings`; an agent carries `id`, `name`, `mode`, `hidden` and optionally `model`.
- `GET /api/config` is an array of config sources, not one object. Each source is `{type, path?, info?}`;
  the effective document is the `info` objects folded in order (the UI's `mergeConfigDocuments`). The last
  source carries `model`, `providers`, `agents`, `permissions` and `share` here; `default_agent` appears
  only when the config names one.
- The settings document holds the registry keys the UI reads, plus computed flags (`hasDesktopUiPassword`,
  `overviewFeatureAvailable`, …). `projects[]` keeps unknown fields, so a client patches only the fields it
  changes.
- The message list is newest first by default (`order: desc`). One agent turn of N tool calls is N + 1
  messages of type `assistant`, one per step; each holds `content` items of type `text` and `tool`
  (`id` = call id, `name`, `state.status`, `state.input`, `state.content`). A turn ends with a message of
  type `idle` and an `outcome` (`succeeded`, `interrupted`). Other types seen: `user` (`text`), `model-switched`.
- Session items carry no `title` until the first turn has been titled.
- The captured permission session answers `permission.asked` once: `decision: "always"` saved a rule
  that covered the later `glob` calls (`save: ["*"]` in the event).

## Scenarios and their sessions

Sessions of the non-permission scenarios are created with a session rule that allows `glob`
(`session-create-allow-glob`, `session-create-interrupt`); the OpenCode config asks for every `glob`,
`bash`, `edit` and `webfetch` call otherwise (`permission: {"glob": "ask", "bash": "ask", "edit": "ask",
"webfetch": "ask"}`), and `session-create-ask-glob` uses the `glob` rule. The write tool shares the `edit`
permission action, so the write turn asks too.

The last four sessions are the sidebar split's cases, created newest-first in this order so they land on
the first two global pages:

| Case | How it was made |
| --- | --- |
| Archived | a session, then `POST /api/openchamber/sessions/archive` for it |
| `/btw` fork | a session, then `openchamber.kind: "btw"` and `originalSessionID` through the metadata route |
| Pinned | a session, then `openchamber.pinned.at` through the metadata route |
| Chat | `POST /api/fs/mkdir` under `chatsRoot`, then a session whose directory is that folder |

## Turn control scenario

After the sidebar sessions, one run covers every turn-control route. Each model in
`scripts/perf/fixture-provider.mjs` named `<tool>-1200cps` calls that tool once, then streams the
document:

| Model | Call | Outcome |
| --- | --- | --- |
| `bash-1200cps` | `bash` with `echo fixture-permission` | asked, answered `once` |
| `edit-1200cps` | `edit` of `fixture-edit.txt`, seeded in the project with the fixed old line | asked, answered `reject` |
| `write-1200cps` | `write` of `fixture-write.txt` | asked (via the edit action), answered `once` |
| `webfetch-1200cps` | `webfetch` of `http://127.0.0.1:4611/fixture-page` | asked, answered `always` |
| `question-1200cps` | `question` with three fields: three options, a multiselect with custom, and free text | the form is replied to with one answer per field |

- The bash request and the question form are pending at the same time, so `permission-request-list.json` and
  `form-list.json` both carry a live entry. The question form also carries `metadata.kind: "question"` and
  the `metadata.tool` link the recovered-question rule reads.
- The `global` form is seeded through `POST /api/session/global/form` because OpenCode itself raises
  location-scoped forms only from MCP elicitations; the field list is fixed. It is then cancelled, which is
  the `form.cancelled` event.
- The second question turn is interrupted while its form is pending. Its question tool part ends in
  `state.status: "error"` with `Tool execution interrupted`, which is a turn death for the recovered-question
  rule.
- The queue session runs a `stream-100cps` turn; one prompt is steered into it, three messages are queued,
  reordered, taken one at a time and all at once, a fourth is removed, and a remove of the message the server
  is sending catches the 409.
- The safety session is set to `safety` with `PUT /api/permission-auto-accept/sessions/:id`. With no routing
  provider the safety net holds its bash request and the control stream reports
  `openchamber:permission-auto-accept.left-for-user`; the script then replies `once`.
- `prompt-steer.request.json` is the only prompt body with `delivery`, and the event stream then carries
  `session.inbox.enqueued` and `session.inbox.delivered` for it.
