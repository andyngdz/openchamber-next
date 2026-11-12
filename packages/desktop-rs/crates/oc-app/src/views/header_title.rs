//! The header's title slot: the open session's name, or the draft's muted placeholder.

use gpui_kit::component::{ActiveTheme, Icon, Sizable, spinner::Spinner};
use gpui_kit::{AnyElement, App, FontWeight, IntoElement, ParentElement, Styled, div, px};
use oc_state::SessionStatus;

use super::Scene;
use crate::icons::RemixIcon;

/// The open session's title, medium weight and truncating, behind a spinner while the
/// session works; the empty state names the draft in muted text.
pub(super) fn title(scene: Scene<'_>, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let Some(entry) = scene.state.open_entry() else {
        return div()
            .text_size(px(13.))
            .text_color(theme.muted_foreground)
            .child("New session")
            .into_any_element();
    };
    let title = entry.title.clone().unwrap_or_default();
    let working = scene.state.open_status() == SessionStatus::Busy;
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(8.))
        .children(working.then(|| {
            Spinner::new()
                .icon(Icon::new(RemixIcon::Loader4))
                .with_size(px(14.))
                .color(theme.info)
        }))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .into_any_element()
}
