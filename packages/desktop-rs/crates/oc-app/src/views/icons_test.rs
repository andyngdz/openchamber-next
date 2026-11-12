//! Every icon the views name is a generated Remix icon with a file, and no gpui-kit icon
//! name survives in the views.

use std::path::{Path, PathBuf};

use gpui_kit::component::IconNamed;

use crate::icons::RemixIcon;

/// `RemixIcon::Foo` tokens in the view sources, and every source that mentions `IconName`.
fn view_icons() -> (Vec<String>, Vec<String>) {
    let views = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("views");
    let mut found = Vec::new();
    let mut stale = Vec::new();
    let mut folders = vec![views];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(folder).expect("views folder reads") {
            let path = entry.expect("entry reads").path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            let name = path.to_string_lossy();
            if name.ends_with("_test.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("source reads");
            if source.contains("IconName") {
                stale.push(name.into_owned());
            }
            for (index, _) in source.match_indices("RemixIcon::") {
                let token: String = source[index + "RemixIcon::".len()..]
                    .chars()
                    .take_while(char::is_ascii_alphanumeric)
                    .collect();
                found.push(token);
            }
        }
    }
    found.sort();
    found.dedup();
    (found, stale)
}

fn asset_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

#[test]
fn every_icon_the_views_use_is_a_remix_file() {
    let (used, stale) = view_icons();
    assert!(
        stale.is_empty(),
        "views still name gpui-kit icons: {stale:?}"
    );
    assert!(!used.is_empty(), "the views name no Remix icons");
    let missing: Vec<String> = used
        .iter()
        .filter(|token| {
            let icon = RemixIcon::ALL
                .iter()
                .find(|icon| format!("{icon:?}") == **token);
            icon.is_none_or(|icon| !asset_dir().join(icon.path().as_ref()).is_file())
        })
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "views name icons without a generated file: {missing:?}"
    );
}
