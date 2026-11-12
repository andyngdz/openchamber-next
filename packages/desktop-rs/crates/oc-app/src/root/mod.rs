//! The window's root view: holds the [`State`] entity and applies every action through
//! [`State::apply`], so a fixture replay and live events take the same path. With a live
//! server it also turns intents into requests and applies what the server sends back.

#[cfg(test)]
#[path = "chrome_test.rs"]
mod chrome_test;
mod keys;
mod live;
#[cfg(test)]
#[path = "overlay_test.rs"]
mod overlay_test;
mod picks;
mod plan;
mod requests;
mod updates;

pub(crate) use live::Started;

use std::collections::BTreeSet;
use std::rc::Rc;

use gpui_kit::component::input::{Enter, InputEvent, TextareaState};
use gpui_kit::component::{ActiveTheme, window_border, window_corner_radii};
use gpui_kit::{
    AppContext, Context, DismissEvent, Entity, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, ScrollHandle, Styled, Subscription, Task, Window, div, px,
};
use oc_state::{Action, State};

use crate::layout::{LayoutRegistry, content};
use crate::live::ServerLink;
use crate::theme::{DepthStyle, Tokens};
use crate::views::{
    Dispatch, PLACEHOLDER, PopoverKind, PopoverLists, STRIP_HEIGHT, Scene, chat_column,
    filter_model_list, header, icon_rail, model_search_field, panel_header, sidebar,
    window_controls, work_status,
};

/// Gap above and below the chat card.
const CARD_INSET: f32 = 8.;
/// Width of the right-hand zone, which holds the Work status panel.
const PANEL_WIDTH: f32 = 320.;
/// Title of the right-hand zone's header row.
const PANEL_TITLE: &str = "Session";
/// Width of the icon rail, and its gap from the panel.
const RAIL_WIDTH: f32 = 44.;
const RAIL_GAP: f32 = 8.;

/// The whole window.
pub(crate) struct RootView {
    state: Entity<State>,
    registry: LayoutRegistry,
    draft: Entity<TextareaState>,
    scroll: ScrollHandle,
    dispatch: Dispatch,
    /// The open picker popup, if any.
    pub(super) popover: Option<PopoverKind>,
    /// The popup row the keyboard is on.
    pub(super) cursor: usize,
    /// The lists and menu the popovers render.
    pub(super) lists: PopoverLists,
    /// The live server; `None` for a fixture, whose server-bound controls do nothing.
    pub(super) link: Option<ServerLink>,
    /// Waits for the server thread's answer to the latest start; a new start replaces it.
    pub(super) starting: Option<Task<()>>,
    /// Waits for the server thread's answer to the latest exit check; a start drops it, so an
    /// answer about the old server never shows over the new start.
    pub(super) exit_check: Option<Task<()>>,
    /// The first session list after a start records OpenCode's pids: listing goes through
    /// OpenCode, so it runs by then, even if the check at connect caught it before `exec`.
    pub(super) record_pids_on_list: bool,
    /// Directories whose catalogs the window asked for; a reconnect refetches them.
    pub(super) catalogs: BTreeSet<String>,
    _draft_events: Subscription,
    _thinking_dismiss: Subscription,
    _model_search: Subscription,
}

impl RootView {
    /// A window over `state`, showing `popover`; the registry receives every frame's bounds.
    pub(crate) fn new(
        state: State,
        popover: Option<PopoverKind>,
        registry: LayoutRegistry,
        link: Option<ServerLink>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let draft = cx.new(|cx| TextareaState::new(window, cx).placeholder(PLACEHOLDER));
        let draft_events =
            cx.subscribe_in(&draft, window, |this, draft, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = draft.read(cx).value().to_string();
                    this.dispatch(Action::DraftChanged(text), cx);
                }
            });
        let state = cx.new(|_| state);
        let root = cx.entity().downgrade();
        let dispatch: Dispatch = Rc::new(move |intent, window, cx| {
            root.update(cx, |this, cx| this.handle(intent.clone(), window, cx))
                .ok();
        });
        let lists = PopoverLists::new(&state, dispatch.clone(), registry.clone(), window, cx);
        let thinking_dismiss = cx.subscribe_in(
            &lists.thinking,
            window,
            |this, _, _: &DismissEvent, window, cx| this.close_popover(window, cx),
        );
        // The model panel draws its own search field; its edits filter that list, and the
        // field's keys (Enter included) are handled by the panel.
        let model_search = model_search_field(&lists, cx);
        let model_search_events = cx.subscribe_in(
            &model_search,
            window,
            |this, search, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    filter_model_list(&this.lists, search, window, cx);
                }
            },
        );
        Self {
            state,
            registry,
            draft,
            scroll: ScrollHandle::new(),
            dispatch,
            popover,
            cursor: 0,
            lists,
            link,
            starting: None,
            exit_check: None,
            record_pids_on_list: false,
            catalogs: BTreeSet::new(),
            _draft_events: draft_events,
            _thinking_dismiss: thinking_dismiss,
            _model_search: model_search_events,
        }
    }

    /// Applies one action; the message list follows new content while it sits at the bottom.
    pub(crate) fn dispatch(&mut self, action: Action, cx: &mut Context<Self>) {
        self.state.update(cx, |state, _| state.apply(action));
        if self.scroll.offset().y <= -self.scroll.max_offset().y + px(1.) {
            self.scroll.scroll_to_bottom();
        }
        self.after_change(cx);
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.registry.begin_frame();
        let state = self.state.read(cx);
        let scene = Scene {
            state,
            registry: &self.registry,
            dispatch: &self.dispatch,
            popover: self.popover,
            state_entity: &self.state,
            lists: &self.lists,
            palette: crate::theme::palette(cx),
        };
        let theme = cx.theme();
        let tokens = *Tokens::of(cx);
        let radii = window_corner_radii(window, cx);
        let card = div()
            .flex_1()
            .min_w_0()
            .my(px(CARD_INSET))
            .flex()
            .flex_col()
            .overflow_hidden()
            .raised_surface(&tokens)
            .child(header(scene, cx))
            .child(chat_column(scene, &self.draft, &self.scroll, window, cx));
        let panel = div()
            .w(px(PANEL_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .child(panel_header(PANEL_TITLE, cx))
            .child(work_status(scene, cx));
        let rail = div()
            .w(px(RAIL_WIDTH))
            .h_full()
            .flex_none()
            .ml(px(RAIL_GAP))
            .pt(px(STRIP_HEIGHT))
            .child(icon_rail(&self.registry, cx));
        let root = div()
            .size_full()
            .flex()
            .overflow_hidden()
            .rounded_tl(radii.top_left)
            .rounded_tr(radii.top_right)
            .rounded_bl(radii.bottom_left)
            .rounded_br(radii.bottom_right)
            .bg(tokens.depth.glass)
            .text_color(theme.foreground)
            .child(sidebar(scene, cx))
            .child(card)
            .child(panel)
            .child(rail)
            .child(window_controls(scene, cx));
        let window_content =
            content(&self.registry, root)
                .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                    this.composer_enter(action, window, cx)
                }))
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    this.on_key(event, window, cx)
                }));
        window_border().shadow_size(px(0.)).child(window_content)
    }
}
