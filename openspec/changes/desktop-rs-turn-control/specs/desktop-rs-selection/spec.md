## MODIFIED Requirements

### Requirement: Enter to send

The desktop composer SHALL decide Enter as React's `shouldSubmitEnter` does for the desktop composer that is not expanded. When `enterToSendConfigured` is false or `enterToSend` is true, Enter sends and Shift+Enter or Alt+Enter inserts a newline. When `enterToSend` is false, Enter inserts a newline and Ctrl+Enter or Meta+Enter sends. Enter SHALL NOT send while an IME composition is open, while the session is disconnected, or while the draft's first send is pending. While the session is busy, a send SHALL follow the *Send while a turn runs* requirement of `desktop-rs-turn-control`.

#### Scenario: Enter sends

- **WHEN** the settings leave `enterToSend` unconfigured and the user types text in an idle session and presses Enter
- **THEN** the prompt is sent, as the Send button would

#### Scenario: IME composition

- **WHEN** the user is composing text with an input method and presses Enter to commit it
- **THEN** the text is committed and nothing is sent

#### Scenario: Busy session

- **WHEN** a turn is running, `followUpBehavior` is `queue`, and the user presses Enter with text
- **THEN** the message is enqueued and the draft clears
