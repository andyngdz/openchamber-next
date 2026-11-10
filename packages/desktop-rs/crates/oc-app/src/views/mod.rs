//! The six regions of the window, drawn from `oc-state`'s [`State`].
//!
//! Each region file keeps its control list as data, in the order the reference lists it, and
//! a render function that walks that list; the parity test compares the result with
//! `fixtures/reference/`.

mod chat;
mod composer;
mod empty_state;
mod header;
mod icon_rail;
mod messages;
mod sidebar;
mod sidebar_list;
mod sidebar_model;
mod tool_rows;
mod turn_stats;
mod turns;
mod work_status;

#[cfg(test)]
mod icons_test;
#[cfg(test)]
mod reference_test;

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{App, ClickEvent, SharedString, Window};
use oc_state::{Action, State};

pub(crate) use chat::chat_column;
pub(crate) use composer::PLACEHOLDER;
pub(crate) use header::header;
pub(crate) use icon_rail::icon_rail;
pub(crate) use sidebar::sidebar;
pub(crate) use work_status::work_status;

use crate::layout::{ControlSpec, LayoutRegistry};

/// What a control asks the root view to do.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Intent {
    /// Apply this action; opening a session or retrying a region also sends its request.
    Apply(Action),
    /// Create a session in the directory and open it.
    NewSession,
    /// Send the composer text to the open session.
    SendPrompt,
    /// Stop the server if it still runs and start it again.
    RestartServer,
}

/// Hands an intent to the root view.
pub(crate) type Dispatch = Rc<dyn Fn(&Intent, &mut Window, &mut App)>;

/// What every region reads while it renders.
#[derive(Clone, Copy)]
pub(crate) struct Scene<'a> {
    pub(crate) state: &'a State,
    pub(crate) registry: &'a LayoutRegistry,
    pub(crate) dispatch: &'a Dispatch,
}

impl Scene<'_> {
    /// A click handler that applies `action`.
    fn on_click_dispatch(
        &self,
        action: Action,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        self.on_click_intent(Intent::Apply(action))
    }

    /// A click handler that dispatches `intent`.
    fn on_click_intent(
        &self,
        intent: Intent,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let dispatch = self.dispatch.clone();
        move |_, window, cx| dispatch(&intent, window, cx)
    }
}

/// A ghost icon button; the accessible name is its tooltip. Without a click handler it is inert.
fn icon_button(spec: &ControlSpec, icon: IconName) -> Button {
    Button::new(spec.key.clone())
        .ghost()
        .small()
        .icon(Icon::new(icon))
        .tooltip(spec.name.clone())
}

/// A ghost button with a visible label and an optional leading icon.
fn label_button(
    spec: &ControlSpec,
    icon: Option<IconName>,
    label: impl Into<SharedString>,
) -> Button {
    let button = Button::new(spec.key.clone()).ghost().small().label(label);
    match icon {
        Some(icon) => button.icon(Icon::new(icon)),
        None => button,
    }
}

const RETRY: &str = "Retry";

/// The Retry button a region shows under its error; the caller wires what it retries.
fn retry_button(id: &'static str) -> Button {
    Button::new(id).small().label(RETRY)
}

/// The project name the sidebar and the Work status panel show: the directory's last part.
fn project_name(state: &State) -> &str {
    state
        .directory()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
}
