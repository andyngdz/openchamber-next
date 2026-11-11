use super::new_message_id;

#[test]
fn id_has_the_shape_the_ui_mints() {
    let id = new_message_id();
    let rest = id.strip_prefix("msg_").expect("msg_ prefix");
    assert_eq!(rest.len(), 12 + 14);
    let (stamp, random) = rest.split_at(12);
    assert!(
        stamp
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    assert!(random.chars().all(|c| c.is_ascii_alphanumeric()));
}

#[test]
fn ids_minted_in_a_row_ascend_and_differ() {
    let ids: Vec<String> = (0..50).map(|_| new_message_id()).collect();
    for pair in ids.windows(2) {
        let left = pair[0].as_str();
        let right = pair[1].as_str();
        assert!(left[..16] < right[..16], "{} !< {}", left, right);
        assert_ne!(left, right);
    }
}
