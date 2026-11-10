# Event fixtures

Raw `GET /api/event` streams from a real server (see `packages/desktop-rs/fixtures/api/README.md` for how they are captured and
what varies between runs). Each `*.json` is a JSON array of the parsed `data:` payloads of one
connection, in arrival order, nothing removed or rewritten. A file starts when the connection opens, so
it begins with `server.connected` and, in the first scenario, with startup notices.

| File | Scenario |
| --- | --- |
| `text-reply-stream-300cps.json` | one plain text reply (`stream-300cps`) |
| `agent-20tools-300cps.json` | an agent turn: 20 `glob` calls, then the document; `glob` allowed by a session rule |
| `interrupted-agent-turn.json` | an `agent-20tools-300cps` turn interrupted with `POST /api/session/:id/interrupt` after the first tool result |
| `permission-asked-agent-turn.json` | the same turn with `glob` set to `ask`; the script answers `permission.asked` with decision `always` |
| `two-sessions-stream-300cps.json` | two sessions of the same directory prompted together, one stream |
| `session-lifecycle.json` | a session created and deleted while the stream is open |

`index.json` names the session ids of each file and what it shows, `summary.json` lists for each file every
event type with its count and the keys of the envelope and of `data`, and `framing.json` has the raw
frames.

## Wire framing

Content type `text/event-stream`, status 200. Every event is one frame of a single line,
`data: <json>` followed by a blank line. There are no `event:`, `id:` or `retry:` lines. A keep-alive
comment frame `: heartbeat` follows `server.connected` in every capture. No data event is wrapped in a
comment. Parse by `data:` only; the type is in the JSON.

## Envelope

```json
{ "id": "evt_…", "created": 1794282802926, "type": "session.…", "location": {"directory": "…"}, "data": {…}, "durable": {"aggregateID": "ses_…", "seq": 2, "version": 1}, "metadata": {…} }
```

- `server.connected` has only `id`, `type` and `data: {}` (no `created`, no `location`).
- `created` (ms) is missing only there; `location` is present on most session events but **not** on
  `session.execution.*`, `session.usage.updated`, `session.metadata.updated`, `session.deleted`,
  `models-dev.refreshed` and `worktree.resolved`. The stream cannot be filtered by directory from the
  envelope alone; `data.sessionID` is the key (design.md "filter by known session" holds).
- `durable` (the event log position) is present on events that are stored, absent on `text.delta`,
  `usage.updated` and `permission.*`.
- Types seen but not needed by the spike: `agent.updated`, `command.updated`, `integration.updated`,
  `model.updated`, `models-dev.refreshed`, `plugin.updated`, `project.updated`, `provider.updated`,
  `reference.updated`, `skill.updated`, `vcs.branch.updated`, `websearch.updated`, `worktree.resolved`,
  `session.instructions.updated`, `session.metadata.updated`, `session.usage.updated`,
  `session.inbox.delivered`, `session.step.streamed`, `session.tool.input.ended`.

## Check against `events.ts` (task 2.5)

Each saved type was compared with its `case` in `packages/ui/src/lib/opencode/events.ts`
(`translateWireEvent`, lines 264-870) and with design.md "State". Field names match `events.ts` in every
captured event: nothing is renamed or moved. What differs is the design's list of events and what it
says about them.

### Events the design does not list but the spike needs

1. **`session.inbox.enqueued` is how the user's message arrives.** There is no `session.user.*` event.
   `data` is `{inboxID, sessionID, item: {type: "user", payload: {text}, delivery}}`; `inboxID` is the
   message id the client minted in `POST …/prompt` (`events.ts:422-438` projects it into a user message).
   `session.inbox.delivered` (`{inboxID, sessionID}`) follows and stamps the time (`events.ts:439`). Without
   this the "user message appears" scenario cannot work from the stream; add `session.inbox.enqueued`
   (and `delivered`, `cancelled` for a withdrawn message) to the handled list.
