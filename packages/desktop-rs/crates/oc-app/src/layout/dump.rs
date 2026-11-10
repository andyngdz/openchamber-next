//! The registry written in the shape of `fixtures/reference/<state>.json`.

use gpui_kit::{Bounds, Pixels, Point};
use serde::Serialize;

use super::registry::{Area, LayoutRegistry, PlacedControl, Reveal, Role};
use crate::errors::AppError;

/// One frame's regions and controls, as the parity test reads them.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayoutDump {
    state: String,
    viewport: Extent,
    /// Device pixels per logical pixel; GPUI snaps every edge to this grid.
    scale_factor: f32,
    regions: RegionDumps,
}

#[derive(Debug, Serialize)]
struct Extent {
    width: f64,
    height: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegionDumps {
    sidebar: RegionDump,
    header: RegionDump,
    chat_column: RegionDump,
    composer: RegionDump,
    work_status_panel: RegionDump,
    icon_rail: RegionDump,
    /// The open picker popup; a frame with no popup omits it, like the reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    popover: Option<RegionDump>,
}

#[derive(Debug, Serialize)]
struct RegionDump {
    bounds: BoundsDump,
    controls: Vec<ControlDump>,
}

#[derive(Debug, Serialize)]
struct BoundsDump {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Serialize)]
struct ControlDump {
    role: Role,
    name: String,
    key: String,
    shown: bool,
    bounds: BoundsDump,
}

impl LayoutDump {
    /// Reads the registry after a frame, every bound relative to the content area; fails when
    /// the content area or a region was never laid out.
    pub(crate) fn capture(
        state: &str,
        scale_factor: f32,
        registry: &LayoutRegistry,
    ) -> Result<Self, AppError> {
        let content = registry
            .content()
            .ok_or(AppError::RegionMissing("content"))?;
        let origin = content.origin;
        let region = |area: Area| region_dump(area, registry, origin);
        Ok(Self {
            state: state.to_owned(),
            viewport: Extent {
                width: hundredths(content.size.width),
                height: hundredths(content.size.height),
            },
            scale_factor,
            regions: RegionDumps {
                sidebar: region(Area::Sidebar)?,
                header: region(Area::Header)?,
                chat_column: region(Area::ChatColumn)?,
                composer: region(Area::Composer)?,
                work_status_panel: region(Area::WorkStatusPanel)?,
                icon_rail: region(Area::IconRail)?,
                popover: match registry.region(Area::Popover) {
                    Some(_) => Some(region(Area::Popover)?),
                    None => None,
                },
            },
        })
    }

    /// Pretty JSON with a trailing newline, like the reference files.
    pub(crate) fn to_json(&self) -> Result<String, AppError> {
        let mut json = serde_json::to_string_pretty(self).map_err(AppError::EncodeDump)?;
        json.push('\n');
        Ok(json)
    }
}

fn region_dump(
    area: Area,
    registry: &LayoutRegistry,
    origin: Point<Pixels>,
) -> Result<RegionDump, AppError> {
    let bounds = registry
        .region(area)
        .ok_or(AppError::RegionMissing(area.json_name()))?;
    Ok(RegionDump {
        bounds: bounds_dump(bounds, origin),
        controls: registry
            .controls_in(area)
            .into_iter()
            .map(|placed| control_dump(placed, origin))
            .collect(),
    })
}

fn bounds_dump(bounds: Bounds<Pixels>, origin: Point<Pixels>) -> BoundsDump {
    BoundsDump {
        x: hundredths(bounds.origin.x - origin.x),
        y: hundredths(bounds.origin.y - origin.y),
        width: hundredths(bounds.size.width),
        height: hundredths(bounds.size.height),
    }
}

/// Rounded to 0.01 px, as the capture script rounds the reference, so `f32` noise such as
/// `48.000004` does not read as a difference.
fn hundredths(value: Pixels) -> f64 {
    (f64::from(f32::from(value)) * 100.).round() / 100.
}

fn control_dump(placed: PlacedControl, origin: Point<Pixels>) -> ControlDump {
    let shown = match placed.spec.reveal {
        Reveal::Always => true,
        Reveal::OnHover(_) | Reveal::Hidden => false,
    };
    ControlDump {
        role: placed.spec.role,
        name: placed.spec.name.to_string(),
        key: placed.spec.key.to_string(),
        shown,
        bounds: bounds_dump(placed.bounds, origin),
    }
}

#[cfg(test)]
#[path = "dump_test.rs"]
mod dump_test;
