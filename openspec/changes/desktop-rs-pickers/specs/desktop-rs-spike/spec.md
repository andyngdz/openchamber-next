## MODIFIED Requirements

### Requirement: Session list and messages

The app SHALL check `GET /auth/session` for `authenticated: true`, list the sessions of every project in the shared settings and of the Chats section from the global session list, show the messages of the opened session, and create a new session on request. A failed request SHALL show an error with a retry action and SHALL NOT show an empty list. A page that fails after earlier pages loaded SHALL NOT hide the sessions already shown.

#### Scenario: Directory has sessions

- **WHEN** the app is connected and the settings list projects that have sessions
- **THEN** the sidebar shows the sections the settings enable (Chats, Pinned, In work, Projects by default) in their settings order, each project with its sessions in the React order and limit, and selecting any session shows its user and assistant messages

#### Scenario: New session

- **WHEN** the user presses "New session" for a project or sends from a Chat draft
- **THEN** the app creates a session in that project's directory, or in a new folder under the chats root for Chat, adds it to the sidebar and opens it

#### Scenario: Fetch or auth fails

- **WHEN** the auth check does not return `authenticated: true`, or listing sessions or messages returns a non-2xx status or a body that does not decode
- **THEN** the affected region shows the error and a retry button

#### Scenario: Session list is incomplete

- **WHEN** the first page of the global session list loads and a later page fails after its retries
- **THEN** the sessions already shown stay, the sidebar shows that the list is incomplete with a Retry action, and nothing is presented as the full list

### Requirement: Event stream reconnect

The app SHALL reconnect a dropped event stream with backoff from 500 ms doubling up to 10 seconds, and SHALL refetch the settings, the global session list, the model and agent catalog of every loaded directory, and the open session's messages after reconnecting.

#### Scenario: Stream drops

- **WHEN** the event stream closes or errors while the server is running
- **THEN** the header server state shows reconnecting, the messages on screen stay, and after reconnecting the settings, the global session list, the loaded catalogs and the open messages are refetched
