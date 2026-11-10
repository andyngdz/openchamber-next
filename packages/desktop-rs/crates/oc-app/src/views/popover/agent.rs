//! The agent menu: a fixed 280px panel with a search field and the visible primary agents.
//!
//! The reference names each row and its star by the star's aria-label
//! (`ModelControls.tsx:120,2828-2846`), not by the agent's visible name; the keys follow the
//! accessibility tree verbatim.

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{SelectionAction, ranked_agents};

use super::common::{control, frame};
use crate::layout::{Area, ControlSpec, Role, control as control_element};
use crate::views::text_metrics;
use crate::views::{Intent, Scene, model_names::agent_display_name};

/// `w-[min(280px,calc(100vw-2rem))]` (`ModelControls.tsx:2894`).
const WIDTH: f32 = 280.;
const SEARCH: &str = "Search agents";
/// `chat.modelControls.agentFavoriteAdd` (`en.ts:4003`).
const STAR_LABEL: &str = "Add to favorites. With favorites, Tab cycles through them only";
/// `h-5 w-5` on `AgentFavoriteToggle` (`ModelControls.tsx:139`): 1.25rem.
const STAR_SIZE: f32 = 20.;
/// `px-2 py-1` around the search field (`ModelControls.tsx:2895`): 0.5rem / 0.25rem.
const SEARCH_PADDING_X: f32 = 8.;
const SEARCH_PADDING_Y: f32 = 4.;
/// `h-7` on the search field (`ModelControls.tsx:2903`): 1.75rem.
const SEARCH_HEIGHT: f32 = 28.;
/// `p-1` around the agent list (`ModelControls.tsx:2908`): 0.25rem.
const LIST_PADDING: f32 = 4.;
/// `gap-1.5` between a row's children (`ModelControls.tsx:2849`): 0.375rem.
const ROW_GAP: f32 = 6.;

/// One agent row: `px-2 py-1 typography-meta` (`dropdown-menu.styles.ts`,
/// `ModelControls.tsx:2836-2843`) around a 20px star (`ModelControls.tsx:139`).
fn row_height() -> f32 {
    2. * 4. + text_metrics::line_height(text_metrics::META).max(STAR_SIZE)
}

/// The agent menu panel.
pub(super) fn panel(scene: Scene<'_>, cx: &App) -> Div {
    let query = scene.search.agent.read(cx).value().to_string();
    let rows = super::selection_directory(scene.state)
        .as_deref()
        .and_then(|directory| scene.state.catalog(directory))
        .map(|catalog| ranked_agents(catalog, &query))
        .unwrap_or_default();
    let list = div().p(px(LIST_PADDING)).flex().flex_col().children(
        rows.into_iter()
            .enumerate()
            .map(|(index, agent)| agent_row(scene, agent, index + 1)),
    );
    frame(
        scene,
        cx,
        Some(WIDTH),
        div()
            .flex()
            .flex_col()
            .child(search_row(scene, cx))
            .child(list),
    )
}

fn search_row(scene: Scene<'_>, cx: &App) -> Div {
    let input = Input::new(&scene.search.agent)
        .appearance(false)
        .small()
        .w_full();
    div()
        .px(px(SEARCH_PADDING_X))
        .py(px(SEARCH_PADDING_Y))
        .border_b_1()
        .border_color(cx.theme().border)
        .child(control(
            scene,
            Role::Textbox,
            SEARCH,
            &[],
            div().h(px(SEARCH_HEIGHT)).child(input),
        ))
}

fn agent_row(scene: Scene<'_>, agent: &oc_client::catalog::Agent, index: usize) -> Div {
    let favorite = scene
        .state
        .settings()
        .is_some_and(|settings| settings.favorite_agents.iter().any(|id| id == &agent.id));
    let star = agent_star(
        scene,
        agent,
        if favorite {
            Favorite::Yes
        } else {
            Favorite::No
        },
        index,
    );
    let content = agent_content(scene, agent, star);
    let row_spec = ControlSpec::named(Area::Popover, Role::Menuitem, STAR_LABEL)
        .described("menuitem")
        .repeated(index);
    let row = control_element(scene.registry, row_spec, content);
    div().w_full().child(row)
}

/// Whether the agent is starred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Favorite {
    Yes,
    No,
}

fn agent_star(
    scene: Scene<'_>,
    agent: &oc_client::catalog::Agent,
    favorite: Favorite,
    index: usize,
) -> Div {
    let label = match favorite {
        Favorite::Yes => "Remove from favorites",
        Favorite::No => STAR_LABEL,
    };
    let spec = ControlSpec::named(Area::Popover, Role::Button, label).repeated(index);
    control_element(
        scene.registry,
        spec.clone(),
        div()
            .id(spec.key.clone())
            .w(px(STAR_SIZE))
            .h(px(STAR_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .child(match favorite {
                Favorite::Yes => IconName::StarFill,
                Favorite::No => IconName::Star,
            })
            .on_click(scene.on_click_intent(Intent::ApplySelection(
                SelectionAction::ToggleFavoriteAgent(agent.id.clone()),
            ))),
    )
}

fn agent_content(scene: Scene<'_>, agent: &oc_client::catalog::Agent, star: Div) -> AnyElement {
    let name = agent_display_name(&agent.display_name, &agent.id);
    div()
        .id(agent.id.clone())
        .h(px(row_height()))
        .px(px(SEARCH_PADDING_X))
        .flex()
        .items_center()
        .gap(px(ROW_GAP))
        .child(name)
        .child(div().flex_1())
        .child(star)
        .on_click(scene.on_click_intent(Intent::Pick(super::Pick::Agent(agent.id.clone()))))
        .into_any_element()
}

#[cfg(test)]
#[path = "agent_test.rs"]
mod agent_test;
