//! The window's root view: holds the [`State`] entity and applies every action through
//! [`State::apply`], so a fixture replay and live events take the same path. With a live
//! server it also turns intents into requests and applies what the server sends back.

mod live;

pub(crate) use live::Started;

use std::rc::Rc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, ScrollHandle, Styled,
    Subscription, Task, Window, div, px,
};
use oc_state::{Action, State};

use crate::layout::{LayoutRegistry, content};
use crate::live::ServerLink;
use crate::views::{
    Dispatch, PLACEHOLDER, Scene, chat_column, header, icon_rail, sidebar, work_status,
};

/// The whole window.
pub(crate) struct RootView {
    state: Entity<State>,
    registry: LayoutRegistry,
    draft: Entity<TextareaState>,
    scroll: ScrollHandle,
    dispatch: Dispatch,
    /// The live server; `None` for a fixture, whose server-bound controls do nothing.
    link: Option<ServerLink>,
    /// Waits for the server thread's answer to the latest start; a new start replaces it.
    starting: Option<Task<()>>,
    _draft_events: Subscription,
}

impl RootView {
    /// A window over `state`; the registry receives every frame's bounds.
    pub(crate) fn new(
        state: State,
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
            link,
            starting: None,
            _draft_events: draft_events,
        }
    }

    /// Applies one action; the message list follows new content while it sits at the bottom.
    pub(crate) fn dispatch(&mut self, action: Action, cx: &mut Context<Self>) {
        self.state.update(cx, |state, _| state.apply(action));
        if self.scroll.offset().y <= -self.scroll.max_offset().y + px(1.) {
            self.scroll.scroll_to_bottom();
        }
        cx.notify();
    }
}

impl Render for RootView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.registry.begin_frame();
        let state = self.state.read(cx);
        let scene = Scene {
            state,
            registry: &self.registry,
            dispatch: &self.dispatch,
        };
        let theme = cx.theme();
        let body = div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(chat_column(scene, &self.draft, &self.scroll, cx))
            .child(work_status(scene, cx))
            .child(icon_rail(&self.registry, cx));
        let root = div()
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
        content(&self.registry, root)
    }
}
