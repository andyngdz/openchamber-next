//! Keyboard handling: `mod+shift+m`, and Enter-to-send.
//!
//! The draft is gpui-base's `TextareaState`, whose `Input` key context binds `enter`,
//! `shift-enter` and the platform's secondary-enter to its own `Enter` action. GPUI dispatches
//! a bound action before key-down listeners, so the composer decides on the action in its
//! capture phase. Only combinations no binding claims reach the key-down listener.

use gpui_kit::Focusable as _;
use gpui_kit::component::input::Enter;
use gpui_kit::{App, Context, EntityInputHandler, KeyDownEvent, Modifiers, Window};
use oc_state::{ComposerStatus, ConnectionStatus, EnterKeys, SessionStatus, should_submit_enter};

use super::RootView;
use crate::views::PopoverKind;

/// The modifier state, mapped the way React reads it.
///
/// `keyboardPolicy.ts:1-26` treats `ctrlKey` and `metaKey` as separate inputs
/// (`isCtrlEnter = ctrlKey || metaKey`). GPUI's `secondary` is Cmd on macOS, so mapping it to
/// both would make Ctrl+Enter there report as `metaKey`; map the physical modifiers instead.
fn enter_keys(modifiers: Modifiers) -> EnterKeys {
    EnterKeys {
        shift: modifiers.shift,
        ctrl: modifiers.control,
        meta: modifiers.platform,
        alt: modifiers.alt,
    }
}

/// The modifiers of a bound `Enter` action. Its own fields say which binding fired, so the
/// action does not depend on the window having seen a modifiers-changed event first.
fn action_keys(action: &Enter, window: &Window, cx: &App) -> EnterKeys {
    let modifiers = window.modifiers();
    let (ctrl, meta) = match (action.secondary, cx.platform_info().is_mac()) {
        (false, _) => (modifiers.control, modifiers.platform),
        (true, true) => (modifiers.control, true),
        (true, false) => (true, modifiers.platform),
    };
    EnterKeys {
        shift: action.shift || modifiers.shift,
        ctrl,
        meta,
        alt: modifiers.alt,
    }
}

const KEY_MODEL_PICKER: &str = "m";
const KEY_ENTER: &str = "enter";

impl RootView {
    /// Handles a key before the focused control sees it.
    pub(super) fn on_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        if keystroke.key == KEY_ENTER || matches!(keystroke.key_char.as_deref(), Some("\n" | "\r"))
        {
            tracing::debug!(
                target: crate::logging::APP,
                key = %keystroke.key,
                key_char = ?keystroke.key_char,
                marked = self.marked_text(window, cx),
                "[KEYS] Enter key reached the window"
            );
        }
        let secondary = keystroke.modifiers.secondary(cx);
        if secondary
            && keystroke.modifiers.shift
            && keystroke.key.eq_ignore_ascii_case(KEY_MODEL_PICKER)
        {
            self.open_popover(PopoverKind::Model, window, cx);
            window.prevent_default();
            cx.stop_propagation();
            return;
        }
        // An open popup owns its keys; its component handles navigation, Enter and Escape.
        if self.popover.is_none() {
            self.composer_key(event, window, cx);
        }
    }

    /// The composer's Enter action, before the textarea inserts a newline.
    pub(super) fn composer_enter(
        &mut self,
        action: &Enter,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        tracing::debug!(
            target: crate::logging::APP,
            shift = action.shift,
            secondary = action.secondary,
            popover = self.popover.is_some(),
            marked = self.marked_text(window, cx),
            "[KEYS] Composer Enter action"
        );
        if self.popover.is_some() || self.marked_text(window, cx) {
            return;
        }
        let keys = action_keys(action, window, cx);
        if self.enter_sends(keys, cx) && self.can_send(cx) {
            self.send_prompt(cx);
            cx.stop_propagation();
        }
    }

    /// Enter combinations no key binding claims, such as Alt+Enter.
    fn composer_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key != KEY_ENTER || self.marked_text(window, cx) {
            return;
        }
        let keys = enter_keys(event.keystroke.modifiers);
        if self.enter_sends(keys, cx) && self.can_send(cx) {
            self.send_prompt(cx);
        } else {
            // Let the textarea's own handler insert the newline for this combination.
            window.dispatch_action(
                Box::new(Enter {
                    secondary: false,
                    shift: true,
                }),
                cx,
            );
        }
        window.prevent_default();
        cx.stop_propagation();
    }

    /// Whether React's composer policy sends for these modifiers.
    fn enter_sends(&self, keys: EnterKeys, cx: &App) -> bool {
        let settings = self.state.read(cx).settings().cloned();
        settings.map_or(
            !keys.shift && (!keys.alt || keys.ctrl || keys.meta),
            |settings| should_submit_enter(&settings, keys),
        )
    }

    /// The checks `send_prompt` makes before it builds a request.
    fn can_send(&self, cx: &App) -> bool {
        let state = self.state.read(cx);
        let composer = state.composer();
        state.connection() == &ConnectionStatus::Connected
            && composer.status != ComposerStatus::Sending
            && !composer.draft.trim().is_empty()
            && (state.open().is_none() || state.open_status() != SessionStatus::Busy)
    }

    /// Whether the composer's input has marked (composing) text.
    fn marked_text(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.draft.update(cx, |draft, cx| {
            draft.marked_text_range(window, cx).is_some()
        })
    }

    /// Opens a picker, clears its search and focuses it.
    pub(super) fn open_popover(
        &mut self,
        kind: PopoverKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.popover = Some(kind);
        self.cursor = 0;
        match kind {
            PopoverKind::Project => self
                .lists
                .project
                .update(cx, |list, cx| list.focus(window, cx)),
            PopoverKind::Model => crate::views::open_model_picker(&self.lists, window, cx),
            PopoverKind::Agent => self
                .lists
                .agent
                .update(cx, |list, cx| list.focus(window, cx)),
            PopoverKind::Thinking => {
                let rows = crate::views::thinking_rows(self.state.read(cx));
                let dispatch = self.dispatch.clone();
                let registry = self.registry.clone();
                self.lists.thinking.update(cx, |menu, cx| {
                    menu.rebuild(window, cx, |menu, _, _| {
                        crate::views::thinking_items(menu, rows.clone(), &dispatch, &registry)
                    })
                });
                let handle = self.lists.thinking.read(cx).focus_handle(cx);
                handle.focus(window, cx);
            }
        }
        cx.notify();
    }

    /// Closes the picker and puts focus back in the composer.
    pub(super) fn close_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.popover = None;
        self.cursor = 0;
        self.draft.update(cx, |draft, cx| draft.focus(window, cx));
        cx.notify();
    }
}

#[cfg(test)]
#[path = "keys_test.rs"]
mod keys_test;
