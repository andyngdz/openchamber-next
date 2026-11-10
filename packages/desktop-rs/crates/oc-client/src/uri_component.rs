//! `encodeURIComponent` for the `x-opencode-directory` header and the query string.

const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Percent-encodes everything except `A-Z a-z 0-9 - _ . ! ~ * ' ( )`, byte for byte as
/// JavaScript's `encodeURIComponent` does.
pub(crate) fn encode_uri_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    out
}

#[cfg(test)]
#[path = "uri_component_test.rs"]
mod uri_component_test;
