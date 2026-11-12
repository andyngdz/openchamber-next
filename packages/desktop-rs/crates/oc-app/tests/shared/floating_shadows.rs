/// The outer floating shadows of the dark theme as `(offset y, blur, spread)`. The frame check
/// reads them to size the popup skip, and `tokens_test.rs` asserts `Tokens::floating_shadows`
/// paints exactly these, so the two cannot drift apart.
pub const FLOATING_SHADOWS: [(f64, f64, f64); 2] = [(2., 8., 0.), (28., 64., -12.)];
