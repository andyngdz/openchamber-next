use oc_state::{
    AssistantMessage, ChatMessage, Delivery, Item, MessageId, ModelRef, StepTiming, TextItem,
    ToolItem, ToolStatus, UserMessage,
};

use serde_json::json;

use super::{Block, ToolTitle, format_seconds, repeat_indexes, tool_status, turns};

fn user(text: &str) -> ChatMessage {
    ChatMessage::User(UserMessage {
        id: MessageId::from("msg_user"),
        text: text.to_owned(),
        delivery: Delivery::Delivered,
    })
}

fn step(items: Vec<Item>, timing: StepTiming) -> ChatMessage {
    ChatMessage::Assistant(AssistantMessage {
        id: MessageId::from("msg_step"),
        agent: "build".to_owned(),
        model: ModelRef {
            id: "agent-20tools-300cps".to_owned(),
            ..ModelRef::default()
        },
        items,
        timing,
    })
}

fn text(body: &str) -> Item {
    Item::Text(TextItem {
        ordinal: 0,
        text: body.to_owned(),
    })
}

fn tool(name: &str) -> Item {
    Item::Tool(ToolItem {
        call_id: "call_1".into(),
        name: name.to_owned(),
        input: serde_json::Value::Null,
        status: ToolStatus::Completed,
    })
}

#[test]
fn tool_calls_between_texts_share_one_activity_block() {
    let messages = [
        user("List the files."),
        step(
            vec![text("Step 1"), tool("glob")],
            StepTiming::Completed {
                started: 1_000,
                completed: 2_000,
            },
        ),
        step(
            vec![text("  "), tool("glob")],
            StepTiming::Completed {
                started: 2_000,
                completed: 3_000,
            },
        ),
        step(
            vec![text("Done.\n\n")],
            StepTiming::Completed {
                started: 3_000,
                completed: 25_500,
            },
        ),
    ];

    let turns = turns(&messages);

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].user_text, Some("List the files."));
    let blocks = turns[0].blocks();
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0], Block::Text("Step 1"));
    assert!(matches!(&blocks[1], Block::Activity(tools) if tools.len() == 2));
    assert_eq!(blocks[2], Block::Text("Done."));
    let meta = turns[0].meta().expect("turn ended");
    assert_eq!(meta.duration, "24.5s");
    assert_eq!(meta.agent, "build");
}

#[test]
fn a_running_turn_has_no_meta_row() {
    let messages = [
        user("Hi"),
        step(vec![text("Hel")], StepTiming::Running { started: 1_000 }),
    ];

    assert_eq!(turns(&messages)[0].meta(), None);
}

#[test]
fn steps_before_any_user_message_form_their_own_turn() {
    let messages = [step(
        vec![text("Hello")],
        StepTiming::Running { started: 1 },
    )];

    let turns = turns(&messages);

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].user_text, None);
}

#[test]
fn seconds_round_to_one_decimal() {
    assert_eq!(format_seconds(24_449), "24.4s");
    assert_eq!(format_seconds(24_450), "24.5s");
    assert_eq!(format_seconds(300), "0.3s");
}

fn glob(pattern: serde_json::Value, status: ToolStatus) -> ToolItem {
    ToolItem {
        call_id: "call_1".into(),
        name: "glob".to_owned(),
        input: pattern,
        status,
    }
}

#[test]
fn a_glob_row_is_titled_find_files_and_its_pattern() {
    let row = glob(json!({"pattern": "*.md"}), ToolStatus::Completed);

    let title = ToolTitle::of(&row);

    assert_eq!(title.name, "Find Files");
    assert_eq!(title.detail, Some("*.md"));
    assert_eq!(title.accessible_name(), "Find Files *.md");
    assert_eq!(tool_status(&row), None);
}

#[test]
fn a_glob_row_without_input_yet_is_titled_find_files() {
    let row = glob(serde_json::Value::Null, ToolStatus::Pending);

    assert_eq!(ToolTitle::of(&row).accessible_name(), "Find Files");
    assert_eq!(tool_status(&row), Some("pending"));
}

#[test]
fn another_tool_keeps_its_raw_name() {
    let Item::Tool(row) = tool("webfetch") else {
        unreachable!("tool() builds a tool item");
    };

    assert_eq!(ToolTitle::of(&row).accessible_name(), "webfetch");
}

#[test]
fn only_repeated_titles_get_an_index() {
    let titles = ["a", "b", "a", "c", "a"].map(str::to_owned);

    assert_eq!(
        repeat_indexes(&titles),
        [Some(1), None, Some(2), None, Some(3)]
    );
}
