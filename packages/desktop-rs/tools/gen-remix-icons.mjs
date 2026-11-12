#!/usr/bin/env node
// Writes every symbol of React's Remix sprite as an SVG file and the Rust icon index.
//
//   node tools/gen-remix-icons.mjs
//
// `packages/ui/src/components/icon/sprite.ts` is the source of truth: each symbol becomes
// `crates/oc-app/assets/icons/remix/<name>.svg`, a 24x24 viewBox around the symbol's content
// with its `currentColor` fills kept so GPUI tints them. The same run rewrites
// `crates/oc-app/src/icons/remix.rs` (the names and `IconNamed` impl) and
// `crates/oc-app/src/icons/remix_files.rs` (the embedded bytes). A test in `src/icons/` fails
// when the three disagree.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

/** Walks up from `start` to the checkout that holds the React icon sprite. */
function findRepoRoot(start) {
  let directory = resolve(start);
  for (;;) {
    if (existsSync(join(directory, 'packages', 'ui', 'src', 'components', 'icon', 'sprite.ts'))) {
      return directory;
    }
    const parent = dirname(directory);
    if (parent === directory) {
      throw new Error(`no packages/ui/src/components/icon/sprite.ts above ${start}`);
    }
    directory = parent;
  }
}

/** Reads the sprite's `"name": \`content\`,` lines, in file order. */
function parseSymbols(source) {
  const start = source.indexOf('export const iconSpriteData');
  if (start < 0) {
    throw new Error('sprite.ts has no iconSpriteData export');
  }
  const symbols = [];
  for (const raw of source.slice(start).split('\n')) {
    const match = /^"([a-z0-9-]+)":\s*`(.*)`(?:,)?$/.exec(raw.trim());
    if (match) {
      symbols.push({ name: match[1], content: match[2] });
    }
  }
  if (symbols.length === 0) {
    throw new Error('sprite.ts holds no symbols');
  }
  const seen = new Set();
  for (const { name } of symbols) {
    if (seen.has(name)) {
      throw new Error(`sprite.ts repeats the symbol ${name}`);
    }
    seen.add(name);
  }
  return symbols;
}

/** `arrow-down-s` -> `ArrowDownS`, the Rust name of the symbol. */
function variantOf(name) {
  return name
    .split('-')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join('');
}

/** Breaks `items` into lines of at most `size`, joined by spaces. */
function lines(items, size) {
  const rows = [];
  for (let index = 0; index < items.length; index += size) {
    rows.push(items.slice(index, index + size).join(' '));
  }
  return rows;
}

const root = findRepoRoot(here);
const crate = join(root, 'packages', 'desktop-rs', 'crates', 'oc-app');
const spritePath = join(root, 'packages', 'ui', 'src', 'components', 'icon', 'sprite.ts');
const iconsDirectory = join(crate, 'assets', 'icons', 'remix');
const sourceDirectory = join(crate, 'src', 'icons');

const symbols = parseSymbols(readFileSync(spritePath, 'utf8'));
const seenVariants = new Set();
for (const symbol of symbols) {
  symbol.variant = variantOf(symbol.name);
  if (seenVariants.has(symbol.variant)) {
    throw new Error(`two sprite names map to RemixIcon::${symbol.variant}`);
  }
  seenVariants.add(symbol.variant);
}

rmSync(iconsDirectory, { recursive: true, force: true });
mkdirSync(iconsDirectory, { recursive: true });
for (const { name, content } of symbols) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">${content}</svg>\n`;
  writeFileSync(join(iconsDirectory, `${name}.svg`), svg);
}

const enumBody = lines(symbols.map(({ variant }) => `${variant},`), 8)
  .map((row) => `    ${row}`)
  .join('\n');
const allBody = lines(symbols.map(({ variant }) => `RemixIcon::${variant},`), 5)
  .map((row) => `        ${row}`)
  .join('\n');
const nameArms = lines(symbols.map(({ name, variant }) => `Self::${variant} => "${name}",`), 5)
  .map((row) => `            ${row}`)
  .join('\n');

const remix = `//! Remix icons, generated from \`packages/ui/src/components/icon/sprite.ts\`.
//!
//! Do not edit by hand: \`node tools/gen-remix-icons.mjs\` rewrites this file and the SVG
//! files it points at. Names are React's sprite symbols, one per icon file.

use gpui_kit::SharedString;
use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{App, IntoElement, RenderOnce, Window};

/// One symbol of React's Remix sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoElement)]
#[rustfmt::skip]
pub(crate) enum RemixIcon {
${enumBody}
}

impl RemixIcon {
    /// Every icon, in sprite order.
    #[rustfmt::skip]
    pub(crate) const ALL: &[RemixIcon] = &[
${allBody}
    ];
}

impl IconNamed for RemixIcon {
    #[rustfmt::skip]
    fn path(self) -> SharedString {
        let name = match self {
${nameArms}
        };
        format!("icons/remix/{name}.svg").into()
    }
}

impl RenderOnce for RemixIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}
`;

const files = `//! The Remix SVG bytes, generated by \`tools/gen-remix-icons.mjs\`; do not edit.
//!
//! Each file is embedded from \`assets/icons/remix/\`. The absolute path comes from
//! \`CARGO_MANIFEST_DIR\`, so the generated code carries no parent-relative path.

macro_rules! icon {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/icons/remix/",
                $name,
                ".svg"
            )),
        )
    };
}

/// Every Remix icon's SVG, in sprite order, keyed by sprite symbol.
#[rustfmt::skip]
pub(super) const FILES: &[(&str, &[u8])] = &[
${lines(symbols.map(({ name }) => `icon!("${name}"),`), 4)
  .map((row) => `    ${row}`)
  .join('\n')}
];
`;

mkdirSync(sourceDirectory, { recursive: true });
writeFileSync(join(sourceDirectory, 'remix.rs'), remix);
writeFileSync(join(sourceDirectory, 'remix_files.rs'), files);

console.log(`gen-remix-icons: wrote ${symbols.length} icons to assets/icons/remix`);
