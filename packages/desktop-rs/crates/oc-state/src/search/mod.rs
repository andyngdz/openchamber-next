//! Picker search: the React token ranking (`rank_by_query`, `matches_rank_query`) and the parts
//! of Fuse.js 7.1.0 it falls back to.
//!
//! The source of truth is `packages/ui/src/lib/search/fuzzySearch.ts`; Fuse.js comes from
//! `packages/ui/node_modules/fuse.js/dist/fuse.mjs`. `tools/gen-fuse-fixture.mjs` runs both over
//! one fixed corpus and query set into `fixtures/search/`, and the tests here hold this port to
//! that output: same indices in the same order, scores within the 2-ULP difference the one `pow`
//! call allows (design.md "Testing").
//!
//! Strings are measured in UTF-16 code units throughout, because JavaScript counts them; see
//! [`text`].

mod bitap;
mod fuse;
mod mask;
mod rank;
mod scan;
mod text;
mod token;

pub use rank::{RankByQueryOptions, matches_rank_query, rank_by_query};

#[cfg(test)]
mod fixture_test;
