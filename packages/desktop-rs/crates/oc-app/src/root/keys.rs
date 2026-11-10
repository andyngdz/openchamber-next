//! Keyboard handling: picker navigation, `mod+shift+m`, and Enter-to-send.

use gpui_kit::{App, Context, EntityInputHandler, KeyDownEvent, Window};
use oc_state::{EnterKeys, should_submit_enter};

use super::RootView;
use crate::views::PopoverKind;

const KEY_MODEL_PICKER: &str = "m";
const KEY_ENTER: &str = "enter";

/// A popup key, parsed once at the keyboard boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PopupKey {
    Escape,
    Enter,
    Up,
    Down,
    Next,
    Previous,
}

/// Whether the secondary modifier is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Secondary {
    Held,
    NotHeld,
}

/// Which way the popup cursor moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Forward,
    Backward,
}

impl RootView {
    /// Handles a key before the focused control sees it.
    pub(super) fn on_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
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
        match self.popover {
            Some(kind) => self.popup_key(kind, event, window, cx),
            None => self.composer_key(event, window, cx),
        }
    }

    fn popup_key(
        &mut self,
        kind: PopoverKind,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let held = event.keystroke.modifiers.secondary(cx);
        let secondary = if held {
            Secondary::Held
        } else {
            Secondary::NotHeld
        };
        let Some(key) = PopupKey::parse(event.keystroke.key.as_str(), secondary) else {
            return;
        };
        let query = self.query(kind, cx);
        let picks = crate::views::selectables(self.state.read(cx), kind, &query);
        match key {
            PopupKey::Escape => self.close_popover(window, cx),
            PopupKey::Enter => {
                if let Some(pick) = picks.get(self.cursor).cloned() {
                    self.pick(pick, window, cx);
                }
            }
            PopupKey::Up => self.move_cursor(picks.len(), Direction::Backward, cx),
            PopupKey::Down => self.move_cursor(picks.len(), Direction::Forward, cx),
            PopupKey::Next => self.move_cursor(picks.len(), Direction::Forward, cx),
            PopupKey::Previous => self.move_cursor(picks.len(), Direction::Backward, cx),
        }
        window.prevent_default();
        cx.stop_propagation();
    }

    fn composer_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key != KEY_ENTER || self.marked_text(window, cx) {
            return;
        }
        let settings = self.state.read(cx).settings().cloned();
        let keys = EnterKeys {
            shift: event.keystroke.modifiers.shift,
            ctrl: event.keystroke.modifiers.secondary(cx),
            meta: false,
            alt: event.keystroke.modifiers.alt,
        };
        let send = settings.map_or(!keys.shift, |settings| should_submit_enter(&settings, keys));
        if send {
            self.send_prompt(cx);
            window.prevent_default();
            cx.stop_propagation();
        }
    }

    /// Whether the composer's input has marked (composing) text.
    fn marked_text(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.draft.update(cx, |draft, cx| {
            draft.marked_text_range(window, cx).is_some()
        })
    }

    fn query(&self, kind: PopoverKind, cx: &App) -> String {
        self.search
            .for_kind(kind)
            .map(|input| input.read(cx).value().to_string())
            .unwrap_or_default()
    }

    fn move_cursor(&mut self, count: usize, direction: Direction, cx: &mut Context<Self>) {
        if count == 0 {
            return;
        }
        self.cursor = match direction {
            Direction::Forward => (self.cursor + 1) % count,
            Direction::Backward => (self.cursor + count - 1) % count,
        };
        cx.notify();
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
        if let Some(input) = self.search.for_kind(kind) {
            input.update(cx, |state, cx| {
                state.set_value("", window, cx);
                state.focus(window, cx);
            });
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

impl PopupKey {
    fn parse(key: &str, secondary: Secondary) -> Option<Self> {
        match key {
            "escape" => Some(Self::Escape),
            KEY_ENTER => Some(Self::Enter),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "n" if secondary == Secondary::Held => Some(Self::Next),
            "p" if secondary == Secondary::Held => Some(Self::Previous),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "keys_test.rs"]
mod keys_test;
