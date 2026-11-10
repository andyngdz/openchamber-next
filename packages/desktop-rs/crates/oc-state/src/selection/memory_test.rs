//! The per-session agent memory.

use super::super::value::{EffortChoice, ModelKey};
use super::SelectionMemory;

#[test]
fn a_model_is_remembered_per_agent() {
    let mut memory = SelectionMemory::default();
    let first = ModelKey::new("p", "one");
    let second = ModelKey::new("q", "two");
    memory.remember_model("build", &first);
    memory.remember_model("plan", &second);
    assert_eq!(memory.agent_model("build"), Some(&first));
    assert_eq!(memory.agent_model("plan"), Some(&second));
    assert_eq!(memory.agent_model("other"), None);
}

#[test]
fn a_variant_is_remembered_per_agent_and_model_and_none_forgets_it() {
    let mut memory = SelectionMemory::default();
    let key = ModelKey::new("p", "one");
    memory.remember_variant("build", &key, Some(&EffortChoice::Default));
    assert_eq!(
        memory.agent_variant("build", &key),
        Some(&EffortChoice::Default)
    );
    memory.remember_variant("build", &key, Some(&EffortChoice::Named("high".to_owned())));
    assert_eq!(
        memory.agent_variant("build", &key),
        Some(&EffortChoice::Named("high".to_owned()))
    );
    memory.remember_variant("build", &key, None);
    assert_eq!(memory.agent_variant("build", &key), None);
}