2. **The tool name comes only from `session.tool.input.started`.** It carries `{id, name, assistantMessageID,
   sessionID}`. `session.tool.called` has `{id, input, executed, …}`, and `session.tool.success` has
   `{id, content: [{type, text}], metadata, executed, …}`; neither has `name`. The design lists only `called`,
   `success`, `failed`; to show `tool name: status` the reducer must create the tool row on `input.started`
   (status pending, `events.ts:635`). `input.ended` carries the raw input text; `input.delta` and `progress`
   are not produced by the fixture provider.
3. **`session.execution.started` is the only "busy" event, and there are no `session.status` or
   `session.idle` events in any capture.** `events.ts` itself turns `execution.started` into a busy
   status (`:392`), and `execution.succeeded|interrupted|failed` into idle (`:397-418`). `session.status`
   and `session.idle` are in the design's list and in `events.ts` (`:386-389`) but the server never sent
   them here, so the reducer's tests cannot be fed from fixtures for those two.

### Fields and semantics the design leaves out

4. **`session.execution.interrupted` has `reason`** (`"user"` here). `events.ts:402-413` ignores the event
   when `reason === "shutdown"` (OpenCode itself going away; the turn stays open). The design does not say
   so; the reducer must not mark such a session idle. Only `"user"` is in the fixtures.
5. **Each step is its own assistant message.** An agent turn with 20 tool calls has 21 `session.step.started`,
   each with its own `assistantMessageID`, a `text.started` (always `ordinal: 0`) and, but the last, one
   tool call. The design's "meta row under each assistant turn" is therefore one row per step if it follows
   `assistantMessageID`; grouping steps into a turn (the first `execution.started` to the next
   `execution.succeeded`/`interrupted`) is left to the reducer. `step.started.data.model` is an object
   `{id, providerID, variant}` (not a string) and `agent` is a string; `step.ended` has
   `finish` (`"tool-calls"`, `"stop"`), `rawFinish`, `tokens`, `cost`, `snapshot`, `files`. The step's start
   time is `data.started`; `events.ts:510` uses the envelope `created` for `time.created`.
6. **`session.text.ended` carries the whole `text`** and `events.ts:586-596` replaces the part with it. A
   reducer that only appends deltas will diverge if it ever misses one; replace on `ended`.
7. **`permission.asked` names the request `data.id`, `permission.replied` names it `data.requestID`.**
   `replied` also has `reply` (`"always"`). The pending flag must match `asked.data.id` with
   `replied.data.requestID`. `asked.data` also has `action`, `resources`, `save`, `metadata`,
   `source: {type: "tool", messageID, id: <call id>}`. `events.ts:810-813` passes `asked` through whole and
   keeps only `sessionID` and `requestID` of `replied`. Order in the fixture: `tool.called`,
   `text.ended`, `step.streamed`, `permission.asked`, `permission.replied`, `tool.success`. The
   permission reply endpoint is `POST /api/session/:id/permission/:requestID/reply`.
8. **`session.created` has no `title`** (`data`: `sessionID`, `slug`, `version`, `projectID`, `location`,
   `subpath`; `events.ts:274-290` reads `title ?? ""` and the optional `parentID`, `agent`, `model`,
   `metadata`, `permissions`, which are absent). Titles arrive later in `session.renamed`
   (`{sessionID, title}`); here the generated title was the first line of the reply, because the fixture
   provider cannot title. `session.deleted` has only `sessionID`.
9. **`session.usage.updated` (once per step) and `session.metadata.updated` (twice per turn)** are
   patched into the session record by `events.ts` (`:359`, `:300`). The design does not list them, and
   the spike can drop them.
10. The first state of a connection is not a snapshot: `server.connected` carries no session data, and a
    session already running when the stream opens shows nothing until its next event. The refetch after a
    reconnect in design.md "Runtime" is the only way to learn it.

### Not covered by any capture

`session.tool.failed`, `session.execution.failed`, `session.step.failed`, `session.reasoning.*`,
`session.tool.input.delta`, `session.tool.progress`, `form.created|replied|cancelled`, `session.status`,
`session.idle` and `session.retry.scheduled`: the fixture provider never fails a call, never reasons and never
asks a form. The reducer tests for these have to build their payloads by hand from `events.ts` and the
`@opencode/client` types.

### A delta before its start

The captures always show `text.started` before the first `text.delta` of an item, so the "delta before
started" test must be built by deleting the `text.started` event from one of these files.
