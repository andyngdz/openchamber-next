//! The frame check of design.md: every color a layout dump claims is checked against the
//! frame the same headless window paints, so the dump cannot report a color the screen does
//! not show.
//!
//! Each fixture state is rendered twice, as `--dump-layout` and as `--screenshot`, and every
//! shown control's draws are read back from the PNG:
//! - an icon or text run passes when one pixel inside its bounds equals its expected color
//!   within 1 per channel;
//! - a mark passes when its center pixel does.
//!
//! The expected color is the claim composited source-over on the backdrop. A draw that
//! carries its own bounds sits on the most frequent color of the one-pixel ring around them.
//! A draw that only knows its control's bounds sits on the control's own surface, the most
//! frequent color inside them, since its glyphs cover a small part of the box.
//!
//! Only pixels the screen can show are read: a draw is clipped to its region's bounds, and a
//! draw under the open popup, outside the popup region, is skipped.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use image::{Rgba, RgbaImage};
use serde_json::Value;

/// One channel may differ by this much, as the parity colors may.
const CHANNEL_TOLERANCE: u8 = 1;
/// The region a popup draws in; it covers the regions under it.
const POPOVER: &str = "popover";
/// The draw kinds a control records, and whether the kind is a mark.
const DRAW_KINDS: [(&str, bool); 3] = [("icons", false), ("marks", true), ("texts", false)];

/// A box in device pixels, right and bottom exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PixelBox {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl PixelBox {
    /// Logical bounds from the dump, scaled to the frame's device pixels.
    fn read(bounds: &Value, scale: f64) -> Self {
        let number = |field: &str| {
            bounds[field]
                .as_f64()
                .unwrap_or_else(|| panic!("bounds without {field}: {bounds}"))
        };
        let (x, y) = (number("x"), number("y"));
        let device = |value: f64| (value * scale).round() as i32;
        Self {
            left: device(x),
            top: device(y),
            right: device(x + number("width")),
            bottom: device(y + number("height")),
        }
    }

    fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    fn intersect(self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }

    fn overlaps(self, other: Self) -> bool {
        !self.intersect(other).is_empty()
    }

    fn contains(self, x: i32, y: i32) -> bool {
        (self.left..self.right).contains(&x) && (self.top..self.bottom).contains(&y)
    }

    fn points(self) -> impl Iterator<Item = (i32, i32)> {
        (self.top..self.bottom).flat_map(move |y| (self.left..self.right).map(move |x| (x, y)))
    }

    /// The one-pixel ring just outside the box.
    fn ring(self) -> impl Iterator<Item = (i32, i32)> {
        let rows =
            (self.left - 1..=self.right).flat_map(move |x| [(x, self.top - 1), (x, self.bottom)]);
        let columns =
            (self.top..self.bottom).flat_map(move |y| [(self.left - 1, y), (self.right, y)]);
        rows.chain(columns)
    }
}

/// One rendered state: its dump and its frame.
struct Rendered {
    name: String,
    layout: Value,
    frame: RgbaImage,
    scale: f64,
}

impl Rendered {
    fn pixel(&self, x: i32, y: i32) -> Option<Rgba<u8>> {
        let column = u32::try_from(x).ok()?;
        let row = u32::try_from(y).ok()?;
        (column < self.frame.width() && row < self.frame.height())
            .then(|| *self.frame.get_pixel(column, row))
    }

    fn most_frequent(&self, points: impl Iterator<Item = (i32, i32)>) -> Option<Rgba<u8>> {
        let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
        for color in points.filter_map(|(x, y)| self.pixel(x, y)) {
            *counts.entry(color.0).or_default() += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(color, count)| (*count, *color))
            .map(|(color, _)| Rgba(color))
    }

    fn region_box(&self, region: &str) -> Option<PixelBox> {
        let bounds = &self.layout["regions"][region]["bounds"];
        (!bounds.is_null()).then(|| PixelBox::read(bounds, self.scale))
    }
}

/// `#rrggbbaa` composited source-over on an opaque backdrop.
fn composite(claim: &str, backdrop: Rgba<u8>) -> Rgba<u8> {
    let digits = claim
        .strip_prefix('#')
        .filter(|digits| digits.len() == 8)
        .unwrap_or_else(|| panic!("{claim} is not #rrggbbaa"));
    let channel = |index: usize| {
        u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16)
            .unwrap_or_else(|_| panic!("{claim} is not hex"))
    };
    let alpha = u32::from(channel(3));
    let blend = |index: usize| {
        let over = u32::from(channel(index)) * alpha;
        let under = u32::from(backdrop.0[index]) * (255 - alpha);
        u8::try_from((over + under + 127) / 255).unwrap_or(u8::MAX)
    };
    Rgba([blend(0), blend(1), blend(2), u8::MAX])
}

fn close_to(actual: Rgba<u8>, expected: Rgba<u8>) -> bool {
    actual
        .0
        .iter()
        .zip(expected.0)
        .all(|(drawn, wanted)| drawn.abs_diff(wanted) <= CHANNEL_TOLERANCE)
}

/// Where one draw is checked, and on what backdrop.
struct Target {
    visible: PixelBox,
    backdrop: Rgba<u8>,
}

