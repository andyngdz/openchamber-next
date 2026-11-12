//! The picker header records the colors it paints.

use crate::icons::RemixIcon;
use crate::layout::LayoutRegistry;
use crate::theme;

use super::{Header, header};

fn a_header(icon: Option<RemixIcon>) -> Header {
    Header {
        name: "Favorites".to_owned(),
        label: "Favorites".to_owned(),
        icon,
        logo: None,
        ancestors: Vec::new(),
    }
}

#[test]
fn a_section_header_records_its_label_in_the_muted_foreground() {
    let registry = LayoutRegistry::default();
    header(&registry, &a_header(Some(RemixIcon::StarFill)));

    let runs = registry.texts_of("popover/Favorites");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].0.color, theme::palette().muted_foreground.into());
}

#[test]
fn the_favorites_star_is_recorded_in_the_primary_color() {
    let registry = LayoutRegistry::default();
    header(&registry, &a_header(Some(RemixIcon::StarFill)));

    let icons = registry.icons_of("popover/Favorites");
    assert_eq!(icons.len(), 1);
    assert_eq!(
        icons[0].0.color(),
        Some(theme::hex(theme::palette().primary.into()))
    );
}
