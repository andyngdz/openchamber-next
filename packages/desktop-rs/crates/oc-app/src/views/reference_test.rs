//! Reads control keys from `fixtures/reference/<state>.json` for the region tests.

use std::path::Path;

use serde_json::Value;

fn reference(state: &str) -> Value {
    let path = Path::new(env!("OC_DESKTOP_FIXTURES"))
        .join("reference")
        .join(format!("{state}.json"));
    let text = std::fs::read_to_string(&path).expect("reference file reads");
    serde_json::from_str(&text).expect("reference file is JSON")
}

/// Accessible names of one region's controls in reference order. Names, not keys, so the
/// region tests hold whichever key format the reference uses.
pub(crate) fn names(state: &str, region: &str) -> Vec<String> {
    reference(state)["regions"][region]["controls"]
        .as_array()
        .expect("region has controls")
        .iter()
        .map(|control| control["name"].as_str().expect("name is text").to_owned())
        .collect()
}

#[test]
fn names_reads_the_reference_order() {
    assert_eq!(
        names("empty", "iconRail").first().map(String::as_str),
        Some("Context")
    );
}
