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

## What is normalized

Nothing. Files are the response body, re-indented with `JSON.stringify(_, null, 2)`; no field is
removed or rewritten. Every run mints new ids (`ses_…`, `msg_…`, `evt_…`, `per_…`), timestamps
(`time.*`, `created`), pagination cursors, and a different count of streamed text chunks. The project
directory is `/tmp/oc-desktop-rs-capture/project` and OpenCode's home `/tmp/oc-desktop-rs-capture/state/home`
in every file (`os.tmpdir()` on the capturing machine). Run-to-run the same files exist
with the same keys; only those values differ. Decoders must not depend on them.

## Index

`index.json` lists every saved call: `name`, HTTP method, path, status, content type, whether a
request body (`<name>.request.json`) and a response body (`<name>.json`) exist. `routes.json` is the
`--api-only` route check (task 2.2): every route answered, none with 404 or the HTML fallback.

| Route | Result under `--api-only` |
| --- | --- |
| `GET /auth/session` | 200 `{"authenticated": true, "disabled": true}` |
| `GET /api/session` | 200 |
| `GET /api/session/:id/message` | 200 |
| `POST /api/session/:id/prompt` | 200 |
| `POST /api/session/:id/interrupt` | 200 |
| `GET /api/event` | 200 `text/event-stream` |

Also used and answering: `POST /api/session`, `DELETE /api/session/:id`, `POST /api/session/:id/model`,
`POST /api/session/:id/permission/:requestID/reply`.

## Files

| File | Call |
| --- | --- |
| `auth-status.json` | `GET /auth/session`, no `x-opencode-directory` header |
| `session-list-empty.json` | `GET /api/session` before any session exists |
| `session-list-empty-query.json` | the same with `?limit=50&directory=<encoded dir>`, the query the React UI adds |
| `session-create.json`, `.request.json` | `POST /api/session` with `{"location": {"directory": …}}` |
| `session-list-nonempty.json` | list right after the create: the item has no `title` |
| `session-list-after-turn.json` | list after a finished turn: `title`, `outcome`, `metadata`, `time.busy` appear (`subpath` only when the project is a subfolder of a repository, not here) |
| `prompt-text.request.json`, `prompt-text.json` | the prompt body and its response |
| `messages-empty.json`, `messages-text-turn.json`, `messages-agent-turn.json`, `messages-interrupted-turn.json`, `messages-permission-turn.json` | `GET /api/session/:id/message` |
| `interrupt-idle.json`, `interrupt-running.json` | `POST /api/session/:id/interrupt` with nothing running (`interrupted: false`) and during a turn (`true`) |
| `switch-model*.request.json` | `POST /api/session/:id/model` (the UI sends this before a prompt that changes the model; no response body) |
| `session-create-*.json`, `prompt-*.json` | the remaining sessions and prompts of the event scenarios |
| `permission-reply.request.json` | `POST /api/session/:id/permission/:requestID/reply` (no response body) |

## Request shapes

Taken from the `@opencode/client` 2.2.6 SDK, which `packages/ui/src/lib/opencode/client.ts` calls
(checked by running the SDK against a recording `fetch`; the UI's base URL is the server origin, so
paths begin with `/api/…`):

- Every OpenCode call carries `x-opencode-directory: encodeURIComponent(<directory>)`.
- Prompt (`client.ts:1610-1620`): `POST /api/session/:id/prompt`, `content-type: application/json`,
  body `{"id": "<client-minted msg_…>", "text": "…"}`. `files`, `agents`, `skills`, `metadata` and
  `delivery` are left out when empty, and the composer's plain send passes none of them. The id is
  `ascendingId("msg")` (`packages/ui/src/lib/opencode/ids.ts`): `msg_` + 12 hex digits of
  `Date.now() * 4096 + counter` + 14 random base-62 characters; the capture script mints it the same way.
  The response carries `delivery: "steer"`, the server default.
- Interrupt: `POST /api/session/:id/interrupt`, no body, no content type.
- Permission reply: `POST /api/session/:id/permission/:requestID/reply`, body `{"decision": "once" | "always" | "reject"}`.
- Create session: body `{"location": {"directory": "…"}}`; a session rule is `permissions: [{action, resource, effect}]`.

## Shapes worth knowing for the Rust types

- Responses are wrapped: create is `{"data": {…}}`, lists are `{"data": […], "cursor": {"previous", "next"}}`,
  the prompt response is `{"data": {…}}`. The auth status and the interrupt answer are bare objects.
- The message list is newest first by default (`order: desc`). One agent turn of N tool calls is N + 1
  messages of type `assistant`, one per step; each holds `content` items of type `text` and `tool`
  (`id` = call id, `name`, `state.status`, `state.input`, `state.content`). A turn ends with a message of
  type `idle` and an `outcome` (`succeeded`, `interrupted`). Other types seen: `user` (`text`), `model-switched`.
- Session items carry no `title` until the first turn has been titled.
- The captured permission session answers `permission.asked` once: `decision: "always"` saved a rule
  that covered the later `glob` calls (`save: ["*"]` in the event).

## Scenarios and their sessions

Sessions of the non-permission scenarios are created with a session rule that allows `glob`
(`session-create-allow-glob`, `session-create-interrupt`); the OpenCode config asks for every `glob`
call otherwise (`permission: {"glob": "ask"}`), and `session-create-ask-glob` uses it.
