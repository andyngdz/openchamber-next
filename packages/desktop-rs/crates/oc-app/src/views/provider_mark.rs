//! A provider's mark as React's `ProviderLogo` draws it (`components/ui/ProviderLogo.tsx`).

use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{AnyElement, Hsla, IntoElement, ParentElement, SharedString, Styled, div, px, svg};

use crate::icons::provider::ProviderLogo;

/// `dark:invert` on React's monochrome bundled logo: its black drawn white.
const INVERTED_LOGO: Hsla = Hsla {
    h: 0.,
    s: 0.,
    l: 1.,
    a: 1.,
};

/// The asset path the app's asset source serves a bundled logo under.
pub(crate) fn logo_path(id: &str) -> SharedString {
    format!("provider-logos/{id}.svg").into()
}

/// A `size` box holding the mark. A bundled logo is drawn inverted, as React's `dark:invert`
/// draws it. A models.dev logo keeps its box empty until the live runtime loads it, as
/// React's `<img loading="eager">` holds its place. The fallback draws its icon muted.
pub(crate) fn provider_mark(size: f32, logo: &ProviderLogo) -> AnyElement {
    let slot = div().flex_shrink_0().w(px(size)).h(px(size));
    match logo {
        ProviderLogo::Bundled { id } => slot
            .child(
                svg()
                    .path(logo_path(id))
                    .size_full()
                    .text_color(INVERTED_LOGO),
            )
            .into_any_element(),
        ProviderLogo::Fallback { icon: Some(icon) } => slot
            .child(
                Icon::new(*icon)
                    .with_size(px(size))
                    .text_color(crate::theme::palette().muted_foreground),
            )
            .into_any_element(),
        ProviderLogo::Remote { .. } | ProviderLogo::Fallback { icon: None } => {
            slot.into_any_element()
        }
    }
}

#[cfg(test)]
#[path = "provider_mark_test.rs"]
mod provider_mark_test;
