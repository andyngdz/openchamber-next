//! The six regions of the window, drawn from `oc-state`'s [`State`], and the open picker popup.
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
mod model_names;
mod popover;
mod sidebar;
mod sidebar_list;
mod text_metrics;
mod tool_rows;
mod turn_stats;
mod turns;
mod work_status;

#[cfg(test)]
mod icons_test;
#[cfg(test)]
mod reference_test;

use std::rc::Rc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{App, ClickEvent, Entity, FontWeight, SharedString, TextRun, Window, px};
use oc_state::{Action, SidebarAction, State};

pub(crate) use chat::chat_column;
pub(crate) use composer::PLACEHOLDER;
pub(crate) use header::header;
pub(crate) use icon_rail::icon_rail;
#[cfg(test)]
pub(crate) use popover::trigger_spec;
pub(crate) use popover::{Pick, PopoverKind, PopoverLists};
pub(crate) use popover::{filter_model_list, model_search_field};
pub(crate) use popover::{open_model_picker, thinking_items, thinking_rows};
pub(crate) use sidebar::sidebar;
pub(crate) use work_status::work_status;

use crate::icons::RemixIcon;
use crate::layout::{ControlSpec, LayoutRegistry};

/// What a control asks the root view to do.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Intent {
    /// Apply this action; opening a session or retrying a region also sends its request.
    Apply(Action),
    /// Apply one sidebar input, such as a collapse or Show more click.
    ApplySidebar(SidebarAction),
    /// Apply one picker change that only edits the selection or settings.
    ApplySelection(oc_state::SelectionAction),
    /// Create a session in the directory and open it.
    NewSession,
    /// Send the composer text to the open session.
    SendPrompt,
    /// Stop the server if it still runs and start it again.
    RestartServer,
    /// Open one picker popup.
    OpenPopover(PopoverKind),
    /// Close the open popup and return focus to the composer.
    ClosePopover,
    /// Pick the popup row under the keyboard cursor.
    Pick(Pick),
    /// Load a directory's catalog again after a failure.
    LoadCatalog(String),
}

/// Hands an intent to the root view.
pub(crate) type Dispatch = Rc<dyn Fn(&Intent, &mut Window, &mut App)>;

/// What every region reads while it renders.
#[derive(Clone, Copy)]
pub(crate) struct Scene<'a> {
    pub(crate) state: &'a State,
    pub(crate) registry: &'a LayoutRegistry,
    pub(crate) dispatch: &'a Dispatch,
    /// The open picker popup, if any.
    pub(crate) popover: Option<PopoverKind>,
    /// The shared state entity the popups read.
    pub(crate) state_entity: &'a Entity<State>,
    /// The popup lists and menu, owned by the root view.
    pub(crate) lists: &'a PopoverLists,
}

impl Scene<'_> {
    /// The shared state entity, cloned for a popup builder.
    fn state_entity(&self) -> Entity<State> {
        self.state_entity.clone()
    }

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
fn icon_button(spec: &ControlSpec, icon: RemixIcon) -> Button {
    Button::new(spec.key.clone())
        .ghost()
        .small()
        .icon(Icon::new(icon))
        .tooltip(spec.name.clone())
}

/// A ghost button with a visible label and an optional leading icon.
fn label_button(
    spec: &ControlSpec,
    icon: Option<RemixIcon>,
    label: impl Into<SharedString>,
) -> Button {
    let button = Button::new(spec.key.clone()).ghost().small().label(label);
    match icon {
        Some(icon) => button.icon(Icon::new(icon)),
        None => button,
    }
}

pub(crate) const RETRY: &str = "Retry";

/// The shaped line width of `text` in the theme's family at `size`/`weight`.
///
/// The composer labels use `typography-meta` (`typography.css:26-29`, 12.5px) and
/// `font-medium` (`ModelControls.tsx:2358,2739,2855`); the Thinking footer uses
/// `typography-micro` (`typography.css:31-34`, 12px). The capture's computed styles for
/// these labels are `letter-spacing: normal`, `font-feature-settings: normal`,
/// `font-kerning: auto` and `text-rendering: auto`, so family, size and weight are the
/// shaping inputs.
pub(crate) fn line_width(window: &mut Window, cx: &App, text: &str, size: f32, weight: f32) -> f32 {
    let mut face = gpui_kit::font(cx.theme().font_family.clone());
    face.weight = FontWeight(weight);
    let width = window
        .text_system()
        .shape_line(
            text.to_owned().into(),
            px(size),
            &[TextRun {
                len: text.len(),
                font: face,
                color: cx.theme().foreground,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        )
        .width;
    f32::from(width)
}

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
