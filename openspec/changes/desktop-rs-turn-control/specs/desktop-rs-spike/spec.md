## MODIFIED Requirements

### Requirement: Event stream reconnect

The app SHALL reconnect a dropped event stream with backoff from 500 ms doubling up to 10 seconds. After reconnecting it SHALL refetch the settings, the global session list, the model and agent catalog of every loaded directory, the open session's messages, and the pending permissions and forms of every directory with a loaded catalog or an open session.

#### Scenario: Stream drops

- **WHEN** the event stream closes or errors while the server is running
- **THEN** the header shows the server state as reconnecting and the messages on screen stay, and after reconnecting the settings, the global session list, the loaded catalogs, the open messages and the pending permissions and forms are refetched

#### Scenario: Pending list fails after reconnect

- **WHEN** the permission list for a directory fails after a reconnect
- **THEN** that directory's previously known requests stay in the dock, and the failure is logged
