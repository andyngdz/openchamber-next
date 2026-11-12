## ADDED Requirements

### Requirement: Permission dock

The app SHALL show, above the composer, the pending permission requests of the open session and of every descendant session. Requests SHALL be ordered root first, then descendants breadth-first, and by `id` within a session. One request SHALL be shown at a time, with `{current} of {total}`, a dot per request, and a preview of what the tool will do.

#### Scenario: Subagent request

- **WHEN** a subagent of the open session asks for a permission
- **THEN** the dock shows it after the open session's own requests, marked `from a subagent`

#### Scenario: Edit preview

- **WHEN** a request for the edit tool carries `metadata.files[].patch`
- **THEN** each patch is shown as a highlighted diff inside a scrolling box

### Requirement: Permission replies

The dock SHALL offer Deny, Always when `save` is not empty, and Allow once, also reached by Alt+Backspace, Alt+Shift+Enter and Alt+Enter. A reply SHALL send `POST /api/session/:id/permission/:requestId/reply` with the decision `reject`, `always` or `once`. The request SHALL stay on screen until `permission.replied` arrives or the reply answers 404.

#### Scenario: Allow a command once

- **WHEN** an agent in the open session asks to run a bash command and the user presses Alt+Enter
- **THEN** the dock showed the command highlighted as bash, the reply is sent with decision `once`, and the card disappears when `permission.replied` arrives

#### Scenario: Reply fails

- **WHEN** a reply fails with an error other than 404
- **THEN** the card stays, its buttons are enabled again, and the error is logged

### Requirement: Form dock

The app SHALL show the first pending form of the open session's tree, then the location's `global` forms, one answerable field per step, with `{count} more waiting` for the rest. The dock SHALL be hidden while a permission is pending in that tree. Submit SHALL send `POST /api/session/:id/form/:formId/reply` with the active fields' answers, and Dismiss SHALL send `DELETE /api/session/:id/form/:formId`.

#### Scenario: Permission wins

- **WHEN** a form and a permission are pending in the open session's tree
- **THEN** only the permission dock shows, and the form shows once the permission is replied to

#### Scenario: Answer a question

- **WHEN** an agent asks a question with three options and the user picks one and presses Ctrl+Enter
- **THEN** the reply carries that option's value under the field's key and the dock closes

### Requirement: Form field types

The form dock SHALL support every `FormField` type: string with or without options, multiselect, boolean, number, integer and external. It SHALL honor `required`, `hidden`, `when`, defaults and `custom`, and SHALL refuse a number or integer field whose text does not parse. Bounds, `minItems`, `maxItems`, `pattern` and `format` SHALL be left to the server, whose rejection shows `Could not submit the answer`, as in React.

#### Scenario: External link step

- **WHEN** a form has an external field the user has not opened
- **THEN** Submit is refused on that step until the link has been opened

#### Scenario: Conditional field

- **WHEN** a field's `when` clause compares against an earlier field that is unanswered
- **THEN** the field is skipped and its answer is not sent

### Requirement: Web search consent card

A form whose `metadata.kind` is `websearch.provider` and whose only field is a string keyed `choice` or `provider` with options SHALL render as the consent card instead of the stepper. The `choice` step SHALL offer Allow, Choose provider and Turn off web search, and the `provider` step one button per option. Each button SHALL reply with that value, and Not now SHALL cancel the form.

#### Scenario: Allow web search

- **WHEN** the consent card asks for the `choice` step and the user clicks Allow
- **THEN** the form reply carries `{ "choice": "allow" }`

### Requirement: Recovered question

The app SHALL offer again a question form linked to a tool part whose state is an error that marks a turn death. The offer SHALL appear when no live form is pending, with the warning that Submit starts a new paid turn. Submit SHALL send the answers as a markdown prompt in a new turn, using the linked message's model, agent and variant. Dismiss SHALL send nothing.

#### Scenario: Turn stopped while asking

