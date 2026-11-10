//! Every icon the views name must be one GPUI Kit bundles; the rest of the Lucide enum has
//! no asset and renders blank.

use std::path::Path;

use gpui_kit::AssetSource;
use gpui_kit::assets::{Assets, IconName};

/// `IconName::Foo` tokens in the view sources (read at test time).
fn icons_in_views() -> Vec<String> {
    let views = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("views");
    let mut found = Vec::new();
    let mut folders = vec![views];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(folder).expect("views folder reads") {
            let path = entry.expect("entry reads").path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            if path.to_string_lossy().ends_with("_test.rs") {
                continue;
            }
            let source = std::fs::read_to_string(path).expect("source reads");
            for (index, _) in source.match_indices("IconName::") {
                let name: String = source[index + "IconName::".len()..]
                    .chars()
                    .take_while(char::is_ascii_alphanumeric)
                    .collect();
                found.push(name);
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

#[test]
fn every_icon_the_views_use_is_bundled() {
    let used = icons_in_views();
    assert!(!used.is_empty());
    let missing: Vec<&String> = used
        .iter()
        .filter(|name| {
            let icon = IconName::ALL
                .iter()
                .find(|icon| format!("{icon:?}") == **name);
            icon.is_none_or(|icon| !matches!(Assets.load(&icon.path()), Ok(Some(_))))
        })
        .collect();

    assert!(
        missing.is_empty(),
        "icons without a bundled asset: {missing:?}"
    );
}
