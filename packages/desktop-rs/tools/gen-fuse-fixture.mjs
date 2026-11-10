#!/usr/bin/env node
/**
 * Generates the search fixtures the Rust port of `rankByQuery` is checked against.
 *
 * Runs the React implementation (`packages/ui/src/lib/search/fuzzySearch.ts`) and Fuse.js 7.1.0
 * from `packages/ui/node_modules` over one fixed corpus and one fixed query set, both held in this
 * file, and writes:
 *
 * - `fixtures/search/fuse.json`: every Fuse result (`refIndex`, `score`) for the corpus with the
 *   options `rankByQuery` passes (threshold 0.35, ignoreLocation, distance 100, includeScore,
 *   minMatchCharLength 2).
 * - `fixtures/search/rank.json`: `rankByQuery` result indices and `matchesRankQuery` booleans for
 *   the same corpus.
 *
 * The queries cover typos generated deterministically from corpus words, short tokens,
 * multi-token queries, punctuation-only queries, non-ASCII text (Vietnamese, Cyrillic, Latin
 * diacritics, astral characters) and a query longer than Fuse's 32-code-unit pattern chunk.
 *
 * TypeScript runs through Node's type stripping (Node 22.6+ with the flag, 22.18+ by default);
 * `fuzzySearch.ts` imports only Fuse, so no loader is needed. Both outputs are held in memory and
 * written only after a check that no personal path or repository path appears in them.
 *
 * Usage: node packages/desktop-rs/tools/gen-fuse-fixture.mjs [--out <dir>]
 */

import { createRequire } from "node:module"
import { existsSync, mkdirSync, writeFileSync } from "node:fs"
import { homedir, userInfo } from "node:os"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import process from "node:process"

const findRepoRoot = (start) => {
  for (let dir = start; ; dir = dirname(dir)) {
    if (existsSync(join(dir, "packages/web/bin/cli.js"))) return dir
    if (dirname(dir) === dir) throw new Error(`No repository root (packages/web/bin/cli.js) above ${start}`)
  }
}

const TOOL_DIR = dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = findRepoRoot(TOOL_DIR)
const PACKAGE_ROOT = join(REPO_ROOT, "packages/desktop-rs")
const UI_ROOT = join(REPO_ROOT, "packages/ui")
const EXPECTED_FUSE_VERSION = "7.1.0"

// The options `rankByQuery` passes in `packages/ui/src/lib/search/fuzzySearch.ts:148-151`.
const FUSE_OPTIONS = {
  threshold: 0.35,
  ignoreLocation: true,
  distance: 100,
  includeScore: true,
  minMatchCharLength: 2,
}

const parseArgs = (argv) => {
  const options = { out: join(PACKAGE_ROOT, "fixtures") }
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--out") options.out = resolve(argv[++index])
    else throw new Error(`Unknown option: ${argv[index]}`)
  }
  return options
}

// Picker-like labels: projects, paths and branches, models and providers, agents.
const CORPUS = [
  "OpenChamber",
  "OpenChamber Next",
  "openchamber-next",
  "desktop-rs pickers",
  "feat/desktop-rs-pickers",
  "feat/fix-scroll",
  "fix/sidebar-scroll",
  "workspace-fix",
  "prefixed-thing",
  "feat/scroll-anchored-chat",
  "fix/chat-header",
  "feat/scroll-perf",
  "main",
  "maintenance",
  "release/main",
  "feat/main-menu",
  "sidebar",
  "sidebar-collapse",
  "chats",
  "Chat session",
  "New session",
  "Dự án OpenChamber",
  "Ứng dụng trò chuyện",
  "Trò chuyện mới",
  "Đồ án cuối kỳ",
  "dự án cá nhân",
  "Розбір бенчу",
  "Перебір",
  "Über-Projekt",
  "Café résumé",
  "naïve-search",
  "東京プロジェクト",
  "🚀 Dự án mới",
  "Projet 🌍 été",
  "𐐀𐐨 Deseret",
  "Zhipu",
  "GLM-5.3",
  "claude-sonnet-5",
  "gpt-4o-mini",
  "gpt-5-codex",
  "opencode/big-pickle",
  "anthropic/claude-opus-4-1",
  "google/gemini-2.5-pro",
  "build",
  "Plan",
  "explore",
  "general",
  "OpenChamber shared web desktop VS Code hosted mobile and native mobile UI surfaces for OpenCode",
  "Dự án OpenChamber chia sẻ giao diện web cho mọi nền tảng máy tính",
  "",
]

// A deterministic typo set: each helper works on ASCII words at a fixed position.
const swapNeighbours = (word, at) => word.slice(0, at) + word[at + 1] + word[at] + word.slice(at + 2)
const dropCharacter = (word, at) => word.slice(0, at) + word.slice(at + 1)

const longEnglish = CORPUS.find((entry) => entry.startsWith("OpenChamber shared web"))
const longVietnamese = CORPUS.find((entry) => entry.startsWith("Dự án OpenChamber chia sẻ"))
const vietnameseLongPrefix = longVietnamese.slice(0, 36)

