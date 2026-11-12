//! The app's asset source: Remix icons, gpui-kit's Lucide paths mapped to Remix, and the
//! rest from gpui-kit's own bundle.

use std::borrow::Cow;

use gpui_kit::component::IconNamed;
use gpui_kit::{AssetSource, Result, SharedString};

use super::RemixIcon;
use super::remix_files::FILES;

/// The folder the generated Remix files live under.
const REMIX_PREFIX: &str = "icons/remix/";
/// The extension every icon asset carries.
const SVG_SUFFIX: &str = ".svg";

/// The gpui-kit icon paths the components oc-app uses draw for themselves, and the Remix
/// symbol React draws in the same place. A path here never reaches gpui-kit's own bundle.
pub(crate) const SUBSTITUTIONS: &[(&str, RemixIcon)] = &[
    ("icons/arrow-down.svg", RemixIcon::ArrowDown),
    ("icons/arrow-up.svg", RemixIcon::ArrowUp),
    ("icons/book-open.svg", RemixIcon::BookMarked),
    ("icons/calendar.svg", RemixIcon::Calendar),
    ("icons/case-sensitive.svg", RemixIcon::Text),
    ("icons/check.svg", RemixIcon::Check),
    ("icons/chevron-down.svg", RemixIcon::ArrowDownS),
    ("icons/chevron-left.svg", RemixIcon::ArrowLeftS),
    ("icons/chevron-right.svg", RemixIcon::ArrowRightS),
    ("icons/chevron-up.svg", RemixIcon::ArrowUpS),
    ("icons/chevrons-up-down.svg", RemixIcon::ArrowUpDown),
    ("icons/circle-check.svg", RemixIcon::CheckboxCircle),
    ("icons/circle-x.svg", RemixIcon::CloseCircle),
    ("icons/close.svg", RemixIcon::Close),
    ("icons/copy.svg", RemixIcon::FileCopy),
    ("icons/ellipsis.svg", RemixIcon::More),
    ("icons/external-link.svg", RemixIcon::ExternalLink),
    ("icons/eye-off.svg", RemixIcon::EyeOff),
    ("icons/eye.svg", RemixIcon::Eye),
    ("icons/inbox.svg", RemixIcon::InboxArchive),
    ("icons/info.svg", RemixIcon::Information),
    ("icons/loader.svg", RemixIcon::Loader4),
    ("icons/mic.svg", RemixIcon::Mic),
    ("icons/minus.svg", RemixIcon::Subtract),
    ("icons/plus.svg", RemixIcon::Add),
    ("icons/refresh-cw.svg", RemixIcon::Refresh),
    ("icons/search.svg", RemixIcon::Search),
    ("icons/square.svg", RemixIcon::Stop),
    ("icons/star-fill.svg", RemixIcon::StarFill),
    ("icons/star.svg", RemixIcon::Star),
    ("icons/triangle-alert.svg", RemixIcon::Alert),
    ("icons/user.svg", RemixIcon::User),
];

/// Serves the embedded Remix files, substitutes gpui-kit's own icons, and delegates the rest.
pub(crate) struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        #[cfg(test)]
        super::icons_test::record(path);
        if let Some(name) = remix_name(path) {
            return Ok(remix_bytes(name).map(Cow::Borrowed));
        }
        if let Some((_, icon)) = SUBSTITUTIONS
            .iter()
            .find(|(candidate, _)| *candidate == path)
        {
            return Ok(remix_file(*icon).map(Cow::Borrowed));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut listed: Vec<SharedString> = RemixIcon::ALL
            .iter()
            .map(|icon| icon.path())
            .filter(|candidate| candidate.as_ref().starts_with(path))
            .collect();
        listed.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(listed)
    }
}

/// The sprite name inside `icons/remix/<name>.svg`.
fn remix_name(path: &str) -> Option<&str> {
    path.strip_prefix(REMIX_PREFIX)?.strip_suffix(SVG_SUFFIX)
}

/// The embedded bytes of one sprite symbol.
fn remix_bytes(name: &str) -> Option<&'static [u8]> {
    FILES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, bytes)| *bytes)
}

/// The embedded file of one [`RemixIcon`].
fn remix_file(icon: RemixIcon) -> Option<&'static [u8]> {
    let path = icon.path();
    remix_bytes(remix_name(path.as_ref())?)
}