- **WHEN** the user stops a turn while its question form is pending, and the question's tool part is still running when the interrupt arrives
- **THEN** the part is marked `Interrupted` locally, the question is shown again with the warning, and submitting it starts a new turn whose text holds the answers

#### Scenario: Server reports a dismissed question

- **WHEN** a question's tool part ends in an error whose `errorType` is set and is not `aborted`
- **THEN** the question is not offered again

### Requirement: Stop a running turn

While the open session is not idle, the composer SHALL show Stop instead of Send. Stop, or Escape pressed twice within 3000 ms in the composer editor for the same session, SHALL send `POST /api/session/:id/interrupt`. Escape SHALL be ignored while a popup is open, during IME composition, and while the session is idle. The session status SHALL change only from the server's events.

#### Scenario: Escape twice

- **WHEN** a turn runs and the user presses Escape twice within 3 seconds in the composer
- **THEN** the interrupt is sent once, and the composer shows Send again after the session's idle event

#### Scenario: Escape once

- **WHEN** a turn runs and the user presses Escape once and waits more than 3 seconds
- **THEN** nothing is sent

### Requirement: Permission auto-accept mode

The composer's shield SHALL show the open session's effective mode: `ask`, `safety` or `auto`, resolved through the nearest ancestor with an explicit mode. A click SHALL set the next mode with `PUT /api/permission-auto-accept/sessions/:id`, skipping `safety` when the routing state reports it unavailable. The mode SHALL change on screen only from the server's snapshot.

#### Scenario: Cycle to auto

- **WHEN** the safety net is unavailable, the mode is ask, and the user clicks the shield
- **THEN** the PUT carries mode `auto`, and the shield shows the accept-everything icon after the update arrives

### Requirement: Requests the server answers

A fresh `permission.asked` SHALL stay out of the dock when, as it arrives, the session's effective mode is `safety` or `auto`. It SHALL appear once the server reports it left for the user, including a report that arrived before the request, and SHALL be dropped when it is replied to. A request loaded from the pending list SHALL always show, and changing the mode SHALL NOT hide a request already shown. A request the safety net holds SHALL show that it was held and, when known, the kind of action.

#### Scenario: Safety holds a request

- **WHEN** a session is in `safety` and the server reports a request left for the user
- **THEN** that request appears in the dock, opening with `The safety net held this action for you to decide`

#### Scenario: Reconnect while in auto

- **WHEN** a session is in `auto` and the pending list after a reconnect returns one of its requests
- **THEN** that request shows in the dock

### Requirement: Send while a turn runs

With the session busy and text present, the plain send key SHALL enqueue the message when `followUpBehavior` is `queue` (the default) and steer it when it is `steer`. Ctrl+Enter or Meta+Enter SHALL send a plain prompt, and when `enterToSend` is false it is the only send key, so the keyboard never enqueues or steers. The app SHALL NOT dispatch queued messages itself.

#### Scenario: Queue during a turn

- **WHEN** a turn runs, `followUpBehavior` is unset, and the user types and presses Enter
- **THEN** the message is posted to the session's queue with the composer's model, agent and variant, the composer clears, and the queue strip shows it

### Requirement: Sending dismisses pending requests

A plain prompt or a steer sent from the composer SHALL first reject the open session tree's pending permissions and cancel its pending forms, removing them from the dock at once and restoring any whose dismissal fails with an error other than 404. Enqueueing and a queue row's send now SHALL dismiss nothing.

#### Scenario: Dismissal fails

- **WHEN** a send dismisses a pending form and the cancel fails with a server error
- **THEN** the form comes back in the dock, and the error is logged

#### Scenario: Send now keeps a pending permission

- **WHEN** a permission is pending and the user clicks send now on a queued row
- **THEN** the row is steered into the turn and the permission stays in the dock

### Requirement: A dismissing send is enqueued

When the dismissal step removed anything and the send is not a steer, the send SHALL stop there. The composer text SHALL be enqueued, an empty composer SHALL enqueue nothing, and the queue SHALL stay as it is.