/// The visible part of a draw and its backdrop; `None` when the screen cannot show it.
fn target(rendered: &Rendered, region: &str, control: &Value, draw: &Value) -> Option<Target> {
    let bounds = PixelBox::read(&draw["bounds"], rendered.scale);
    let popup = rendered.region_box(POPOVER);
    if region != POPOVER && popup.is_some_and(|popup| popup.overlaps(bounds)) {
        return None;
    }
    let clip = rendered.region_box(region)?;
    let visible = bounds.intersect(clip);
    if visible.is_empty() {
        return None;
    }
    let own_bounds = bounds != PixelBox::read(&control["bounds"], rendered.scale);
    let backdrop = if own_bounds {
        rendered.most_frequent(bounds.ring())
    } else {
        rendered.most_frequent(visible.points())
    }?;
    Some(Target { visible, backdrop })
}

/// Whether the frame shows one draw's claimed color, or `None` when it cannot be seen.
fn draw_shows(
    rendered: &Rendered,
    region: &str,
    control: &Value,
    draw: &Value,
    is_mark: bool,
) -> Option<bool> {
    let claim = draw["color"].as_str()?;
    let target = target(rendered, region, control, draw)?;
    let expected = composite(claim, target.backdrop);
    let shows = |(x, y): (i32, i32)| {
        rendered
            .pixel(x, y)
            .is_some_and(|drawn| close_to(drawn, expected))
    };
    if !is_mark {
        return Some(target.visible.points().any(shows));
    }
    let bounds = PixelBox::read(&draw["bounds"], rendered.scale);
    let center = (
        (bounds.left + bounds.right) / 2,
        (bounds.top + bounds.bottom) / 2,
    );
    target
        .visible
        .contains(center.0, center.1)
        .then(|| shows(center))
}

/// Every claimed color of one state that its frame does not show.
fn frame_failures(rendered: &Rendered) -> Vec<String> {
    let regions = rendered.layout["regions"]
        .as_object()
        .unwrap_or_else(|| panic!("{} has no regions", rendered.name));
    let mut failures = Vec::new();
    let mut checked = 0;
    for (region, content) in regions {
        let controls = content["controls"].as_array().into_iter().flatten();
        for control in controls.filter(|control| control["shown"] == true) {
            for (kind, is_mark) in DRAW_KINDS {
                for draw in control[kind].as_array().into_iter().flatten() {
                    let shows = draw_shows(rendered, region, control, draw, is_mark);
                    checked += usize::from(shows.is_some());
                    if shows == Some(false) {
                        failures.push(format!(
                            "{} {region} {} {kind}: {draw}",
                            rendered.name, control["key"]
                        ));
                    }
                }
            }
        }
    }
    // Every fixture draws colored controls, so a state with nothing checked means the dump
    // lost its draws, not that the frame matched.
    if checked == 0 {
        failures.push(format!("{}: no claimed color was checked", rendered.name));
    }
    failures
}

fn collect_states(directory: &Path, states: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", directory.display()));
    for entry in entries {
        let path = entry.expect("fixture entry reads").path();
        if path.is_dir() {
            collect_states(&path, states);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            states.push(path);
        }
    }
}

fn run_headless(state: &Path, flag: &str, output: &Path) {
    let run = Command::new(env!("CARGO_BIN_EXE_oc-app"))
        .arg(flag)
        .arg(output)
        .arg("--state")
        .arg(state)
        .output()
        .expect("oc-app starts");
    assert!(
        run.status.success(),
        "oc-app {flag} {} failed: {}\n{}",
        state.display(),
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Renders one state as a dump and as a frame, in a scratch folder of its own.
fn render(state: &Path, scratch: &Path) -> Rendered {
    let states = Path::new(env!("OC_DESKTOP_FIXTURES")).join("states");
    let name = state
        .strip_prefix(&states)
        .unwrap_or(state)
        .with_extension("")
        .to_string_lossy()
        .into_owned();
    let stem = name.replace('/', "-");
    let dump_path = scratch.join(format!("{stem}.json"));
    let frame_path = scratch.join(format!("{stem}.png"));
    run_headless(state, "--dump-layout", &dump_path);
    run_headless(state, "--screenshot", &frame_path);
    let dump = std::fs::read(&dump_path).expect("the dump reads");
    let layout: Value = serde_json::from_slice(&dump).expect("the dump is JSON");
    let frame = image::open(&frame_path)
        .expect("the frame reads")
        .to_rgba8();
    let scale = layout["scaleFactor"]
        .as_f64()
        .expect("the dump has a scale");
    Rendered {
        name,
        layout,
        frame,
        scale,
    }
}

#[test]
fn every_claimed_color_shows_in_the_frame() {
    let mut states = Vec::new();
    collect_states(
        &Path::new(env!("OC_DESKTOP_FIXTURES")).join("states"),
        &mut states,
    );
    states.sort();
    let scratch = std::env::temp_dir().join(format!("oc-frame-check-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch folder is created");
    let queue = Mutex::new(states);
    let failures = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(2, |count| count.get().min(6));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(state) = queue.lock().expect("queue lock").pop() {
                    let found = frame_failures(&render(&state, &scratch));
                    failures.lock().expect("failure lock").extend(found);
                }
            });
        }
    });
    std::fs::remove_dir_all(&scratch).ok();
    let mut failures = failures.into_inner().expect("failure lock");
    failures.sort();
    assert!(
        failures.is_empty(),
        "the frame does not show these claimed colors:\n{}",
        failures.join("\n")
    );
}
