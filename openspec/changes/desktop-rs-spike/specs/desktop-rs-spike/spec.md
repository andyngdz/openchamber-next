## ADDED Requirements

### Requirement: Sidecar server lifecycle

The desktop app SHALL start the OpenChamber server by running the existing `openchamber` CLI with `serve --foreground --quiet --api-only --host 127.0.0.1 --port <free port>`, read the listening port from the first stdout line within 30 seconds, and stop the server and its managed OpenCode process when the app exits.

#### Scenario: Server starts

- **WHEN** the app launches and the CLI prints a port line
- **THEN** the app connects to `http://127.0.0.1:<printed port>`, `GET /auth/session` returns `authenticated: true`, and the header server state shows connected

#### Scenario: Server fails to start

- **WHEN** `node` or the CLI is missing, the process exits early, or no valid port line arrives within 30 seconds
- **THEN** the header server state shows the error, the chat column shows the last lines of stderr and of the server log file with a retry button, and the sidebar shows that the server is not running

#### Scenario: App closes normally

- **WHEN** the user closes the window
- **THEN** the app sends SIGTERM to the CLI, and after 10 seconds sends SIGKILL to a CLI still running and SIGTERM to each recorded `opencode serve` child still running, so that neither the CLI nor its OpenCode process remains

#### Scenario: App crashes

- **WHEN** the app process dies without running its shutdown
- **THEN** the CLI receives SIGTERM through the parent-death signal and stops itself and its OpenCode process

### Requirement: Session list and messages

The app SHALL check `GET /auth/session` for `authenticated: true`, list the sessions of the selected directory, show the messages of the opened session, and create a new session on request. A failed request SHALL show an error with a retry action and SHALL NOT show an empty list.

#### Scenario: Directory has sessions

- **WHEN** the app is connected and the directory has sessions
- **THEN** the sidebar lists them and selecting one shows its user and assistant messages

#### Scenario: New session

- **WHEN** the user presses "New session"
- **THEN** the app creates a session in the directory, adds it to the sidebar and opens it

#### Scenario: Fetch or auth fails

- **WHEN** the auth check does not return `authenticated: true`, or listing sessions or messages returns a non-2xx status or a body that does not decode
- **THEN** the affected region shows the error and a retry button

### Requirement: Send a prompt and stream the reply

The app SHALL send the composer text as a prompt to the open session and render the reply as events arrive on `/api/event`.

#### Scenario: Prompt streams

- **WHEN** the user sends a prompt in an idle session
- **THEN** the user message appears, the assistant text grows as text delta events arrive, tool calls appear with their status, the send button is disabled while the session is busy, and it is enabled again when the session is idle

#### Scenario: Prompt fails

- **WHEN** the prompt request fails
- **THEN** an error shows above the composer and the typed text stays in the composer

#### Scenario: Agent asks for permission

- **WHEN** a `permission.asked` or `form.created` event arrives for the open session
- **THEN** the message pane shows that the agent is waiting for an answer, until a matching reply or cancel event arrives

### Requirement: Event stream reconnect

The app SHALL reconnect a dropped event stream with backoff from 500 ms doubling up to 10 seconds, and SHALL refetch the session list and the open session's messages after reconnecting.

#### Scenario: Stream drops

- **WHEN** the event stream closes or errors while the server is running
- **THEN** the header server state shows reconnecting, the messages on screen stay, and after reconnecting the session list and open messages are refetched

### Requirement: Layout parity with the current desktop

The app SHALL place every region and control where the current OpenChamber desktop places it: sidebar, header, chat column with composer, Work status panel and icon rail. Regions and controls outside the spike's features SHALL render in place and do nothing when used. Styling MAY differ. UI text SHALL be English.

#### Scenario: Region bounds match

- **WHEN** `oc-app --dump-layout` runs at a 1440x900 window with `--state` set to each reference state (empty, open session, streaming)
- **THEN** the x, y, width and height of each region equal those in `fixtures/reference/<state>.json`, captured from the Electron desktop window at the same size with the sidebar at 280 px and the Work status panel at its default width

#### Scenario: No control is missing

- **WHEN** the dumped controls of a region are compared with the reference
- **THEN** every reference control is present by name and the controls appear in the same order

#### Scenario: Inert control is used

- **WHEN** the user clicks a control outside the spike's features
- **THEN** nothing changes and no error is shown
