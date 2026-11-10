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
mod tool_rows;
mod turn_stats;
mod turns;
mod work_status;

#[cfg(test)]
mod icons_test;
#[cfg(test)]
mod reference_test;

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::InputState;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{
    App, Bounds, ClickEvent, Entity, FontWeight, Pixels, SharedString, TextRun, Window, px,
};
use oc_state::{Action, SidebarAction, State};

pub(crate) use chat::chat_column;
pub(crate) use composer::PLACEHOLDER;
pub(crate) use header::header;
pub(crate) use icon_rail::icon_rail;
pub(crate) use popover::popover as popup;
pub(crate) use popover::{Pick, PopoverKind, selectables};
pub(crate) use sidebar::sidebar;
pub(crate) use work_status::work_status;

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
    /// The popup search fields.
    pub(crate) search: &'a PopoverInputs,
    /// The bounds of every popup's trigger, from the previous frame.
    pub(crate) triggers: &'a TriggerBounds,
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

/// The search field of each popup that has one.
#[derive(Clone)]
pub(crate) struct PopoverInputs {
    pub(crate) project: Entity<InputState>,
    pub(crate) model: Entity<InputState>,
    pub(crate) agent: Entity<InputState>,
}

impl PopoverInputs {
    /// The search field of one popup, if it has one.
    pub(crate) fn for_kind(&self, kind: PopoverKind) -> Option<&Entity<InputState>> {
        match kind {
            PopoverKind::Project => Some(&self.project),
            PopoverKind::Model => Some(&self.model),
            PopoverKind::Agent => Some(&self.agent),
            PopoverKind::Thinking => None,
        }
    }
}

/// The bounds of each popup trigger, recorded while the frame is prepainted.
#[derive(Clone, Default)]
pub(crate) struct TriggerBounds {
    pub(crate) project: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub(crate) model: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub(crate) agent: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub(crate) thinking: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl TriggerBounds {
    /// The cell of one popup's trigger.
    pub(crate) fn for_kind(&self, kind: PopoverKind) -> &Rc<Cell<Option<Bounds<Pixels>>>> {
        match kind {
            PopoverKind::Project => &self.project,
            PopoverKind::Model => &self.model,
            PopoverKind::Agent => &self.agent,
            PopoverKind::Thinking => &self.thinking,
        }
    }

    /// The trigger's bounds from the last frame, if it was drawn.
    pub(crate) fn get(&self, kind: PopoverKind) -> Option<Bounds<Pixels>> {
        self.for_kind(kind).get()
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
