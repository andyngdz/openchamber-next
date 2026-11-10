use super::encode_uri_component;

#[test]
fn matches_encode_uri_component() {
    assert_eq!(
        encode_uri_component("/tmp/My Projects/ünï"),
        "%2Ftmp%2FMy%20Projects%2F%C3%BCn%C3%AF"
    );
    assert_eq!(
        encode_uri_component("a-b_c.d!e~f*g'h(i)"),
        "a-b_c.d!e~f*g'h(i)"
    );
    assert_eq!(encode_uri_component("a&b=c?d#e"), "a%26b%3Dc%3Fd%23e");
}
