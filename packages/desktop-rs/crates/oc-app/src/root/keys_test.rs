//! Enter-to-send through the real composer key dispatch.

use gpui_kit::component::input::Enter;
use gpui_kit::{App, Entity, EntityInputHandler as _, TestAppContext, VisualTestContext, Window};
use oc_state::{ComposerStatus, State};

use super::RootView;
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::layout::LayoutRegistry;

fn replay(name: &str) -> State {
    StateScript::load(&state_path(name))
        .expect("state fixture loads")
        .state()
}

/// The session fixture with Enter configured to insert a newline.
fn configured_off() -> State {
    let path = state_path("session");
    let text = std::fs::read_to_string(&path).expect("fixture reads");
    let mut document: serde_json::Value = serde_json::from_str(&text).expect("fixture is JSON");
    document["settings"]["enterToSend"] = serde_json::Value::Bool(false);
    document["settings"]["enterToSendConfigured"] = serde_json::Value::Bool(true);
    let temp = std::env::temp_dir().join(format!(
        "oc-app-enter-off-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&temp, document.to_string()).expect("temp fixture writes");
    let state = StateScript::load(&temp)
        .expect("temp fixture loads")
        .state();
    std::fs::remove_file(temp).ok();
    state
}

fn open(
    cx: &mut TestAppContext,
    state: State,
    popover: Option<crate::views::PopoverKind>,
) -> (Entity<RootView>, &mut VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.add_window_view(move |window, cx| {
        RootView::new(state, popover, LayoutRegistry::default(), None, window, cx)
    })
}

fn draft_text(root: &Entity<RootView>, cx: &App) -> String {
    root.read(cx).draft.read(cx).value().to_string()
}

fn status(root: &Entity<RootView>, cx: &App) -> ComposerStatus {
    root.read(cx).state.read(cx).composer().status.clone()
}

fn focus_draft(root: &Entity<RootView>, window: &mut Window, cx: &mut App) {
    root.update(cx, |root, cx| {
        root.draft.update(cx, |draft, cx| draft.focus(window, cx));
    });
}

fn type_hello(root: &Entity<RootView>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| focus_draft(root, window, cx));
    cx.simulate_input("Hello");
}

#[gpui_kit::test]
fn enter_sends_without_inserting_a_newline(cx: &mut TestAppContext) {
    let (root, cx) = open(cx, replay("session"), None);
    type_hello(&root, cx);

    cx.simulate_keystrokes("enter");

    cx.update(|_, cx| {
        assert_eq!(draft_text(&root, cx), "Hello");
        assert!(
            matches!(status(&root, cx), ComposerStatus::Failed { .. }),
            "Enter did not attempt a send"
        );
    });
}

#[gpui_kit::test]
fn shift_enter_and_alt_enter_insert_a_newline(cx: &mut TestAppContext) {
    for keys in ["shift-enter", "alt-enter"] {
        let (root, cx) = open(cx, replay("session"), None);
        type_hello(&root, cx);
        cx.simulate_keystrokes(keys);
        cx.update(|_, cx| {
            assert_eq!(status(&root, cx), ComposerStatus::Idle, "{keys} sent");
            assert!(
                draft_text(&root, cx).contains('\n'),
                "{keys} inserted no newline"
            );
        });
    }
}

#[gpui_kit::test]
fn configured_off_makes_enter_a_newline_and_ctrl_enter_send(cx: &mut TestAppContext) {
    let (root, cx) = open(cx, configured_off(), None);
    type_hello(&root, cx);

    cx.simulate_keystrokes("enter");
    cx.update(|_, cx| {
        assert_eq!(status(&root, cx), ComposerStatus::Idle);
        assert!(
            draft_text(&root, cx).contains('\n'),
            "Enter sent when configured off"
        );
    });

    cx.simulate_keystrokes("ctrl-enter");
    cx.update(|_, cx| {
        assert!(
            matches!(status(&root, cx), ComposerStatus::Failed { .. }),
            "Ctrl+Enter did not send"
        );
    });
}

#[gpui_kit::test]
fn marked_ime_text_does_not_send_on_enter(cx: &mut TestAppContext) {
    let (root, cx) = open(cx, replay("session"), None);
    cx.update(|window, cx| {
        focus_draft(&root, window, cx);
        root.update(cx, |root, cx| {
            root.draft.update(cx, |draft, cx| {
                draft.replace_and_mark_text_in_range(None, "に", None, window, cx);
            });
        });
    });

    cx.simulate_keystrokes("enter");

    cx.update(|_, cx| {
        assert_eq!(status(&root, cx), ComposerStatus::Idle, "IME Enter sent");
    });
}

#[gpui_kit::test]
fn the_action_path_uses_the_binding_shift_and_secondary(cx: &mut TestAppContext) {
    let (root, cx) = open(cx, replay("session"), None);
    type_hello(&root, cx);

    cx.dispatch_action(Enter {
        secondary: true,
        shift: false,
    });
    cx.update(|_, cx| {
        assert!(
            matches!(status(&root, cx), ComposerStatus::Failed { .. }),
            "the secondary Enter action did not send"
        );
    });
}
