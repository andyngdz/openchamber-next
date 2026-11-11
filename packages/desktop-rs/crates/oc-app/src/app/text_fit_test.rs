//! Every fixture state, drawn headless with its popup, paints all of its text whole.
//!
//! The check reads the glyphs GPUI painted. A glyph its clip cuts on the left or right is text
//! a box hides ("Buil"). A line whose glyphs run past the painted box under its first or last
//! glyph is text spilling out of its box (a menu's hints past the menu's edge). Vertical
//! clipping is a list scrolled part way and stays allowed.
//!
//! GPUI drops a glyph its clip hides whole, so a clip edge that falls between two glyphs leaves
//! nothing here to see; the composer triggers check their label widths for that in
//! `composer_controls_test.rs`. A label centered in a box and spilling past both edges with no
//! clip is not caught either.

use std::path::PathBuf;

use gpui_kit::{Bounds, Hsla, Quad, ScaledPixels, Scene};

use super::with_settled_window;
use crate::fixture::StateScript;

/// One painted glyph, monochrome or subpixel.
#[derive(Clone, Copy)]
struct Glyph {
    order: u32,
    bounds: Bounds<ScaledPixels>,
    mask: Bounds<ScaledPixels>,
    color: Hsla,
}

fn glyphs(scene: &Scene) -> Vec<Glyph> {
    let mono = scene.monochrome_sprites.iter().map(|sprite| Glyph {
        order: sprite.order,
        bounds: sprite.bounds,
        mask: sprite.content_mask.bounds,
        color: sprite.color,
    });
    let subpixel = scene.subpixel_sprites.iter().map(|sprite| Glyph {
        order: sprite.order,
        bounds: sprite.bounds,
        mask: sprite.content_mask.bounds,
        color: sprite.color,
    });
    // A transparent glyph paints nothing: a control's icon that only shows on hover.
    mono.chain(subpixel)
        .filter(|glyph| !glyph.color.is_transparent())
        .collect()
}

fn left(bounds: &Bounds<ScaledPixels>) -> f32 {
    bounds.origin.x.as_f32()
}

fn right(bounds: &Bounds<ScaledPixels>) -> f32 {
    bounds.origin.x.as_f32() + bounds.size.width.as_f32()
}

fn top(bounds: &Bounds<ScaledPixels>) -> f32 {
    bounds.origin.y.as_f32()
}

fn bottom(bounds: &Bounds<ScaledPixels>) -> f32 {
    bounds.origin.y.as_f32() + bounds.size.height.as_f32()
}

/// Whether `glyph` belongs to `line`: one layer, one color, one clip, one row. GPUI paints a
/// shaped line on one layer, but the scene sorts sprites by atlas texture within a layer, so
/// a line's glyphs need not sit next to each other in the scene.
fn joins(line: &[Glyph], glyph: &Glyph) -> bool {
    let Some(first) = line.first() else {
        return false;
    };
    let band_top = line
        .iter()
        .map(|g| top(&g.bounds))
        .fold(f32::INFINITY, f32::min);
    let band_bottom = line
        .iter()
        .map(|g| bottom(&g.bounds))
        .fold(f32::NEG_INFINITY, f32::max);
    first.order == glyph.order
        && first.color == glyph.color
        && first.mask == glyph.mask
        && top(&glyph.bounds) < band_bottom
        && band_top < bottom(&glyph.bounds)
}

/// The painted lines, each sorted left to right.
fn lines(glyphs: &[Glyph]) -> Vec<Vec<Glyph>> {
    let mut lines: Vec<Vec<Glyph>> = Vec::new();
    for glyph in glyphs {
        match lines.iter_mut().find(|line| joins(line, glyph)) {
            Some(line) => line.push(*glyph),
            None => lines.push(vec![*glyph]),
        }
    }
    for line in &mut lines {
        line.sort_by(|a, b| left(&a.bounds).total_cmp(&left(&b.bounds)));
    }
    lines
}

fn painted(quad: &Quad) -> bool {
    !quad.background.is_transparent()
        || (!quad.border_color.is_transparent()
            && quad.border_widths.any(|width| width.as_f32() > 0.))
}

fn contains(outer: &Bounds<ScaledPixels>, inner: &Bounds<ScaledPixels>) -> bool {
    left(inner) >= left(outer)
        && right(inner) <= right(outer)
        && top(inner) >= top(outer)
        && bottom(inner) <= bottom(outer)
}

/// The topmost painted box under `glyph`: the last box drawn below it that holds it.
fn box_under<'a>(quads: &'a [Quad], glyph: &Glyph) -> Option<&'a Quad> {
    quads
        .iter()
        .filter(|quad| quad.order < glyph.order && painted(quad))
        .filter(|quad| contains(&quad.bounds, &glyph.bounds))
        .max_by_key(|quad| quad.order)
}

fn text_problems(scene: &Scene) -> Vec<String> {
    let glyphs = glyphs(scene);
    let mut problems = Vec::new();
    for glyph in &glyphs {
        let cut =
            left(&glyph.bounds) < left(&glyph.mask) || right(&glyph.bounds) > right(&glyph.mask);
        if cut {
            problems.push(format!(
                "a glyph at {:?} is cut by its clip {:?}",
                glyph.bounds, glyph.mask
            ));
        }
    }
    for line in lines(&glyphs) {
        let (Some(first), Some(last)) = (line.first(), line.last()) else {
            continue;
        };
        // The line's box holds its start or its end: a label spills right past a left-aligned
        // box and left past a right-aligned one.
        let under = [
            box_under(&scene.quads, first),
            box_under(&scene.quads, last),
        ]
        .into_iter()
        .flatten()
        .max_by_key(|quad| quad.order);
        let Some(under) = under else { continue };
        if let Some(out) = line.iter().find(|glyph| {
            left(&glyph.bounds) < left(&under.bounds) || right(&glyph.bounds) > right(&under.bounds)
        }) {
            problems.push(format!(
                "a line starting at {:?} runs past its box {:?} at {:?}",
                first.bounds, under.bounds, out.bounds
            ));
        }
    }
    problems
}

fn states() -> Vec<PathBuf> {
    let dir = std::path::Path::new(env!("OC_DESKTOP_FIXTURES")).join("states");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures/states reads")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn every_state_paints_its_text_whole() {
    let mut failures = Vec::new();
    for path in states() {
        let (state, popover) = StateScript::load(&path)
            .expect("state fixture loads")
            .into_parts();
        let problems = with_settled_window(state, popover, |headless, window, _| {
            headless
                .update_window(window, |_, window, _| {
                    text_problems(window.rendered_scene())
                })
                .map_err(super::render_error)
        })
        .expect("state renders headless");
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned());
        failures.extend(
            problems
                .into_iter()
                .map(|problem| format!("{}: {problem}", name.as_deref().unwrap_or("?"))),
        );
    }
    assert!(
        failures.is_empty(),
        "text not painted whole:\n{}",
        failures.join("\n")
    );
}
