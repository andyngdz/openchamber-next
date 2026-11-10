use oc_state::{ConnectionStatus, ServerExit, ServerFailure};

use super::{CONTEXT_USAGE, HeaderItem, HeaderMode, items, server_label};
use crate::views::reference_test::names as reference_names;

fn control_names(mode: HeaderMode) -> Vec<&'static str> {
    items(mode)
        .into_iter()
        .filter_map(|item| match item {
            HeaderItem::Tool(name, _) | HeaderItem::Window(name, _, _) => Some(name),
            HeaderItem::ContextUsage => Some(CONTEXT_USAGE),
            HeaderItem::Title | HeaderItem::Spacer | HeaderItem::ServerState => None,
        })
        .collect()
}

#[test]
fn header_controls_follow_the_reference_order() {
    assert_eq!(
        control_names(HeaderMode::Empty),
        reference_names("empty", "header")
    );
    assert_eq!(
        control_names(HeaderMode::Session),
        reference_names("session", "header")
    );
    assert_eq!(
        control_names(HeaderMode::Session),
        reference_names("streaming", "header")
    );
}

#[test]
fn server_label_names_every_connection_state() {
    let failure = ServerFailure {
        message: "node not found".to_owned(),
        ..ServerFailure::default()
    };

    assert_eq!(server_label(&ConnectionStatus::Starting), "Starting server");
    assert_eq!(server_label(&ConnectionStatus::Connected), "Connected");
    assert_eq!(
        server_label(&ConnectionStatus::Reconnecting { attempt: 2 }),
        "Reconnecting (attempt 2)"
    );
    assert_eq!(
        server_label(&ConnectionStatus::Failed(failure)),
        "node not found"
    );
    assert_eq!(
        server_label(&ConnectionStatus::ServerExited(ServerExit::Code(1))),
        "Server exited with code 1"
    );
    assert_eq!(
        server_label(&ConnectionStatus::ServerExited(ServerExit::Signal(9))),
        "Server stopped by signal 9"
    );
}