#### Scenario: Send dismisses a pending permission

- **WHEN** a permission is pending and the user presses Ctrl+Enter with text
- **THEN** the permission is rejected, and the message is enqueued

#### Scenario: Pending permission with an empty composer

- **WHEN** a permission is pending, two messages are queued, the composer is empty, and the user presses Enter
- **THEN** the permission is rejected, nothing is taken or sent, and both items stay queued

### Requirement: A send carries the queue

Every prompt or steer from the composer that goes ahead after the dismissal step SHALL take the whole queue and send the queued texts, oldest first, then the composer text, joined by blank lines. An empty composer with queued items SHALL send the queue this way, busy or idle. A failed take SHALL send nothing.

#### Scenario: Empty composer flushes the queue

- **WHEN** two messages are queued, the composer is empty, and the user presses Enter
- **THEN** both items are taken and sent as one prompt, the older text first, separated by a blank line

### Requirement: Queue strip

The app SHALL show the open session's queue above the editor, from the server's snapshots, skipping a snapshot only when its revision is older than the one held for that session. One item SHALL show as a row, and more SHALL collapse under `Queued messages {n}`.

#### Scenario: Older snapshot

- **WHEN** a snapshot arrives with a revision older than the one shown
- **THEN** the strip keeps what it shows

### Requirement: Queue row actions

A queued row SHALL offer:

- edit, which takes the item and puts its text back in the composer;
- send now, which takes that item and steers it, or sends it as a plain prompt when the session is idle;
- remove;
- drag to reorder, which sends the complete new order.

An item that is sending SHALL offer no edit or remove.

#### Scenario: Reorder

- **WHEN** the user drags the third queued row above the first
- **THEN** `PUT /api/message-queue/sessions/:id/order` carries all three ids in the new order, and the strip follows the next snapshot

### Requirement: Control stream

The app SHALL read `/api/openchamber/events` alongside the event stream, and apply the message queue, auto-accept and routing updates it carries. On every ready frame it SHALL re-read the queue, the auto-accept snapshot and the routing state. It SHALL reconnect with backoff from 1 to 30 seconds, and treat 45 seconds without a frame as a drop.

#### Scenario: Control stream drops

- **WHEN** the control stream drops and comes back
- **THEN** the queue strip and the shield keep their last state while it is down, and are replaced by the re-read snapshots after the ready frame

### Requirement: Highlighted previews

The bash command and the diffs in the permission dock SHALL be highlighted with gpui-kit's tree-sitter highlighter, colored from the syntax colors of the Electron default dark theme, `openchamber-dark`, by the token rules of React's `zedThemeFromAppTheme`. A language without a grammar SHALL show unhighlighted.

#### Scenario: Theme colors

- **WHEN** the dock shows a Rust diff
- **THEN** a keyword is painted with `openchamber-dark`'s `syntax.base.keyword` color

### Requirement: Icon and indicator parity

Every control the parity gate compares SHALL also match the Electron reference in the icons it draws, by Remix symbol name and color in order, in its color marks (dots and other small textless colored shapes), and in the color of each run of its text, on every captured state. Colors SHALL match within 1 per channel.

#### Scenario: Agent menu dots

- **WHEN** the agent menu is open with Build selected
- **THEN** Build and Plan each show the agent's color dot, Build shows the check icon, and the parity test fails if either is missing or differs in color

#### Scenario: Wrong icon

- **WHEN** a view draws a different icon than React for a control
- **THEN** the parity test fails and names the control, the expected icon and the drawn one

### Requirement: Remix icons and the Electron palette

The app SHALL draw icons from the same Remix set as React's `Icon`, including the icons drawn inside gpui-kit components, and SHALL paint the window palette of the Electron default dark theme, `openchamber-dark`.

#### Scenario: A gpui-kit component's icon

- **WHEN** the model popover's search field is drawn
- **THEN** its search icon is the Remix icon React draws there, and no gpui-kit icon file is loaded

