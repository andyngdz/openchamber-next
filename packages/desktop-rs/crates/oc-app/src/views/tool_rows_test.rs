use oc_state::{
    AssistantMessage, ChatMessage, Delivery, Item, MessageId, ModelRef, StepTiming, ToolItem,
    ToolStatus, UserMessage,
};
use serde_json::json;

use super::tool_row_specs;
use crate::views::turns::turns;

fn user() -> ChatMessage {
    ChatMessage::User(UserMessage {
        id: MessageId::from("msg_user"),
        text: "List the files.".to_owned(),
        delivery: Delivery::Delivered,
    })
}

fn step(patterns: &[&str]) -> ChatMessage {
    let items = patterns
        .iter()
        .map(|pattern| {
            Item::Tool(ToolItem {
                call_id: "call".into(),
                name: "glob".to_owned(),
                input: json!({ "pattern": pattern }),
                status: ToolStatus::Completed,
            })
        })
        .collect();
    ChatMessage::Assistant(AssistantMessage {
        id: MessageId::from("msg_step"),
        agent: "build".to_owned(),
        model: ModelRef::default(),
        items,
        timing: StepTiming::Running { started: 1 },
    })
}

#[test]
fn a_repeated_title_is_numbered_across_turns_and_a_single_one_is_not() {
    let messages = [user(), step(&["*.md", "*.rs"]), user(), step(&["*.md"])];

    let keys: Vec<String> = tool_row_specs(&turns(&messages))
        .into_iter()
        .map(|spec| spec.key.to_string())
        .collect();

    assert_eq!(
        keys,
        [
            "chatColumn/Find Files *.md#1",
            "chatColumn/Find Files *.rs",
            "chatColumn/Find Files *.md#2",
        ]
    );
}
