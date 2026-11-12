//! The path a bundled logo is drawn from is one the asset source serves.

use gpui_kit::AssetSource;

use super::logo_path;
use crate::icons::AppAssets;
use crate::icons::provider::{ProviderLogo, provider_logo};

#[test]
fn a_bundled_logo_is_drawn_from_a_path_the_assets_serve() {
    let ProviderLogo::Bundled { id } = provider_logo("anthropic") else {
        panic!("anthropic has a bundled logo");
    };
    let bytes = AppAssets
        .load(&logo_path(&id))
        .expect("the asset source answers")
        .expect("the bundled logo is served");
    assert!(!bytes.is_empty());
}