const FUSE_QUERIES = [
  // Exact and prefix tokens.
  "fix",
  "main",
  "OpenChamber",
  // Multi-token, matched in any order.
  "chat scroll",
  "zhipu glm",
  // Punctuation-insensitive and punctuation-only tokens.
  "gpt4o",
  "sonnet5",
  "...",
  "--",
  // Non-ASCII text, including uppercase forms that must lowercase like JavaScript.
  "dự án",
  "DỰ ÁN",
  "trò chuyện",
  "đồ án",
  "бенч",
  "перебір",
  "über",
  "café",
  // Short tokens, below the fuzzy fallback and minMatchCharLength boundaries.
  "a",
  "ab",
  "oc",
  "rs",
  // Typos: Fuse's fallback is reached for these single tokens of three or more units.
  "worskpace",
  swapNeighbours("workspace", 4),
  dropCharacter("openchamber", 4),
  dropCharacter("sidebar", 3),
  dropCharacter("desktop", 3),
  dropCharacter("sonnet", 3),
  dropCharacter("gemini", 3),
  dropCharacter("pickle", 3),
  // Longer than Fuse's 32-code-unit pattern chunk, with ASCII and non-ASCII text.
  longEnglish.slice(0, 40),
  vietnameseLongPrefix,
]

// Queries that only `rankByQuery` and `matchesRankQuery` have to answer: an empty query lists
// everything and a whitespace-only query is the same after trimming.
const RANK_ONLY_QUERIES = ["", "  "]

const requireFromUi = createRequire(join(UI_ROOT, "package.json"))
const Fuse = requireFromUi("fuse.js")
if (Fuse.version !== EXPECTED_FUSE_VERSION) {
  throw new Error(`fuse.js ${EXPECTED_FUSE_VERSION} expected in packages/ui/node_modules, found ${Fuse.version}`)
}

const loadFuzzySearch = async () => {
  if (!process.features.typescript) {
    throw new Error("This script reads packages/ui/src/lib/search/fuzzySearch.ts; run it with --experimental-strip-types on Node older than 22.18")
  }
  const module = await import(pathToFileURL(join(UI_ROOT, "src/lib/search/fuzzySearch.ts")).href)
  return { rankByQuery: module.rankByQuery, matchesRankQuery: module.matchesRankQuery }
}

const assertUnique = (values, label) => {
  const seen = new Set()
  for (const value of values) {
    if (seen.has(value)) throw new Error(`${label} repeats ${JSON.stringify(value)}`)
    seen.add(value)
  }
}

const assertNoPersonalPaths = (outputs) => {
  const username = userInfo().username
  const forbidden = [
    { label: "the home directory", test: (text) => text.includes(homedir()) },
    { label: "the repository root", test: (text) => text.includes(REPO_ROOT) },
    { label: "the user name as a path segment", test: (text) => username.length > 0 && [`/${username}/`, `\\\\${username}\\\\`, `/${username}"`].some((needle) => text.includes(needle)) },
  ]
  const hits = []
  for (const [file, content] of outputs) {
    for (const rule of forbidden) if (rule.test(content)) hits.push(`${file}: contains ${rule.label}`)
  }
  if (hits.length > 0) throw new Error(`Refusing to write fixtures with personal paths:\n${hits.join("\n")}`)
}

const main = async () => {
  const { out } = parseArgs(process.argv.slice(2))
  assertUnique(CORPUS, "the corpus")
  assertUnique(FUSE_QUERIES, "the Fuse query set")
  const { rankByQuery, matchesRankQuery } = await loadFuzzySearch()

  const fuse = new Fuse(CORPUS, FUSE_OPTIONS)
  const fuseOutput = {
    note: "Generated by packages/desktop-rs/tools/gen-fuse-fixture.mjs; do not edit by hand.",
    fuseVersion: Fuse.version,
    options: FUSE_OPTIONS,
    corpus: CORPUS,
    queries: FUSE_QUERIES.map((query) => ({
      query,
      results: fuse.search(query).map(({ refIndex, score }) => ({ refIndex, score })),
    })),
  }

  const indexOf = new Map(CORPUS.map((entry, index) => [entry, index]))
  const rankOutput = {
    note: "Generated by packages/desktop-rs/tools/gen-fuse-fixture.mjs; do not edit by hand.",
    corpus: CORPUS,
    queries: [...FUSE_QUERIES, ...RANK_ONLY_QUERIES].map((query) => ({
      query,
      ranked: rankByQuery(CORPUS, query, (item) => [item]).map((item) => indexOf.get(item)),
      matched: CORPUS.map((item) => matchesRankQuery([item], query)),
    })),
  }

  const searchDir = join(out, "search")
  const files = [
    [join(searchDir, "fuse.json"), `${JSON.stringify(fuseOutput, null, 2)}\n`],
    [join(searchDir, "rank.json"), `${JSON.stringify(rankOutput, null, 2)}\n`],
  ]
  assertNoPersonalPaths(files)
  mkdirSync(searchDir, { recursive: true })
  for (const [file, content] of files) writeFileSync(file, content)
  console.log(`Corpus: ${CORPUS.length} strings; Fuse queries: ${FUSE_QUERIES.length}; rank queries: ${rankOutput.queries.length}`)
  console.log(`Wrote ${files.map(([file]) => file).join(", ")}`)
}

main().catch((error) => {
  console.error(`gen-fuse-fixture failed: ${error.message}`)
  process.exitCode = 1
})
