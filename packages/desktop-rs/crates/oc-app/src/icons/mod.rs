//! The Remix icon set and the asset source that serves it.
//!
//! React draws its icons from the Remix sprite in
//! `packages/ui/src/components/icon/sprite.ts`. [`RemixIcon`] names one symbol per file and
//! [`AppAssets`] answers GPUI's asset paths: `icons/remix/<name>.svg` from the embedded
//! sprite, and the `icons/<lucide>.svg` paths gpui-kit components draw for themselves from
//! [`SUBSTITUTIONS`], the one table that maps each of those to the Remix symbol React draws
//! in the same place. Every other path goes to gpui-kit's own bundle.
//!
//! The table is the rule that keeps a gpui-kit icon off the screen. A test renders every
//! fixture state headless and fails when the asset source is asked for a gpui-kit path the
//! table does not answer.

mod assets;
mod remix;
mod remix_files;

pub(crate) use assets::AppAssets;
pub(crate) use remix::RemixIcon;

#[cfg(test)]
mod icons_test;
