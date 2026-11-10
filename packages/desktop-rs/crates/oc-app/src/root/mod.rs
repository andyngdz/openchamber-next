//! The window's root view: holds the [`State`] entity and applies every action through
//! [`State::apply`], so a fixture replay and live events take the same path. With a live
//! server it also turns intents into requests and applies what the server sends back.

mod keys;
mod live;
mod picks;
mod plan;
mod requests;
mod updates;

pub(crate) use live::Started;

use std::collections::BTreeSet;
use std::rc::Rc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::{
    AppContext, Context, Entity, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Render, ScrollHandle, Styled, Subscription, Task, Window, div, px,
};
use oc_state::{Action, State};

use crate::layout::{LayoutRegistry, content};
use crate::live::ServerLink;
use crate::views::{
    Dispatch, PLACEHOLDER, PopoverInputs, PopoverKind, Scene, TriggerBounds, chat_column, header,
    icon_rail, popup, sidebar, work_status,
};

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
    pub(super) search: PopoverInputs,
    pub(super) triggers: TriggerBounds,
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
    _search_events: Vec<Subscription>,
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
        let project = cx.new(|cx| InputState::new(window, cx).placeholder("Search projects..."));
        let model = cx.new(|cx| InputState::new(window, cx).placeholder("Search models"));
        let agent = cx.new(|cx| InputState::new(window, cx).placeholder("Search agents"));
        let mut search_events = Vec::new();
        for input in [&project, &model, &agent] {
            search_events.push(cx.subscribe_in(
                input,
                window,
                |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.cursor = 0;
                        cx.notify();
                    }
                },
            ));
        }
        let root = cx.entity().downgrade();
        let dispatch: Dispatch = Rc::new(move |intent, window, cx| {
            root.update(cx, |this, cx| this.handle(intent.clone(), window, cx))
                .ok();
        });
        Self {
            state: cx.new(|_| state),
            registry,
            draft,
            scroll: ScrollHandle::new(),
            dispatch,
            popover,
            cursor: 0,
            search: PopoverInputs {
                project,
                model,
                agent,
            },
            triggers: TriggerBounds::default(),
            link,
            starting: None,
            exit_check: None,
            record_pids_on_list: false,
            catalogs: BTreeSet::new(),
            _draft_events: draft_events,
            _search_events: search_events,
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
            search: &self.search,
            triggers: &self.triggers,
        };
        let theme = cx.theme();
        let body = div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(chat_column(scene, &self.draft, &self.scroll, window, cx))
            .child(work_status(scene, cx))
            .child(icon_rail(&self.registry, cx));
        let mut root = div()
            .size_full()
            .flex()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(sidebar(scene, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(header(scene, cx))
                    .child(body),
            );
        if let Some(popup) = popup(scene, window, cx) {
            root = root.child(popup);
        }
        content(&self.registry, root).capture_key_down(
            cx.listener(|this, event: &KeyDownEvent, window, cx| this.on_key(event, window, cx)),
        )
    }
}
