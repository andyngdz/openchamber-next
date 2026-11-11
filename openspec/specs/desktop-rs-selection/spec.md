# desktop-rs-selection Specification

## Purpose
TBD - created by archiving change desktop-rs-pickers. Update Purpose after archive.

## Requirements

### Requirement: Project selection

The app SHALL let the user pick Chat or any project from settings in the empty state's Choose project popover, and SHALL make a picked project the active project in the shared settings.

#### Scenario: Pick a project

- **WHEN** the user opens Choose project, types part of a project's label or path and presses Enter on it
- **THEN** the draft targets that project, the trigger shows its label, `activeProjectId` is written to settings, and the model and agent defaults of that project apply

#### Scenario: Pick Chat

- **WHEN** the user picks the Chat row
- **THEN** the draft targets Chat, the trigger shows "Choose project", and `activeProjectId` is not changed

#### Scenario: Collapse a project

- **WHEN** the user clicks a project header in the sidebar
- **THEN** its sessions collapse or expand, `sidebarCollapsed` for that project is written to settings, and the active project does not change

### Requirement: Model and variant picker

The composer SHALL offer the enabled chat models of the draft's or session's directory, grouped as Favorites, Recent and one section per provider, searchable, with a star toggle per model and a Thinking menu for models that have variants.

#### Scenario: Pick a model

- **WHEN** the user opens the model picker, moves with the arrow keys and presses Enter on a model
- **THEN** the trigger shows that model, the model moves to the front of Recent (at most 5), and `recentModels` and `lastSelectedModel` are written to settings

#### Scenario: Pick a variant

- **WHEN** the model has variants and the user picks one in the Thinking menu
- **THEN** the Thinking trigger shows it and `lastUsedEfforts` for that model is written; picking Default stores `null` and no variant is sent

#### Scenario: Catalog fails

- **WHEN** the provider or model list for the directory fails
- **THEN** the model trigger shows Unavailable and the popover shows the error with Retry

### Requirement: Agent picker

The composer SHALL offer the visible primary agents of the directory, searchable, with favorites, and SHALL default to the agent the React cascade picks.

#### Scenario: Default agent

- **WHEN** no project or settings default agent is set and the OpenCode config names none
- **THEN** the agent picker shows `build`, or the first primary agent when `build` is missing

#### Scenario: Subagents are not offered

- **WHEN** the directory has agents with mode `subagent` or marked hidden
- **THEN** they do not appear in the agent picker

### Requirement: Selection travels with the session

The app SHALL create a new session with the picked agent and model, and SHALL switch an existing session's model or agent before sending when the pick differs from the session's.

#### Scenario: New session

- **WHEN** the user sends from a draft with an agent, model and variant picked
- **THEN** `POST /api/session` carries `agent` and `model: { providerID, id, variant }`, with `variant` left out for Default

#### Scenario: Changed pick in an open session

- **WHEN** the user picks another model, or only another variant, in an open idle session and sends
- **THEN** `POST /api/session/:id/model` completes before `POST /api/session/:id/prompt`, without `variant` when Default was picked, and a failure of either keeps the draft and shows the error

### Requirement: Enter to send

The desktop composer SHALL decide Enter as React's `shouldSubmitEnter` does for the desktop composer that is not expanded: when `enterToSendConfigured` is false or `enterToSend` is true, Enter sends and Shift+Enter or Alt+Enter inserts a newline; when `enterToSend` is false, Enter inserts a newline and Ctrl+Enter or Meta+Enter sends. Enter SHALL NOT send while an IME composition is open or while the session is busy or disconnected.

#### Scenario: Enter sends

- **WHEN** the settings leave `enterToSend` unconfigured and the user types text in an idle session and presses Enter
- **THEN** the prompt is sent, as the Send button would

#### Scenario: IME composition

- **WHEN** the user is composing text with an input method and presses Enter to commit it
- **THEN** the text is committed and nothing is sent

#### Scenario: Busy session

- **WHEN** a turn is running and the user presses Enter
- **THEN** nothing is sent and the draft stays

### Requirement: Layout parity for pickers

The multi-project sidebar SHALL be placed as the current OpenChamber desktop places it, and every popover SHALL list the same controls in the same order, checked by the parity gate against references captured from the Electron window.

#### Scenario: New states match

- **WHEN** `oc-app --dump-layout` runs for the states `projects`, `project-popover`, `model-popover`, `agent-menu` and `thinking-menu`
- **THEN** every region's bounds except the `popover` region's equal the reference, and every reference control, the popover's included, is present in the same order under the existing comparison rules
