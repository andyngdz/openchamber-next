use gpui_kit::component::IconNamed;

use super::CustomIcon;

#[test]
fn every_custom_icon_has_a_file_and_is_named_other() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    for icon in [
        CustomIcon::Stop,
        CustomIcon::ContextRing,
        CustomIcon::ArrowsMerge,
    ] {
        assert!(
            root.join(icon.path().as_ref()).is_file(),
            "{} has no file; run tools/gen-remix-icons.mjs",
            icon.path()
        );
    }
}
