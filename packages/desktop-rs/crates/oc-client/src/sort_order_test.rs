use super::SidebarSortOrder;

#[test]
fn every_stored_value_decodes_and_encodes_unchanged() {
    for (name, order) in [
        ("manual", SidebarSortOrder::Manual),
        ("a-z", SidebarSortOrder::AZ),
        ("z-a", SidebarSortOrder::ZA),
        ("date-added", SidebarSortOrder::DateAdded),
        ("recent", SidebarSortOrder::Recent),
    ] {
        let decoded: SidebarSortOrder =
            serde_json::from_value(serde_json::json!(name)).expect("decode");
        assert_eq!(decoded, order);
        let encoded = serde_json::to_value(order).expect("encode");
        assert_eq!(encoded, serde_json::json!(name));
    }
}

#[test]
fn an_unknown_value_keeps_the_stored_order() {
    let decoded: SidebarSortOrder =
        serde_json::from_value(serde_json::json!("by-color")).expect("decode");
    assert_eq!(decoded, SidebarSortOrder::Other);
    assert_eq!(
        serde_json::to_value(SidebarSortOrder::Other).expect("encode"),
        serde_json::json!("manual")
    );
}
