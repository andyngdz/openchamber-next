use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::component::diff::{Diff, DiffFile, DiffState};
use gpui_kit::platform::{current_headless_renderer, current_platform};
use gpui_kit::{
    AppContext as _, Bounds, Context, Entity, HeadlessAppContext, Hsla, IntoElement,
    ParentElement as _, Render, Rgba, Styled as _, Window, WindowBounds, WindowOptions, assets,
    div, init, open_window, px, size,
};
use serde::Deserialize;

use super::syntax::{ThemeDocument, TokenStyle};
use super::{DARK_THEME, ITALIC};
use crate::theme::DARK as PALETTE;

/// The repository root, found the way `oc-sidecar` finds the CLI.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join("packages/web/bin/cli.js").is_file())
        .map(Path::to_path_buf)
        .expect("repository root above the crate")
}

/// A theme file from `packages/ui/src/lib/theme/themes/`.
fn ui_theme(id: &str) -> String {
    let path = repo_root()
        .join("packages/ui/src/lib/theme/themes")
        .join(format!("{id}.json"));
    std::fs::read_to_string(path).expect("theme source reads")
}

#[derive(Deserialize)]
struct Fixture {
    themes: BTreeMap<String, FixtureTheme>,
}

#[derive(Deserialize)]
struct FixtureTheme {
    syntax: BTreeMap<String, FixtureStyle>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct FixtureStyle {
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    font_weight: Option<u16>,
    #[serde(default)]
    font_style: Option<String>,
}

fn expected(style: &TokenStyle) -> FixtureStyle {
    FixtureStyle {
        color: style.color.clone(),
        font_weight: style.bold.then_some(700),
        font_style: style.italic.then(|| ITALIC.to_owned()),
    }
}

#[test]
fn the_embedded_dark_theme_is_the_ui_file() {
    assert_eq!(
        DARK_THEME.as_bytes(),
        ui_theme("openchamber-dark").as_bytes()
    );
}

#[test]
fn every_token_matches_the_typescript_fixture() {
    let source = std::fs::read_to_string(
        repo_root().join("packages/desktop-rs/fixtures/theme/zed-tokens.json"),
    )
    .expect("theme fixture reads");
    let fixture: Fixture = serde_json::from_str(&source).expect("theme fixture parses");
    let mut checked = 0;
    for (id, expected_theme) in &fixture.themes {
        let document: ThemeDocument = serde_json::from_str(&ui_theme(id)).expect("theme parses");
        let resolved = document.resolve();
        for (name, expected_style) in &expected_theme.syntax {
            let actual = resolved
                .style(name)
                .unwrap_or_else(|| panic!("{id}: {name} has no rule"));
            assert_eq!(&expected(&actual), expected_style, "{id}: {name}");
            checked += 1;
        }
        assert_eq!(
            resolved.styles().count(),
            expected_theme.syntax.len(),
            "{id}: rule count"
        );
    }
    assert!(checked > 0);
}

fn keyword_color() -> Hsla {
    let document: ThemeDocument = serde_json::from_str(DARK_THEME).expect("embedded theme parses");
    let color = document
        .resolve()
        .style("keyword")
        .and_then(|style| style.color)
        .expect("keyword has a color");
    Rgba::try_from(color.as_str())
        .expect("keyword color parses")
        .into()
}

fn rust_patch() -> String {
    [
        "--- a/sample.rs",
        "+++ b/sample.rs",
        "@@ -1 +1 @@",
        "-fn old() {}",
        "+fn new() {}",
    ]
    .join("\n")
}

struct Host {
    state: Entity<DiffState>,
}

impl Render for Host {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(Diff::new(&self.state))
    }
}

/// The colors GPUI painted, monochrome and subpixel.
fn painted_colors(
    headless: &mut HeadlessAppContext,
    window: gpui_kit::AnyWindowHandle,
) -> Vec<Hsla> {
    headless
        .update_window(window, |_, window, _| {
            let scene = window.rendered_scene();
            let monochrome = scene.monochrome_sprites.iter().map(|sprite| sprite.color);
            let subpixel = scene.subpixel_sprites.iter().map(|sprite| sprite.color);
            monochrome.chain(subpixel).collect()
        })
        .expect("scene reads")
}

#[test]
fn a_rust_diff_paints_a_keyword_with_the_theme_keyword_color() {
    let keyword = keyword_color();
    let text_system = current_platform(true).text_system();
    let mut headless = HeadlessAppContext::with_platform(
        text_system,
        Arc::new(assets::Assets),
        current_headless_renderer,
    );
    headless.allow_parking();
    let window = headless
        .update(|cx| {
            init(cx);
            crate::theme::apply(PALETTE, cx);
            let options = WindowOptions {
                focus: false,
                show: false,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(720.), px(320.)),
                    cx,
                ))),
                ..Default::default()
            };
            open_window(options, cx, |_, cx| {
                let files = DiffFile::parse(&rust_patch()).expect("patch parses");
                let state = cx.new(|cx| DiffState::new(files, cx));
                cx.new(|_| Host { state })
            })
        })
        .expect("window opens")
        .0;

    for _ in 0..4 {
        headless
            .update_window(window, |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .expect("frame draws");
        headless.run_until_parked();
    }

    let painted = painted_colors(&mut headless, window);
    assert!(
        painted.contains(&keyword),
        "the keyword color {keyword:?} was not painted"
    );
}
