#!/usr/bin/env node
// Writes every symbol of React's Remix sprite as an SVG file and the Rust icon index.
//
//   node tools/gen-remix-icons.mjs
//
// `packages/ui/src/components/icon/sprite.ts` is the source of truth: each symbol becomes
// `crates/oc-app/assets/icons/remix/<name>.svg`, a 24x24 viewBox around the symbol's content
// with its `currentColor` fills kept so GPUI tints them. The same run rewrites
// `crates/oc-app/src/icons/remix.rs` (the enum and `IconNamed` impl), `remix_all.rs` (the
// ordered list), `remix_names.rs` (the name match) and `remix_files.rs` (the embedded bytes).
// A test in `src/icons/` fails when they disagree.

import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
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

const remix = `//! Remix icons, generated from \`packages/ui/src/components/icon/sprite.ts\`; do not edit.
//!
//! Do not edit by hand: \`node tools/gen-remix-icons.mjs\` rewrites this file and the SVG
//! files it points at. Names are React's sprite symbols, one per icon file. The sprite order
//! lives in \`remix_all.rs\` and the name match in \`remix_names.rs\`.

use gpui_kit::SharedString;
use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{App, IntoElement, RenderOnce, Window};

#[path = "remix_all.rs"]
mod remix_all;
#[path = "remix_names.rs"]
mod remix_names;

/// One symbol of React's Remix sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoElement)]
#[rustfmt::skip]
pub(crate) enum RemixIcon {
${enumBody}
}

impl IconNamed for RemixIcon {
    #[rustfmt::skip]
    fn path(self) -> SharedString {
        format!("icons/remix/{}.svg", self.name()).into()
    }
}

impl RenderOnce for RemixIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}
`;

const remixAll = `//! Every Remix icon in sprite order, generated by \`tools/gen-remix-icons.mjs\`; do not edit.

use super::RemixIcon;

impl RemixIcon {
    /// Every icon, in sprite order.
    #[rustfmt::skip]
    pub(crate) const ALL: &[RemixIcon] = &[
${allBody}
    ];
}
`;

const remixNames = `//! The sprite symbol name of every Remix icon, generated by
//! \`tools/gen-remix-icons.mjs\`; do not edit.

use super::RemixIcon;

impl RemixIcon {
    /// The sprite symbol's name, the identity the parity reference records.
    #[rustfmt::skip]
    pub(crate) fn name(self) -> &'static str {
        match self {
${nameArms}
        }
    }
}
`;

const files = `//! The Remix SVG bytes, generated by \`tools/gen-remix-icons.mjs\`; do not edit.
//!
//! Each file is embedded from \`assets/icons/remix/\` by the \`asset_bytes!\` macro, which
//! carries one full path per icon.

/// Every Remix icon's SVG, in sprite order, keyed by sprite symbol.
#[rustfmt::skip]
pub(super) const FILES: &[(&str, &[u8])] = &[
${lines(symbols.map(({ name }) => `("${name}", asset_bytes!("/assets/icons/remix/${name}.svg")),`), 2)
  .map((row) => `    ${row}`)
  .join('\n')}
];
`;

mkdirSync(sourceDirectory, { recursive: true });
writeFileSync(join(sourceDirectory, 'remix.rs'), remix);
writeFileSync(join(sourceDirectory, 'remix_all.rs'), remixAll);
writeFileSync(join(sourceDirectory, 'remix_names.rs'), remixNames);
writeFileSync(join(sourceDirectory, 'remix_files.rs'), files);

// Provider logos: the folder React bundles, copied whole, plus the embedded index.
const logosSource = join(root, 'packages', 'ui', 'src', 'assets', 'provider-logos');
const logosDirectory = join(crate, 'assets', 'provider-logos');
rmSync(logosDirectory, { recursive: true, force: true });
mkdirSync(logosDirectory, { recursive: true });
const logoIds = readdirSync(logosSource)
  .filter((name) => name.endsWith('.svg'))
  .map((name) => name.slice(0, -4))
  .sort();
for (const id of logoIds) {
  copyFileSync(join(logosSource, `${id}.svg`), join(logosDirectory, `${id}.svg`));
}
const providerLogos = `//! The provider logos React bundles, generated by \`tools/gen-remix-icons.mjs\`; do not edit.
//!
//! Each file is embedded from \`assets/provider-logos/\` by the \`asset_bytes!\` macro, which
//! carries one full path per logo.

/// Every bundled provider logo, keyed by file name without extension.
#[rustfmt::skip]
pub(super) const FILES: &[(&str, &[u8])] = &[
${lines(logoIds.map((id) => `("${id}", asset_bytes!("/assets/provider-logos/${id}.svg")),`), 2)
  .map((row) => `    ${row}`)
  .join('\n')}
];
`;
writeFileSync(join(sourceDirectory, 'provider_logos.rs'), providerLogos);

// React's own SVG icons: their path data, copied from the components that draw them.
const customDirectory = join(crate, 'assets', 'icons', 'custom');
mkdirSync(customDirectory, { recursive: true });
const custom = {
  'stop': `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M208,56V200a8,8,0,0,1-8,8H56a8,8,0,0,1-8-8V56a8,8,0,0,1,8-8H200A8,8,0,0,1,208,56Z" opacity="0.2"/><path d="M200,40H56A16,16,0,0,0,40,56V200a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V56A16,16,0,0,0,200,40Zm0,160H56V56H200V200Z"/></svg>\n`,
  'context-ring': `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><circle cx="9" cy="9" r="8" fill="none" stroke="currentColor" stroke-opacity="0.3" stroke-width="2"/></svg>\n`,
  'arrows-merge': `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M232.49,192.49l-32,32a12,12,0,0,1-17,0l-32-32a12,12,0,0,1,17-17L180,187V141L128,89,76,141V187l11.51-11.52a12,12,0,0,1,17,17l-32,32a12,12,0,0,1-17,0l-32-32a12,12,0,1,1,17-17L52,187V136a12,12,0,0,1,3.51-8.49L116,67V24a12,12,0,0,1,24,0V67l60.49,60.48A12,12,0,0,1,204,136v51l11.51-11.52a12,12,0,0,1,17,17Z" transform="rotate(180 128 128)"/></svg>\n`,
};
for (const [name, svg] of Object.entries(custom)) {
  writeFileSync(join(customDirectory, `${name}.svg`), svg);
}

console.log(`gen-remix-icons: wrote ${symbols.length} icons to assets/icons/remix, ${logoIds.length} logos and ${Object.keys(custom).length} custom icons`);
