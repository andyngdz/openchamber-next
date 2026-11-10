//! The Choose project popup: a fixed `w-72` panel below the composer's project trigger.

use gpui_kit::component::input::Input;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{App, Div, ParentElement, Styled, div, px};
use oc_state::{Project, Settings, ranked_projects};

use super::common::{control, frame, option_row};
use crate::layout::Role;
use crate::views::Scene;
use crate::views::popover::Pick;

/// `w-72` = 18rem at the 16px root (`DraftTargetSelectors.tsx:373`).
const WIDTH: f32 = 288.;
const SEARCH: &str = "Search projects...";
const PROJECT_GROUP: &str = "Project";

/// The project picker panel.
pub(super) fn panel(scene: Scene<'_>, cx: &App) -> Div {
    let query = scene.search.project.read(cx).value().to_string();
    let rows = scene
        .state
        .settings()
        .map(|settings| project_rows(scene, settings, &query))
        .unwrap_or_default();
    frame(
        scene,
        cx,
        Some(WIDTH),
        div()
            .flex()
            .flex_col()
            .child(search_row(scene, cx))
            .children(rows),
    )
}

fn search_row(scene: Scene<'_>, cx: &App) -> Div {
    let input = Input::new(&scene.search.project)
        .appearance(false)
        .small()
        .w_full();
    // `h-8 ... border-b px-3` on the command input wrapper (`command.tsx:68-72`).
    div()
        .h(px(32.))
        .border_b_1()
        .border_color(cx.theme().border)
        .px(px(12.))
        .flex()
        .items_center()
        .child(control(scene, Role::Combobox, SEARCH, &[], input))
}

fn project_rows(scene: Scene<'_>, settings: &Settings, query: &str) -> Vec<Div> {
    let mut rows = vec![option_row(
        scene,
        Role::Option,
        "Chat",
        &[PROJECT_GROUP],
        Pick::Chat,
    )];
    rows.extend(
        ranked_projects(settings, query)
            .into_iter()
            .filter_map(|project| {
                let id = project.id()?.to_owned();
                let name = label(project);
                Some(option_row(
                    scene,
                    Role::Option,
                    name,
                    &[PROJECT_GROUP],
                    Pick::Project(id),
                ))
            }),
    );
    rows
}

fn label(project: &Project) -> String {
    project
        .label()
        .map(str::to_owned)
        .or_else(|| {
            project.path().map(|path| {
                path.trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(path)
                    .to_owned()
            })
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "project_test.rs"]
mod project_test;
