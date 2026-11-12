#!/usr/bin/env node
/**
 * Captures the layout reference the GPUI parity test compares against:
 * `fixtures/reference/<state>.json` and `<state>.png`, taken from the Electron
 * dev window at a 1440x900 content area, and writes the same run's server
 * responses as version 2 state fixtures to `fixtures/states/<state>.json`. Each reference
 * control carries the bounds of the element its key comes from, the measurement the composer
 * and popover layout follows.
 *
 * The window runs isolated from the user's own app: its own HOME (and XDG
 * directories, so OpenCode's database lands there too), OPENCHAMBER_DATA_DIR,
 * Electron profile (OPENCHAMBER_DESKTOP_USER_DATA_DIR, which also avoids the
 * dev profile's single-instance lock), HMR ports, and an OpenCode config that
 * registers the fixture provider (`scripts/perf/fixture-provider.mjs`) twice,
 * under `perf` and `perf-alt`, with several models each. `perf`'s agent model
 * carries two variants so the Thinking menu has content. No window opens on
 * the user's session: Electron starts on Chromium's Ozone headless platform
 * (`--ozone-platform=headless`, sized by `--ozone-override-screen-size`) with
 * DISPLAY, WAYLAND_DISPLAY, XDG_RUNTIME_DIR and DBUS_SESSION_BUS_ADDRESS
 * removed from the child environment, so compositing and CDP screenshots use
 * an offscreen 1440x900 surface. Three headless details make an Electron window
 * behave like the desktop one: the pointer and hover media features are put
 * back with `--blink-settings` (the Ozone platform reports no input device, so
 * hover-revealed controls would otherwise render); the generated main module
 * (`headless-main.mjs`) drops the native tray menu, whose GTK construction
 * segfaults without a display and which sits outside every captured region; and
 * the data dir's settings.json starts with the desktop host choice completed,
 * so the shell never falls back to its first-launch chooser, which mounts the
 * main window and then restarts the app about a minute later. Stub `gh`, `glab`
 * and `gt` executables come first on PATH, since `gh auth token` reads the
 * system keyring even with an empty HOME. Everything lives under one fixed
 * temporary root, removed at the end (kept, with its logs, when a run fails). A
 * fresh Electron profile relaunches the app once shortly after its first start;
 * the script follows the relaunch and waits for a page that stays mounted.
 *
 * Seed (`fixtures/seed.json`): two projects from a settings PUT, one collapsed
 * and one with a colour and an icon; two project sessions; one In work session
 * (metadata `openchamber.work.state=open` through the server's metadata route);
 * one session in the second project; two Chat sessions created under
 * `chatsRoot` with `POST /api/fs/mkdir` and `POST /api/session`, as React
 * creates them. The settings also seed favourites, recents and `lastUsedEfforts`.
 *
 * States: `empty` with no session open; `projects` with the same sidebar (the
 * page does not change between the two captures); `project-popover`,
 * `model-popover` and `agent-menu` opened from the empty state by clicking
 * their trigger through CDP; `session` after one finished turn, scrolled to its
 * end with the mouse wheel; `thinking-menu` from that open session;
 * `streaming` when every tool call has run and the document's first heading is
 * on screen, before its first code block. `server-error` has a state fixture
 * but no reference: Electron runs the server in its own process and replaces
 * the window with its connection screen when the data routes fail, so the
 * shell has no regions to capture. The spike design says the same (archived
 * design.md, "Parity reference").
 *
 * Turn states (`--turn`; design.md "Parity reference"): the permission dock,
 * form dock, web search consent, recovered question, queue strip and Stop
 * button, captured from sessions whose model calls one fixture tool and then
 * streams its document (`bash-1200cps`, `edit-1200cps`, `write-1200cps`,
 * `webfetch-1200cps`, `question-1200cps`; the OpenCode config asks before
 * those tools). Forms OpenCode cannot raise from those models are seeded
 * through the server's own form route (`POST /api/session/<session>/form` with
 * the `global` session for a location-scoped form), the way unit 1 seeded the
 * location-scoped form in `capture-api.mjs`: a form whose first field is the
 * multiselect or free-text question, the `external` link field, and the
 * `websearch.provider` consent card. The edit state seeds
 * `FIXTURE_EDIT_CONTENT` in `FIXTURE_EDIT_FILE` before its turn. Their
 * references and state fixtures go to `reference/turn/` and `states/turn/`;
 * the other states stay where they are and `--turn` writes none of them.
 *
 * Regions and the selectors that find them (REGIONS; language independent,
 * and written to every JSON under `selectors`, the only place a selector
 * appears in the output):
 *
 *   sidebar          [data-left-sidebar], plus the titlebar controls that
 *                    float over its top strip ([data-titlebar-left-controls])
 *   header           [data-page-scroll-lock] > header  (the chat column's
 *                    header, from the sidebar's edge to the window's edge)
 *   chatColumn       [data-workspace-chat]
 *   composer         the parent of the outermost [data-dictation-chain]: the
 *                    composer box with its text area and footer, without the
 *                    riders above it or the project picker and chips around it
 *   workStatusPanel  [data-right-slot]  (the slot that holds the Work status
 *                    card; 300 px card plus its 24 px gutter)
 *   iconRail         nav:has([data-context-surface])
 *   dock             the floating panel above the composer that holds the
 *                    permission, form or consent dock, plus the queue strip's
 *                    top-row group inside the composer box. One region, so a
 *                    queued row and a docked request compare in the same place;
 *                    the panel's bounds win when both are on screen, the strip's
 *                    otherwise (it is optional: a state with neither has none)
 *   popover          the open Base UI popup: the rendered element with role
 *                    `dialog` or `menu` that is inside none of the other
 *                    regions. Both pickers use `@base-ui/react`, which portals
 *                    the popup under body, so no other selector can find it
 *                    without also matching closed menus. One of the first six
 *                    regions is found before a popup is looked for, so the
 *                    rule is "inside no other region", not a fixed ancestor.
 *
 * A control belongs to the first region in the order dock, composer,
 * workStatusPanel, iconRail, header, sidebar, chatColumn, popover that contains
 * it, so the composer's controls are not listed again under chatColumn. A
 * control in no region goes to `outside`, never dropped.
 *
 * A control is an element matched by CONTROL_SELECTOR that is rendered
 * (`checkVisibility` with visibility; not display:none, visibility:hidden or
 * content-visibility:hidden), has a non-empty box, and is not inside an
 * `inert` or `aria-hidden="true"` subtree. Controls that are rendered but
 * transparent until hovered (row actions) are kept with `shown: false`. The
 * page ignores real input for the whole run (Input.setIgnoreInputEvents; the
 * window is on the user's desktop), and the pointer rests over the empty
 * area under the Work status card before every capture, so neither a stray
 * click nor hover changes a capture.
 *
 * Names and keys, all from Chromium's accessibility tree
 * (Accessibility.getAXNodeAndAncestors), never from the DOM's structure:
 *
 *   1. Name: the accessible name. Without one, the control's visible text
 *      (trimmed, whitespace collapsed) when it holds no other control (an
 *      unnamed container's text is its children's: the chat scroller's is
 *      the whole transcript); otherwise its role. Both fallbacks are listed
 *      in `accessibility` with the source used. When that text pairs a label
 *      with a value (a Turn stats row: "Elapsed" and "24.5s"), only the label
 *      names the control, since the value is data, not identity (source
 *      `text label`). The label comes from the row's DOM structure, not from
 *      the joined text: descend while one child element holds all of the
 *      text; where it first splits across two or more child elements, with
 *      no loose text beside them, the first element is the label and the
 *      rest is the value. Text that never splits names the control whole.
 *   2. Timing figures in a name become `<duration>` and `<rate> tok/s`
 *      (NAME_NORMALIZATION), and a name that starts with a seed session's
 *      title is cut to the title, dropping the row's status text ("Session
 *      active", "Active for <duration>"; rule `session-title`).
 *   3. A control named from its contents that holds other controls takes the
 *      name of the first control inside it: the project row is named
 *      "desktop-rs-reference", not "desktop-rs-reference New worktree New
 *      draft session" (rule `container-name`).
 *   4. A control holding another control with the same name (the In work and
 *      Projects section rows around their toggle, a session row around its
 *      title, a project row around its name) gets its role description, or
 *      its role when it has none, as a suffix: `sidebar/Projects (sortable)`
 *      for the row, `sidebar/Projects` for the toggle inside it. The role
 *      description is what a screen reader announces for the row; the tree
 *      offers no other difference between the two.
 *   5. Key: `<region>/<names of the named accessibility ancestors inside the
 *      region, outermost first>/<name with its suffix>`. Ignored nodes and
 *      unnamed ones (generic containers) are skipped; an ancestor that is
 *      itself a control contributes its name from 1 to 3, without a suffix,
 *      and only when that name is an accessible name, not a fallback;
 *      the region's own root is not a segment. A segment equal to the one
 *      after it is not repeated, so the toggle inside the Projects row is
 *      `sidebar/Projects`, not `sidebar/Projects/Projects`. Keys are compared
 *      as whole strings; a name may itself contain "/".
 *   6. A key that still repeats (one per tool call in a turn, one per code
 *      block: no message or turn has a name in the tree to tell them apart)
 *      gets `#<n>` in DOM order, and each is listed in `accessibility` as a
 *      repeated key.
 *
 * `normalized` in each JSON lists every control rules 2 and 3 changed, with
 * the rules applied.
 * Nothing else is normalized or dropped.
 *
 * What each control draws (design.md "Parity reference"), for every reference
 * state: `icons`, in DOM order, each svg (`name` from its `<use href="#oc-NAME>`
 * symbol, `other` for an inline svg, with its color) and each img (`name` as
 * `img:<file name without extension>`, such as a provider id, with no color);
 * `marks`, each textless, svg-less descendant with a non-transparent
 * background-color and both sides at most 12 px, with the fill color; and
 * `texts`, each run of visible text with its color, adjacent same-color text
 * nodes on one line merged. Colors are `#rrggbbaa` in sRGB: the computed value
 * is painted on a 1x1 canvas and read back, and the element's own and its
 * ancestors' opacity multiply into the alpha. Icons, marks and text runs whose
 * color is fully transparent at rest (hover-revealed actions) are left out.
 * Each icon, mark and text run carries its bounds.
 *
 * State fixtures (version 2, `fixtures/states/<state>.json`): `home` is the
 * `/api/fs/home` response, `settings` the `surface=desktop` settings document,
 * each `directories` entry the config documents and the provider, model, agent
 * and session payloads for that directory, `draft` the composer target, `steps`
 * the reducer steps (connection, the open session and its messages), and `open`
 * the popup a state shows, when it shows one. Session and message ids and every
 * timestamp are rewritten to stable values, and instance paths and durations
 * to fixed ones, so two runs produce byte-identical JSON while the records keep
 * their shape and order. A turn state adds a `turn` block with the startup
 * snapshots the app reads beside `/api/event`: `permissions` and `forms` as
 * `GET /api/permission/request` and `GET /api/form` answer them, `queue` as
 * `GET /api/message-queue` does, `autoAccept` as
 * `GET /api/permission-auto-accept` does, `routing` as `GET /api/routing`
 * does, plus the composer text and the dock's local choices (a selected
 * permission step and the expanded queue strip, which no reducer holds).
 *
 * The fixtures are committed, so no JSON may name the user: the run fails and
 * writes nothing when one contains the home directory, the repository path or
 * the user name. No path carries a `../` segment.
 *
 * Usage: node packages/desktop-rs/tools/capture-reference.mjs [--output <dir>]
 *          [--states-output <dir>] [--states <names>] [--turn]
 */

import { spawn, spawnSync } from "node:child_process"
import { createWriteStream, existsSync, mkdirSync, readFileSync, readdirSync, readlinkSync, rmSync, writeFileSync } from "node:fs"
import { connect } from "node:net"
import { homedir, tmpdir, userInfo } from "node:os"
import { basename, dirname, join, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import process from "node:process"
import { inflateSync } from "node:zlib"

const findRepoRoot = (start) => {
  for (let dir = start; ; dir = dirname(dir)) {
    if (existsSync(join(dir, "packages/web/bin/cli.js"))) return dir
    if (dirname(dir) === dir) throw new Error(`No repository root (packages/web/bin/cli.js) above ${start}`)
  }
}

const REPO_ROOT = findRepoRoot(dirname(fileURLToPath(import.meta.url)))
const PACKAGE_ROOT = join(REPO_ROOT, "packages/desktop-rs")
const importPerf = (name) => import(pathToFileURL(join(REPO_ROOT, "scripts/perf", name)).href)
const { CdpClient, findPageTarget, reservePort, wait } = await importPerf("cdp.mjs")
const { FIXTURE_EDIT_CONTENT, FIXTURE_EDIT_FILE, FIXTURE_HELD_TEXT, FIXTURE_TOOL_RATE, fixtureProviderConfig } = await importPerf("fixture-provider.mjs")
const { ensureProject, isolatedEnv, startFixture } = await importPerf("isolated-server.mjs")
// A fixed root, so paths the UI shows are the same on every run. The CDP and
// fixture ports are fixed too, so two runs cannot overlap and provider
// settings do not carry a new port each time; both must be free before a run.
const ROOT = join(tmpdir(), "oc-desktop-rs-reference")
const CDP_PORT = 9222
const FIXTURE_PORT = 4601
const VIEWPORT = { width: 1440, height: 900 }
const SIDEBAR_WIDTH = 280
// Chromium's offscreen Ozone platform: no window on the user's session, and a
// content surface the size of the reference viewport. Without the size switch
// the platform reports a 1x1 screen, so no frame is composited and CDP cannot
// take a screenshot. The Ozone platform has no input devices, so it reports
// `pointer: none` and `hover: none`; the blink settings put back the fine
// pointer and hover a desktop session has, which decides pointer-fine CSS.
const HEADLESS_FLAGS = [
  "--ozone-platform=headless",
  `--ozone-override-screen-size=${VIEWPORT.width},${VIEWPORT.height}`,
  "--blink-settings=primaryPointerType=4,availablePointerTypes=4,primaryHoverType=2,availableHoverTypes=2",
  // No GPU process: the same software raster keeps the capture possible on a
  // loaded host, where a failing GPU helper aborts the shell at startup.
  "--disable-gpu",
]
// The dev shell's display variables; they are dropped so the Ozone platform
// cannot fall back to the user's X11 or Wayland session.
const DISPLAY_VARIABLES = ["DISPLAY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"]
const ELECTRON_DIR = join(REPO_ROOT, "packages/electron")
const ELECTRON_BIN = join(ELECTRON_DIR, "node_modules/.bin/electron")
const RELAUNCH_EXIT_CODE = 75
// The unpackaged dev window draws a 5 px construction-tape strip over a page's
// top edge (packages/electron/dev-window-tape.mjs, inserted on every dom-ready).
// The installed app never shows it, so the capture hides it on every document;
// the tape rule has no !important, so this wins.
const TAPE_HIDE_CSS = "html::after { display: none !important; }"
const TAPE_HIDE_SOURCE = `(() => {
  const add = () => {
    if (document.getElementById("oc-capture-hide-tape")) return
    const style = document.createElement("style")
    style.id = "oc-capture-hide-tape"
    style.textContent = ${JSON.stringify(TAPE_HIDE_CSS)}
    const root = document.head || document.documentElement
    if (root) root.appendChild(style)
  }
  add()
  document.addEventListener("DOMContentLoaded", add, { once: true })
})()`
// The 13 Electron states design.md's "Parity reference" lists, in capture
// order, and the subdirectory they live in.
const TURN_STATES = [
  "permission-bash",
  "permission-edit",
  "permission-write",
  "permission-two",
  "form-choice",
  "form-multiselect",
  "form-text",
  "form-external",
  "websearch-consent",
  "recovered-question",
  "queue-one",
  "queue-three-expanded",
  "busy-with-text",
]
const TURN_DIRECTORY = "turn"
// The states the default run captures (plus server-error, which has a state
// fixture only); --states can name any of them.
const LEGACY_STATES = ["empty", "projects", "project-popover", "model-popover", "agent-menu", "session", "thinking-menu", "streaming", "server-error"]
// WORK_STATUS_COLUMN_WIDTH in packages/ui/src/components/chat/work-status/useWorkStatusVisibility.ts:
// the 300 px card and its 24 px gutter. The card has no persisted width.
const WORK_STATUS_CARD_WIDTH = 300
const WORK_STATUS_COLUMN_WIDTH = 324
// Over the right slot, under the Work status card: no control, no hover reveal.
const POINTER_REST = { x: 1230, y: 820 }
// A failed run still has a server error card; the reasons are fixed so the
// fixture does not depend on an actual crash.
const SERVER_ERROR = {
  message: "The server exited before printing its port (exit status 1)",
  stderrTail: [
    "node:internal/modules/cjs/loader:1228",
    "Error: Cannot find module 'express'",
  ],
  logTail: [
    "[openchamber] Starting OpenChamber server",
    "[openchamber] Failed to load the server module",
  ],
}

// The forge CLIs the server asks for credentials (packages/web/server/lib/github, gitlab, gitea).
const FORGE_CLIS = ["gh", "glab", "gt"]

const HELP = `Usage: node packages/desktop-rs/tools/capture-reference.mjs [--output <dir>]
          [--states-output <dir>] [--states <names>] [--turn]

Starts the Electron dev app headless (Chromium's Ozone headless platform, no
window on the session) and isolated from the user's app, seeds two projects,
their sessions, two Chat sessions and an In work session from fixtures/seed.json,
and writes <state>.json and <state>.png for empty, projects, project-popover,
model-popover, agent-menu, session, thinking-menu and streaming to --output
(default fixtures/reference). It also writes the same run's responses as version
2 state fixtures to fixtures/states, including server-error, which has no
Electron reference.

--turn captures only the 13 turn-control states under ${TURN_DIRECTORY}/ in both
directories; --states limits a run to the named states (comma separated).
Needs ports ${CDP_PORT} (CDP) and ${FIXTURE_PORT} (fixture provider) free and no
display server.`

const parseArgs = (argv) => {
  const options = {
    output: join(PACKAGE_ROOT, "fixtures/reference"),
    statesOutput: join(PACKAGE_ROOT, "fixtures/states"),
    states: null,
    turn: false,
  }
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index]
    if (value === "--help") { console.log(HELP); process.exit(0) }
    else if (value === "--output") options.output = resolve(argv[++index])
    else if (value === "--states-output") options.statesOutput = resolve(argv[++index])
    else if (value === "--states") options.states = argv[++index].split(",").map((name) => name.trim()).filter(Boolean)
    else if (value === "--turn") { options.turn = true; options.states = [...TURN_STATES] }
    else throw new Error(`Unknown option: ${value}`)
  }
  return options
}

/** Runtime.evaluate with a short bound: a page that reloads never answers. */
const evaluateValue = async (client, expression, timeoutMs = 15_000) => {
  const reply = await client.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true }, { timeoutMs })
  if (reply.exceptionDetails) throw new Error(`Page script failed: ${reply.exceptionDetails.exception?.description ?? reply.exceptionDetails.text}`)
  return reply.result?.value ?? null
}

const log = (message) => console.log(`[capture-reference] ${message}`)

const waitFor = async (label, check, { timeoutMs = 60_000, intervalMs = 250 } = {}) => {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const value = await check()
    if (value) return value
    await wait(intervalMs)
  }
  throw new Error(`Timed out after ${timeoutMs / 1000}s waiting for ${label}`)
}

const portInUse = (port) => new Promise((resolvePort) => {
  const socket = connect({ port, host: "127.0.0.1" })
  socket.once("connect", () => { socket.destroy(); resolvePort(true) })
  socket.once("error", () => resolvePort(false))
})

// OpenCode's id format, as `packages/ui/src/lib/opencode/ids.ts` mints message
// ids for a prompt the composer sends. Only the child session's turn is
// prompted through the API; every other turn goes through the composer.
const RANDOM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
let lastStamp = 0
let counter = 0
const messageId = () => {
  const stamp = Date.now()
  if (stamp !== lastStamp) { lastStamp = stamp; counter = 0 }
  counter += 1
  const value = BigInt(stamp) * 0x1000n + BigInt(counter)
  let hex = ""
  for (let index = 0; index < 6; index += 1) hex += Number((value >> BigInt(40 - 8 * index)) & 0xffn).toString(16).padStart(2, "0")
  let random = ""
  for (let index = 0; index < 14; index += 1) random += RANDOM_CHARS[Math.floor(Math.random() * RANDOM_CHARS.length)]
  return `msg_${hex}${random}`
}

// ---------------------------------------------------------------------------
// Isolated window
// ---------------------------------------------------------------------------

/** The fixture provider registered under two ids, as the capture's OpenCode config. */
const referenceProviderConfig = (port, { turn }) => {
  const base = fixtureProviderConfig(port).provider.perf
  const models = (ids) => Object.fromEntries(ids.map((id) => [id, base.models[id]]))
  // The tool models the turn states drive, and the permission rule that makes
  // their calls ask. Both belong to a turn run only: the other states' model
  // picker and config fixtures stay as they were. `think-30s` holds a session
  // busy without streaming, for the queue and Stop states.
  const turnModels = turn
    ? models([...["bash", "edit", "write", "webfetch", "question"].map((name) => `${name}-${FIXTURE_TOOL_RATE}cps`), "think-30s"])
    : {}
  const config = {
    provider: {
      perf: {
        ...base,
        models: {
          ...models(["stream-300cps", "code-1200cps", "agent-20tools-hold"]),
          // v1 spelling: a keyed map. An array drops the whole provider in OpenCode v2.
          "agent-20tools-300cps": {
            ...base.models["agent-20tools-300cps"],
            variants: { high: { reasoningEffort: "high" }, low: { reasoningEffort: "low" } },
          },
          ...turnModels,
        },
      },
      "perf-alt": {
        ...base,
        name: "Performance fixture (second)",
        models: models(["stream-1200cps", "unicode-300cps"]),
      },
    },
  }
  if (turn) config.permission = { bash: "ask", edit: "ask", webfetch: "ask" }
  return config
}

const prepareRoot = (seed) => {
  rmSync(ROOT, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 })
  const paths = {
    home: join(ROOT, "home"),
    dataDir: join(ROOT, "oc-data"),
    userData: join(ROOT, "electron-userdata"),
    project: join(ROOT, seed.project.name),
    project2: join(ROOT, seed.secondProject.name),
    bin: join(ROOT, "bin"),
  }
  for (const dir of [paths.home, paths.dataDir, paths.userData, paths.bin]) mkdirSync(dir, { recursive: true })
  // The profile is a set-up install, not a first launch: without this the
  // desktop shell shows its first-launch chooser about a minute later, after
  // the main window has already mounted, and the chooser restarts the app.
  // The server keeps unknown instance keys when it writes its settings.
  writeFileSync(join(paths.dataDir, "settings.json"), `${JSON.stringify({
    desktopInitialHostChoiceCompleted: true,
    desktopDefaultHostId: "local",
    desktopHosts: [],
  }, null, 2)}\n`)
  // `gh auth token` reads the system keyring even with an empty HOME, and the
  // server would then show the user's GitHub account. Stubs that fail first
  // on PATH keep every forge account out of the window.
  for (const cli of FORGE_CLIS) writeFileSync(join(paths.bin, cli), "#!/bin/sh\nexit 1\n", { mode: 0o755 })
  ensureProject(paths.project)
  ensureProject(paths.project2)
  return paths
}

const windowEnv = ({ paths, seed, fixturePort, uiPort, apiPort, turn }) => {
  const env = isolatedEnv({
    HOME: paths.home,
    PATH: `${paths.bin}:${process.env.PATH ?? ""}`,
    XDG_CONFIG_HOME: join(paths.home, ".config"),
    XDG_DATA_HOME: join(paths.home, ".local/share"),
    XDG_CACHE_HOME: join(paths.home, ".cache"),
    XDG_STATE_HOME: join(paths.home, ".local/state"),
    OPENCHAMBER_DATA_DIR: paths.dataDir,
    OPENCHAMBER_DESKTOP_USER_DATA_DIR: paths.userData,
    OPENCHAMBER_ELECTRON_DEV: "1",
    OPENCHAMBER_DISABLE_PWA_DEV: "1",
    OPENCHAMBER_ELECTRON_REMOTE_DEBUGGING_PORT: String(CDP_PORT),
    OPENCHAMBER_ELECTRON_DEV_RELAUNCH_EXIT_CODE: String(RELAUNCH_EXIT_CODE),
    OPENCHAMBER_HMR_UI_PORT: String(uiPort),
    OPENCHAMBER_HMR_API_PORT: String(apiPort),
    OPENCODE_CONFIG_CONTENT: JSON.stringify({
      ...referenceProviderConfig(fixturePort, { turn }),
      model: seed.model,
      small_model: "perf/stream-1200cps",
      autoupdate: false,
      share: "disabled",
    }),
  })
  // The dev app runs the staged OpenCode the release ships when it exists,
  // as electron-dev.mjs hands it over; otherwise OpenCode comes from PATH.
  const bundledCli = join(ELECTRON_DIR, "resources/opencode-cli", "opencode")
  if (existsSync(bundledCli)) env.OPENCHAMBER_BUNDLED_OPENCODE_CLI_DIR = dirname(bundledCli)
  for (const variable of DISPLAY_VARIABLES) delete env[variable]
  return env
}

const isAlive = (pid) => {
  try { process.kill(pid, 0); return true } catch { return false }
}

/** Processes whose working directory is under ROOT: only this run starts those. */
const processesUnderRoot = () => readdirSync("/proc").filter((entry) => /^\d+$/.test(entry)).flatMap((entry) => {
  try {
    const cwd = readlinkSync(`/proc/${entry}/cwd`)
    return cwd === ROOT || cwd.startsWith(`${ROOT}/`) ? [Number(entry)] : []
  } catch {
    return []
  }
})

/** Stops one child's process group by its own pid: the group, then its pid. */
const stopGroup = async (child) => {
  if (!child?.pid || !isAlive(child.pid)) return
  try { process.kill(-child.pid, "SIGTERM") } catch { /* group already gone */ }
  for (let attempt = 0; attempt < 40 && isAlive(child.pid); attempt += 1) await wait(250)
  try { process.kill(-child.pid, "SIGKILL") } catch { /* exited */ }
}

/**
 * Stops what this run started: the Electron group and the HMR dev server by
 * their own pids, then anything left whose working directory is under ROOT
 * (the OpenCode the server started runs in its own process group with HOME as
 * its working directory).
 */
const stopWindow = async (window) => {
  window.stopped = true
  await stopGroup(window.electron)
  await stopGroup(window.devServer)
  let left = processesUnderRoot()
  for (const pid of left) { try { process.kill(pid, "SIGTERM") } catch { /* exited */ } }
  for (let attempt = 0; attempt < 20 && left.some(isAlive); attempt += 1) await wait(250)
  left = left.filter(isAlive)
  for (const pid of left) { try { process.kill(pid, "SIGKILL") } catch { /* exited */ } }
  return left
}

/**
 * The dev shell without `electron-dev.mjs`, because the Electron command line
 * needs the Ozone headless switches and electron-dev spawns Electron with a
 * fixed argument list. The HMR dev server runs as electron-dev starts it, and
 * Electron runs as `electron-dev.mjs` starts it, on the same environment and
 * with the same relaunch-on-75 rule.
 *
 * The app path is a generated main module: under the headless platform GTK has
 * no display, and building the tray's session menu (a GtkMenu) segfaults the
 * main process. The wrapper drops that native menu, which sits outside every
 * captured region, and imports the real entry module.
 */
const HEADLESS_MAIN = `import { createRequire } from "node:module"

const require = createRequire(import.meta.url)
const { Tray, nativeTheme } = require("electron")
// GTK cannot build the tray's session menu without a display: creating it
// segfaults the main process under the Ozone headless platform. The tray is
// native chrome outside every captured region, so the capture keeps the icon
// and drops its menu.
Tray.prototype.setContextMenu = () => undefined

// The Ozone platform reports no OS colour preference, so the app's settings
// resolve to 'system' and the renderer would pick the light theme. Force dark
// before the window exists, then keep it: the app assigns 'system' while it
// builds the early window, and that assignment must not undo this.
nativeTheme.themeSource = "dark"
try {
  Object.defineProperty(nativeTheme, "themeSource", { configurable: true, enumerable: true, get: () => "dark", set: () => {} })
} catch {
  nativeTheme.themeSource = "dark"
}

await import(process.env.OPENCHAMBER_CAPTURE_ENTRY)
`

const writeHeadlessMain = () => {
  const path = join(ROOT, "headless-main.mjs")
  writeFileSync(path, HEADLESS_MAIN)
  return path
}

const startWindow = (env) => {
  const output = createWriteStream(join(ROOT, "electron-dev.log"))
  const devServer = spawn(process.execPath, ["./scripts/dev-web-hmr.mjs"], {
    cwd: REPO_ROOT,
    env,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  })
  devServer.stdout.pipe(output)
  devServer.stderr.pipe(output)
  const headlessMain = writeHeadlessMain()
  const electronEnv = { ...env, OPENCHAMBER_CAPTURE_ENTRY: pathToFileURL(join(ELECTRON_DIR, "entry.mjs")).href }
  const window = { devServer, electron: null, stopped: false }
  const launchElectron = () => {
    const electron = spawn(ELECTRON_BIN, [headlessMain, ...HEADLESS_FLAGS], {
      cwd: ELECTRON_DIR,
      env: electronEnv,
      detached: true,
      stdio: ["ignore", "pipe", "pipe"],
    })
    electron.stdout.pipe(output)
    electron.stderr.pipe(output)
    window.electron = electron
    electron.once("exit", (code, signal) => {
      if (window.stopped) return
      if (code === RELAUNCH_EXIT_CODE) {
        log("the app relaunched itself (exit 75); starting it again")
        launchElectron()
        return
      }
      log(`electron exited with ${code ?? `signal ${signal}`}`)
    })
  }
  launchElectron()
  return window
}

/** Fails before a long wait when the Electron package's binary is missing or incomplete. */
const ensureElectron = () => {
  const result = spawnSync(process.execPath, ["packages/electron/scripts/ensure-electron.mjs"], {
    cwd: REPO_ROOT,
    env: isolatedEnv(),
    stdio: "inherit",
  })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`The Electron binary is missing or incomplete (ensure-electron exited with ${result.status})`)
}

const MOUNTED = `Boolean(document.querySelector("[data-left-sidebar]") && document.querySelector("[role=textbox][contenteditable=true]") && document.querySelector("[data-context-surface]"))`

/**
 * The page of the headless window once the app has mounted and stayed mounted
 * on the same target for STABLE_MS. A fresh Electron profile can relaunch the
 * app once shortly after its first start (the exit-75 relaunch), which
 * replaces the page target.
 */
const connectPage = async (uiPort, window) => {
  const STABLE_MS = 15_000
  const deadline = Date.now() + 240_000
  while (Date.now() < deadline) {
    if (window.devServer.exitCode !== null) throw new Error(`The HMR dev server exited with ${window.devServer.exitCode}; see ${join(ROOT, "electron-dev.log")}`)
    const electronExit = window.electron?.exitCode
    if (electronExit !== null && electronExit !== undefined && electronExit !== RELAUNCH_EXIT_CODE) {
      throw new Error(`Electron exited with ${electronExit}; see ${join(ROOT, "electron-dev.log")}`)
    }
    const target = await findPageTarget(CDP_PORT).catch(() => null)
    if (target?.url.startsWith(`http://127.0.0.1:${uiPort}`)) {
      const client = new CdpClient(target.webSocketDebuggerUrl)
      try {
        await client.connect()
        if (await evaluateValue(client, MOUNTED, 5000)) {
          await wait(STABLE_MS)
          const again = await findPageTarget(CDP_PORT).catch(() => null)
          if (again?.id === target.id && await evaluateValue(client, MOUNTED, 5000)) return { client, origin: new URL(target.url).origin }
        }
      } catch {
        // The page reloaded or the window relaunched; look again.
      }
      client.close()
    }
    await wait(1000)
  }
  throw new Error(`The app did not mount and stay mounted within 240s; see ${join(ROOT, "electron-dev.log")}`)
}

// ---------------------------------------------------------------------------
// Page helpers
// ---------------------------------------------------------------------------

/**
 * The window sits on the user's desktop, and a real click or hover changes
 * what a capture holds (a collapsed Work status section, revealed row
 * actions). The page ignores input for the whole run; this lets the script's
 * own events through for the moment they take.
 */
const sendInput = async (client, events) => {
  await client.send("Input.setIgnoreInputEvents", { ignore: false })
  try {
    for (const [method, params] of events) await client.send(method, params)
  } finally {
    await client.send("Input.setIgnoreInputEvents", { ignore: true })
  }
}

const click = (client, { x, y }) => sendInput(client, [
  ["Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none" }],
  ["Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", clickCount: 1 }],
  ["Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", clickCount: 1 }],
])

const pressEscape = (client) => sendInput(client, ["keyDown", "keyUp"].map((type) => [
  "Input.dispatchKeyEvent",
  { type, key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 },
]))

const restPointer = (client) => sendInput(client, [["Input.dispatchMouseEvent", { type: "mouseMoved", ...POINTER_REST, button: "none" }]])

const centerOf = (client, expression) => evaluateValue(client, `(() => {
  const element = ${expression}
  if (!element) return null
  const rect = element.getBoundingClientRect()
  return rect.width > 0 && rect.height > 0 ? { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 } : null
})()`)

/** The one rendered element matching `selector`; background sessions keep hidden copies of the chat column and composer. */
const renderedExpression = (selector) => `[...document.querySelectorAll(${JSON.stringify(selector)})]
  .find((element) => element.checkVisibility({ visibilityProperty: true }) && element.getBoundingClientRect().width > 0)`

const sidebarRowExpression = (title) => `[...document.querySelectorAll("[data-left-sidebar] *")]
  .find((element) => element.children.length === 0 && element.textContent.trim() === ${JSON.stringify(title)})`

const widthOf = (client, selector) => evaluateValue(client, `document.querySelector(${JSON.stringify(selector)})?.getBoundingClientRect().width ?? null`)

/**
 * Sets the sidebar to SIDEBAR_WIDTH by dragging its resize handle, the way a
 * user does, and checks the Work status card has its column and the context
 * panel is closed. A fresh profile starts at these values; the drag and the
 * toggle only run when the profile held others.
 */
const resetWidths = async (client) => {
  const sidebar = await widthOf(client, "[data-left-sidebar]")
  if (sidebar !== SIDEBAR_WIDTH) {
    const handle = await centerOf(client, `document.querySelector("[data-left-sidebar] > [role=separator]")`)
    if (!handle) throw new Error("The sidebar resize handle is not on screen")
    const to = { x: handle.x + (SIDEBAR_WIDTH - sidebar), y: handle.y }
    await sendInput(client, [
      ["Input.dispatchMouseEvent", { type: "mouseMoved", ...handle, button: "none" }],
      ["Input.dispatchMouseEvent", { type: "mousePressed", ...handle, button: "left", clickCount: 1 }],
      ["Input.dispatchMouseEvent", { type: "mouseMoved", ...to, button: "left", buttons: 1 }],
      ["Input.dispatchMouseEvent", { type: "mouseReleased", ...to, button: "left", clickCount: 1 }],
    ])
  }
  const contextPanelOpen = await evaluateValue(client, `document.querySelector("[data-right-slot]")?.getAttribute("data-context-panel-open")`)
  if (contextPanelOpen !== "false") throw new Error(`The context panel is open (data-context-panel-open=${contextPanelOpen}); close it in this profile first`)
  if ((await widthOf(client, "[data-right-slot]")) < WORK_STATUS_CARD_WIDTH) {
    const toggle = await centerOf(client, `document.querySelector("[data-work-status-toggle]")`)
    if (!toggle) throw new Error("The Work status toggle is not on screen")
    await click(client, toggle)
  }
  await restPointer(client)
  return checkWidths(client)
}

const checkWidths = async (client) => {
  const measured = await waitFor("the sidebar and Work status widths", async () => {
    const sidebar = await widthOf(client, "[data-left-sidebar]")
    const column = await widthOf(client, "[data-right-slot]")
    const card = await widthOf(client, "[data-right-slot] aside")
    return sidebar === SIDEBAR_WIDTH && column === WORK_STATUS_COLUMN_WIDTH && card === WORK_STATUS_CARD_WIDTH ? { sidebar, column, card } : null
  }, { timeoutMs: 10_000 })
  return {
    sidebar: { set: SIDEBAR_WIDTH, measured: measured.sidebar },
    workStatusPanel: { default: WORK_STATUS_COLUMN_WIDTH, measured: measured.column, card: measured.card },
  }
}

// ---------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------

const api = async (origin, path, init = {}, directory = null) => {
  const headers = { "content-type": "application/json", ...init.headers }
  if (directory) headers["x-opencode-directory"] = encodeURIComponent(directory)
  const response = await fetch(`${origin}${path}`, { ...init, headers })
  const body = await response.text()
  if (!response.ok) throw new Error(`${init.method ?? "GET"} ${path} answered ${response.status}: ${body.slice(0, 300)}`)
  return body ? JSON.parse(body) : null
}

const retryApi = async (origin, path, init = {}, directory = null, label = path) => {
  const deadline = Date.now() + 120_000
  let lastError
  while (Date.now() < deadline) {
    try {
      return await api(origin, path, init, directory)
    } catch (error) {
      lastError = error
      await wait(1000)
    }
  }
  throw new Error(`${label} did not answer within 120s: ${lastError?.message ?? lastError}`)
}

const projectId = (path) => `path_${Buffer.from(path).toString("base64url")}`

/** The settings React writes for the seed: two projects, favourites, recents and the picker order. */
const seedSettings = (paths, seed) => ({
  homeDirectory: paths.home,
  projects: [
    {
      id: projectId(paths.project),
      path: paths.project,
      label: basename(paths.project),
      addedAt: 1,
      lastOpenedAt: 1,
      sidebarCollapsed: false,
    },
    {
      id: projectId(paths.project2),
      path: paths.project2,
      label: basename(paths.project2),
      addedAt: 2,
      lastOpenedAt: 2,
      sidebarCollapsed: seed.secondProject.sidebarCollapsed,
      color: seed.secondProject.color,
      icon: seed.secondProject.icon,
    },
  ],
  activeProjectId: projectId(paths.project),
  lastDirectory: paths.project,
  defaultModel: seed.model,
  lastSelectedModel: seed.model,
  lastUsedEfforts: { [seed.model]: seed.variant },
  favoriteModels: seed.favoriteModels,
  recentModels: seed.recentModels,
  // The stable streaming screen's model keeps the same display name as the
  // agent model it stands in for, and stays out of the picker.
  hiddenModels: [{ providerID: "perf", modelID: "agent-20tools-hold" }],
  favoriteAgents: seed.favoriteAgents,
  recentAgents: seed.recentAgents,
  providerOrder: ["perf", "perf-alt"],
  collapsedModelProviders: ["perf-alt"],
  // The OpenCode update check runs a few seconds after mount and its toast
  // sits outside every region; a disabled check keeps the reference to the
  // window's own controls.
  showOpenCodeUpdateNotifications: false,
})

const createSession = async (origin, { title, directory, model, parentID }) => {
  const body = { title, location: { directory } }
  if (model) body.model = model
  if (parentID) body.parentID = parentID
  const created = await api(origin, "/api/session", {
    method: "POST",
    body: JSON.stringify(body),
  })
  const landed = created.data?.location?.directory
  if (landed !== directory) throw new Error(`Session "${title}" landed in ${landed}, not ${directory}`)
  return created.data.id
}

/** Creates a Chat session's folder under `chatsRoot` and its session, as React does. */
const createChatSession = async (origin, { title, directory }) => {
  await api(origin, "/api/fs/mkdir", { method: "POST", body: JSON.stringify({ path: directory }) })
  return createSession(origin, { title, directory })
}

/**
 * Seeds the two projects through a settings PUT (React writes the whole
 * document the same way), then the sessions through the API: project
 * sessions, the In work session's metadata through the server's route, the
 * second project's session and two Chat sessions in their own folders.
 */
const seedFixtures = async (origin, paths, seed) => {
  const settingsBody = seedSettings(paths, seed)
  await api(origin, "/api/config/settings?surface=desktop", { method: "PUT", body: JSON.stringify(settingsBody) })
  const home = await api(origin, "/api/fs/home")
  const settings = await api(origin, "/api/config/settings?surface=desktop")

  const ids = {}
  for (const session of seed.sessions) {
    ids[session.state] = await createSession(origin, { title: session.title, directory: paths.project })
  }
  ids.work = await createSession(origin, { title: seed.workSession.title, directory: paths.project })
  await api(origin, `/api/openchamber/sessions/${ids.work}/metadata`, {
    method: "POST",
    body: JSON.stringify({ patch: { openchamber: { work: { state: "open", openedAt: 1, openedBy: "user" } } } }),
  }, paths.project)
  ids.second = await createSession(origin, { title: seed.secondProjectSession.title, directory: paths.project2 })

  const chatDirectories = seed.chats.map((chat) => join(home.chatsRoot, seed.chatDate, `session-${chat.folder}`))
  for (const [index, chat] of seed.chats.entries()) {
    ids[`chat-${index}`] = await createChatSession(origin, { title: chat.title, directory: chatDirectories[index] })
  }
  return { home, settings, ids, chatDirectories }
}

/**
 * Every directory the state fixtures describe: the two projects and each Chat
 * session's folder. `catalogs` is read once because config, providers, models
 * and agents do not change during the run; the sessions are read again at each
 * state because a turn changes their times, status and metadata. The list
 * endpoints' `data` payloads are stored, the same arrays the app holds after
 * decoding; the HTTP envelopes live in `fixtures/api/`.
 */
/** The session activity the sidebar sorts by: the first turn stamp it has. */
const sessionActivity = (session) => {
  const time = session.time ?? {}
  return [time.idle ?? 0, time.updated ?? 0, time.created ?? 0].find((stamp) => stamp > 0) ?? 0
}

/**
 * The app's session order (pinned first, then newest activity, newer creation,
 * id), written oldest first. The stabilizer turns document order into
 * increasing times, and a replay sorts newest first, so the last row here is
 * the sidebar's first: the document-order ladder lands on the app's order
 * without trusting either absolute timestamps or equal-time ties.
 */
const reverseSidebarOrder = (sessions) => [...sessions].sort((left, right) => {
  const leftPinned = Boolean(left.metadata?.pinned)
  const rightPinned = Boolean(right.metadata?.pinned)
  if (leftPinned !== rightPinned) return leftPinned ? -1 : 1
  const leftActivity = sessionActivity(left)
  const rightActivity = sessionActivity(right)
  if (leftActivity !== rightActivity) return leftActivity - rightActivity
  const leftCreated = left.time?.created ?? 0
  const rightCreated = right.time?.created ?? 0
  if (leftCreated !== rightCreated) return leftCreated - rightCreated
  return String(left.id).localeCompare(String(right.id))
})

const readDirectoryState = async (origin, directory, { catalogs = true } = {}) => {
  const [config, providers, models, agents, sessions] = await Promise.all([
    catalogs ? retryApi(origin, "/api/config", {}, directory, `config for ${directory}`) : null,
    catalogs ? retryApi(origin, "/api/provider", {}, directory, `providers for ${directory}`) : null,
    catalogs ? retryApi(origin, "/api/model?types=chat&types=image", {}, directory, `models for ${directory}`) : null,
    catalogs ? retryApi(origin, "/api/agent", {}, directory, `agents for ${directory}`) : null,
    retryApi(origin, `/api/session?limit=500&directory=${encodeURIComponent(directory)}`, {}, directory, `sessions for ${directory}`),
  ])
  const payload = (response) => Array.isArray(response) ? response : response.data ?? response
  const entry = {}
  if (catalogs) {
    entry.config = config
    entry.providers = payload(providers)
    entry.models = payload(models)
    entry.agents = payload(agents)
  }
  entry.sessions = reverseSidebarOrder(Array.isArray(sessions) ? sessions : sessions.data ?? [])
  return entry
}

const readDirectories = async (origin, directories) => {
  const entries = {}
  for (const directory of directories) {
    entries[directory] = await readDirectoryState(origin, directory)
  }
  return entries
}

/** Re-reads only the session lists, keeping the catalogs captured once. */
const refreshSessions = async (origin, directories) => {
  for (const [directory, entry] of Object.entries(directories)) {
    const current = await readDirectoryState(origin, directory, { catalogs: false })
    entry.sessions = current.sessions
  }
}

const sessionTime = async (origin, id, project) => (await api(origin, `/api/session/${id}`, {}, project)).data?.time ?? {}

const isBusy = (time) => Boolean(time.busy) && !(time.idle >= time.busy)

/**
 * Opens a session from its sidebar row and sends the seed's prompt from the
 * composer with real input, then waits until the server reports the turn
 * running, so a keystroke the editor dropped fails here instead of yielding
 * a capture of an idle session.
 */
const sendFromComposer = async (client, { origin, project, id, title, prompt }) => {
  const rowExpression = `(() => { const element = ${sidebarRowExpression(title)}; element?.scrollIntoView({ block: "center", behavior: "instant" }); return element })()`
  const row = await waitFor(`the sidebar row "${title}"`, () => centerOf(client, rowExpression))
  await wait(300)
  await click(client, row)
  await waitFor(`"${title}" in the header`, () => evaluateValue(client, `document.querySelector("[data-page-scroll-lock] > header")?.textContent.includes(${JSON.stringify(title)})`))
  // The session view finishes switching before the editor takes input.
  await wait(1500)
  const editorExpression = renderedExpression("[role=textbox][contenteditable=true]")
  const editor = await waitFor("the composer", () => centerOf(client, editorExpression))
  await click(client, editor)
  await sendInput(client, [["Input.insertText", { text: prompt }]])
  const promptInComposer = () => evaluateValue(client, `${editorExpression}?.textContent.includes(${JSON.stringify(prompt)})`)
  await waitFor("the prompt in the composer", promptInComposer, { timeoutMs: 5000 })
  const pressEnter = () => sendInput(client, ["keyDown", "keyUp"].map((type) => ["Input.dispatchKeyEvent", { type, key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 }]))
  const waitForStart = () => waitFor(`the turn in "${title}" to start`, async () => isBusy(await sessionTime(origin, id, project)), { timeoutMs: 15_000, intervalMs: 200 })
  await pressEnter()
  // A click on a row that moved during the finished turn's In work relocation
  // can leave the editor unfocused; one refocused retry proves the prompt is
  // still the one in the composer rather than silently capturing an idle one.
  const started = await waitForStart().then(() => true, () => false)
  if (started) return
  if (!await promptInComposer()) throw new Error(`The prompt left the composer but "${title}" did not start; session ${JSON.stringify((await api(origin, `/api/session/${id}`, {}, project)).data?.time)}`)
  await click(client, editor)
  await pressEnter()
  await waitForStart()
}

/** Scrolls the chat column to its end with the mouse wheel, as a reader does. */
const scrollChatToEnd = async (client) => {
  const center = await centerOf(client, renderedExpression("[data-workspace-chat]"))
  for (let turn = 0; turn < 2; turn += 1) {
    await sendInput(client, [["Input.dispatchMouseEvent", { type: "mouseWheel", ...center, deltaX: 0, deltaY: 100_000, button: "none" }]])
    await wait(1500)
  }
  // A wheel can stop a pixel short of the end, which leaves the scroll-to-bottom
  // button visible in one run and hidden in the next. Land it exactly.
  await evaluateValue(client, `(() => {
    const scroller = ${renderedExpression("[data-workspace-chat]")}?.querySelector(".chat-scroll")
    if (!scroller) return false
    scroller.scrollTop = scroller.scrollHeight
    return true
  })()`)
  await wait(500)
}

/** Reloads and waits for the new document's load event, not the old page's evaluate. */
const reloadPage = async (client) => {
  await client.send("Page.enable")
  const loaded = client.once("Page.loadEventFired", 120_000)
  await client.send("Page.reload", { ignoreCache: true })
  await loaded
}

/** Reloads the page after seeding so it reads the settings and sessions the run wrote. */
const reloadSeededApp = async (client) => {
  await reloadPage(client)
  await waitFor("the reloaded app", () => evaluateValue(client, MOUNTED), { timeoutMs: 120_000, intervalMs: 500 })
  await client.send("Emulation.setDeviceMetricsOverride", { ...VIEWPORT, deviceScaleFactor: 1, mobile: false })
  await client.send("Emulation.setFocusEmulationEnabled", { enabled: true })
  await client.send("Accessibility.enable")
  await client.send("Input.setIgnoreInputEvents", { ignore: true })
  await waitFor("the seeded sidebar", () => evaluateValue(client, `[...document.querySelectorAll("[data-left-sidebar] *")]
    .some((element) => element.children.length === 0 && element.textContent.trim() === "Reference: finished turn")`), { timeoutMs: 120_000, intervalMs: 500 })
  // The catalog and agents are fetched per directory after the reload; the
  // model label and the Thinking trigger appear only once they arrived.
  await waitFor("the model catalog", () => evaluateValue(client, `(() => {
    const label = document.querySelector(".model-controls__model-label")?.textContent?.trim() ?? ""
    const agent = document.querySelector(".model-controls__agent-label")?.textContent?.trim() ?? ""
    return Boolean(label) && !label.includes("Select model") && !label.includes("Unavailable")
      && Boolean(agent) && !agent.includes("Select agent")
  })()`), { timeoutMs: 120_000, intervalMs: 500 })
  await waitFor("the Thinking trigger", () => evaluateValue(client, `Boolean(document.querySelector(".model-controls__variant-trigger"))`), { timeoutMs: 120_000, intervalMs: 500 })
  // The seeded second project starts collapsed. Its session row is rendered
  // for a moment while the settings apply; a capture on either side of that
  // moment records a different sidebar.
  await waitFor("the collapsed second project", () => evaluateValue(client, `![...document.querySelectorAll("[data-left-sidebar] *")]
    .some((element) => element.children.length === 0 && element.textContent.trim() === "Reference: second project")`), { timeoutMs: 120_000, intervalMs: 500 })
  await wait(1000)
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

const CONTROL_SELECTOR = [
  "button", "a[href]", "input:not([type=hidden])", "textarea", "select", "summary",
  "[contenteditable=true]", "[contenteditable='']",
  "[role=button]", "[role=link]", "[role=tab]", "[role=menuitem]", "[role=checkbox]", "[role=switch]",
  "[role=combobox]", "[role=textbox]", "[role=searchbox]", "[role=option]", "[role=radio]",
  "[role=slider]", "[role=spinbutton]", "[role=treeitem]",
  // Resize handles: not focusable, but a control the pointer uses.
  "[role=separator][aria-orientation]",
  "[tabindex]:not([tabindex='-1'])",
].join(", ")

/**
 * The regions, in assignment order: a control belongs to the first region
 * whose root contains it. `also` adds a second root to a region, `parent`
 * takes the matched element's parent as the root, and `popup` marks the one
 * optional region: an open Base UI popup (role dialog or menu) that no other
 * region contains. Written to every JSON under `selectors`; these are the
 * only selectors in the output.
 *
 * `dock` is optional too and keeps every match as a root: the floating panel
 * above the composer (the permission, form or consent dock) and the queue
 * strip's top-row group inside the composer box. The strip is a child of the
 * composer box, so the dock must come before `composer` for its rows to be
 * compared in the dock. Its bounds are the union of its roots.
 */
const REGIONS = [
  {
    name: "dock",
    selector: "form[data-prompt-history-search-owner] [role=dialog], [data-dictation-chain]:not([data-dictation-chain] [data-dictation-chain]) > div > div[role=group]",
    optional: true,
    union: true,
  },
  { name: "composer", selector: "[data-dictation-chain]:not([data-dictation-chain] [data-dictation-chain])", parent: true },
  { name: "workStatusPanel", selector: "[data-right-slot]" },
  { name: "iconRail", selector: "nav:has([data-context-surface])" },
  { name: "header", selector: "[data-page-scroll-lock] > header" },
  { name: "sidebar", selector: "[data-left-sidebar]", also: "[data-titlebar-left-controls]" },
  { name: "chatColumn", selector: "[data-workspace-chat]" },
  // The open popup only: a portaled dialog or menu outside every region
  // above. It is optional, so a closed popover contributes no bounds.
  { name: "popover", selector: "[role=dialog], [role=menu]", popup: true },
]
// The order regions appear in each JSON.
const REGION_ORDER = ["sidebar", "header", "chatColumn", "composer", "dock", "workStatusPanel", "iconRail", "popover"]

/**
 * Runs in the page in one task, so the regions and the control list come from
 * the same DOM. Leaves the control elements and the region roots on window
 * for the accessibility lookup.
 */
const collectInPage = `(() => {
  const CONTROL_SELECTOR = ${JSON.stringify(CONTROL_SELECTOR)}
  const REGIONS = ${JSON.stringify(REGIONS)}
  // Sessions kept alive in the background keep their own (hidden) chat
  // column and composer, so a region is the rendered element that matches.
  const rendered = (element) => element.checkVisibility({ visibilityProperty: true }) && element.getBoundingClientRect().width > 0
  const roots = {}
  for (const region of REGIONS) {
    if (region.popup) continue
    const matches = [...document.querySelectorAll(region.selector)].filter(rendered)
    if (region.optional) {
      // Zero matches contribute no bounds at all; every match is a root.
      if (matches.length === 0) continue
      roots[region.name] = matches.map((element) => region.parent ? element.parentElement : element).filter(Boolean)
      continue
    }
    if (matches.length !== 1) throw new Error(region.selector + " matched " + matches.length + " rendered elements")
    roots[region.name] = [region.parent ? matches[0].parentElement : matches[0], region.also ? document.querySelector(region.also) : null].filter(Boolean)
  }
  // A popup is the one rendered dialog or menu that no other region contains.
  // Closed menus unmount, and a portaled popup sits under body, so this finds
  // the open popup without a selector for its portal.
  const regionRoots = Object.values(roots).flat()
  const popups = [...document.querySelectorAll("[role=dialog], [role=menu]")]
    .filter(rendered)
    .filter((element) => !regionRoots.some((root) => root.contains(element)))
  if (popups.length > 1) throw new Error("More than one open popup: " + popups.map((element) => element.getAttribute("role") + " " + (element.getAttribute("aria-label") ?? "")).join(", "))
  roots.popover = popups
  const round = (value) => Math.round(value * 100) / 100
  const bounds = (element) => {
    const rect = element.getBoundingClientRect()
    return { x: round(rect.x), y: round(rect.y), width: round(rect.width), height: round(rect.height) }
  }
  const unionBounds = (elements) => {
    const rects = elements.map((element) => element.getBoundingClientRect())
    const x = Math.min(...rects.map((rect) => rect.x))
    const y = Math.min(...rects.map((rect) => rect.y))
    const right = Math.max(...rects.map((rect) => rect.right))
    const bottom = Math.max(...rects.map((rect) => rect.bottom))
    return { x: round(x), y: round(y), width: round(right - x), height: round(bottom - y) }
  }

  // Colors are painted and read back so oklch() and color() reach sRGB, then
  // the element's own opacity and its ancestors' are multiplied into alpha.
  const canvas = document.createElement("canvas")
  canvas.width = 1
  canvas.height = 1
  const canvasContext = canvas.getContext("2d", { willReadFrequently: true })
  const srgbCache = new Map()
  const toSrgb = (value) => {
    if (srgbCache.has(value)) return srgbCache.get(value)
    canvasContext.clearRect(0, 0, 1, 1)
    canvasContext.fillStyle = value
    canvasContext.fillRect(0, 0, 1, 1)
    const [r, g, b, a] = canvasContext.getImageData(0, 0, 1, 1).data
    const color = { r, g, b, a }
    srgbCache.set(value, color)
    return color
  }
  const opacityOf = (element) => {
    let opacity = 1
    for (let node = element; node; node = node.parentElement) {
      const value = Number.parseFloat(getComputedStyle(node).opacity)
      if (Number.isFinite(value)) opacity *= value
    }
    return opacity
  }
  const hex = (r, g, b, a) => "#" + [r, g, b, a].map((channel) => channel.toString(16).padStart(2, "0")).join("")
  const colorOf = (element, value) => {
    const color = toSrgb(value)
    return hex(color.r, color.g, color.b, Math.round(color.a * opacityOf(element)))
  }
  const drawn = (element) => element.checkVisibility({ visibilityProperty: true }) && element.getBoundingClientRect().width > 0
  const imgName = (element) => {
    const raw = element.currentSrc || element.src || element.getAttribute("src") || ""
    const path = raw.split(/[?#]/)[0]
    const file = path.slice(path.lastIndexOf("/") + 1)
    const id = file.includes(".") ? file.slice(0, file.lastIndexOf(".")) : file
    return "img:" + (id || "other")
  }
  const iconsOf = (control) => [...control.querySelectorAll("svg, img")]
    .filter((element) => drawn(element) && !insideNestedControl(element, control))
    .flatMap((element) => {
      if (element.tagName.toLowerCase() === "img") {
        return [{ name: imgName(element), bounds: bounds(element) }]
      }
      const href = [...element.querySelectorAll("use")]
        .map((use) => use.getAttribute("href") ?? "")
        .find((value) => value.startsWith("#oc-")) ?? ""
      const color = colorOf(element, getComputedStyle(element).color)
      // A hover- or focus-revealed icon is not drawn at rest.
      if (color.endsWith("00")) return []
      return [{ name: href.startsWith("#oc-") ? href.slice(4) : "other", color, bounds: bounds(element) }]
    })
  const marksOf = (control) => [...control.querySelectorAll("*")].flatMap((element) => {
    if (insideNestedControl(element, control)) return []
    if (element.tagName.toLowerCase() === "svg" || element.querySelector("svg")) return []
    if ((element.textContent ?? "").trim().length > 0) return []
    if (!drawn(element)) return []
    const rect = element.getBoundingClientRect()
    if (Math.max(rect.width, rect.height) > 12) return []
    const background = getComputedStyle(element).backgroundColor
    const { a } = toSrgb(background)
    if (a === 0) return []
    const color = colorOf(element, background)
    if (color.endsWith("00")) return []
    return [{ color, bounds: bounds(element) }]
  })
  const textRunsOf = (control) => {
    const runs = []
    let current = null
    let pendingSpace = false
    const rectBounds = (rect) => ({ x: round(rect.x), y: round(rect.y), width: round(rect.width), height: round(rect.height) })
    const extend = (bounds, rect) => {
      const x = Math.min(bounds.x, rect.x)
      const y = Math.min(bounds.y, rect.y)
      return {
        x: round(x),
        y: round(y),
        width: round(Math.max(bounds.x + bounds.width, rect.right) - x),
        height: round(Math.max(bounds.y + bounds.height, rect.bottom) - y),
      }
    }
    const finalize = () => {
      if (!current) return
      const text = current.text.replace(/\\s+/g, " ").trim()
      if (text) runs.push({ text, color: current.color, bounds: current.bounds })
      current = null
    }
    const walker = document.createTreeWalker(control, NodeFilter.SHOW_TEXT)
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const raw = node.nodeValue ?? ""
      if (!raw.trim()) { if (current) pendingSpace = true; continue }
      const parent = node.parentElement
      if (!parent || !parent.checkVisibility({ visibilityProperty: true })) continue
      if (insideNestedControl(parent, control)) continue
      const range = document.createRange()
      range.selectNodeContents(node)
      const rect = range.getBoundingClientRect()
      if (rect.width === 0 && rect.height === 0) continue
      const color = colorOf(parent, getComputedStyle(parent).color)
      // A hover- or focus-revealed run is not drawn at rest.
      if (color.endsWith("00")) { if (current) pendingSpace = true; continue }
      const sameLine = Boolean(current) && Math.min(current.line.bottom, rect.bottom) - Math.max(current.line.top, rect.top) > 0
      if (!current || current.color !== color || !sameLine) {
        finalize()
        current = { color, text: "", line: rect, bounds: rectBounds(rect) }
        pendingSpace = false
      }
      if (current.text && pendingSpace && !/^\\s/.test(raw) && !/\\s$/.test(current.text)) current.text += " "
      pendingSpace = false
      current.text += raw
      current.line = rect
      current.bounds = extend(current.bounds, rect)
    }
    finalize()
    return runs
  }
  const elements = []
  for (const element of document.querySelectorAll(CONTROL_SELECTOR)) {
    if (element.closest("[inert]") || element.closest("[aria-hidden=true]")) continue
    if (!element.checkVisibility({ visibilityProperty: true })) continue
    const box = element.getBoundingClientRect()
    if (box.width === 0 || box.height === 0) continue
    elements.push(element)
  }
  // A control's draw fields cover its own content: a nested control records its
  // own text and icons. Without this a container repeats every child control
  // and its fields follow the virtualizer's mounted window.
  const controlSet = new Set(elements)
  const insideNestedControl = (element, control) => {
    for (let node = element.parentElement; node && node !== control; node = node.parentElement) {
      if (controlSet.has(node)) return true
    }
    return false
  }
  const visibleText = (node) => (node.innerText ?? "").replace(/\\s+/g, " ").trim()
  // A row that pairs a label with a value: descend while one child element
  // holds all of the text; where the text first splits across two or more
  // child elements (and no loose text sits beside them), the first is the
  // label and the rest is the value.
  const labelOf = (element) => {
    let node = element
    for (;;) {
      const loose = [...node.childNodes].some((child) => child.nodeType === Node.TEXT_NODE && child.textContent.trim())
      const parts = [...node.children].filter((child) => visibleText(child))
      if (loose || parts.length === 0) return null
      if (parts.length === 1) { node = parts[0]; continue }
      return visibleText(parts[0])
    }
  }
  const controls = elements.map((element) => ({
    region: REGIONS.find((region) => (roots[region.name] ?? []).some((root) => root.contains(element)))?.name ?? null,
    bounds: bounds(element),
    shown: element.checkVisibility({ visibilityProperty: true, opacityProperty: true }),
    text: visibleText(element),
    label: labelOf(element),
    icons: iconsOf(element),
    marks: marksOf(element),
    texts: textRunsOf(element),
    // Controls inside this one, in DOM order.
    nested: elements.flatMap((other, index) => other !== element && element.contains(other) ? [index] : []),
  }))
  window.__ocReferenceControls = elements
  window.__ocReferenceRoots = roots
  return {
    regions: Object.fromEntries(Object.entries(roots).flatMap(([name, list]) => {
      if (list.length === 0) return []
      const region = REGIONS.find((entry) => entry.name === name)
      return [[name, region?.union ? unionBounds(list) : bounds(list[0])]]
    })),
    controls,
    chatCodeBlocks: [...roots.chatColumn[0].querySelectorAll("pre")].filter((element) => element.checkVisibility()).length,
  }
})()`

const collapse = (value) => String(value ?? "").replace(/\s+/g, " ").trim()

/** Each control's own accessibility node and its ancestors, nearest first, from Chromium's accessibility tree. */
const accessibilityOf = async (client, count) => {
  const { result } = await client.send("Runtime.evaluate", { expression: "window.__ocReferenceControls" })
  const { result: properties } = await client.send("Runtime.getProperties", { objectId: result.objectId, ownProperties: true })
  const byIndex = new Map(properties.filter((property) => /^\d+$/.test(property.name)).map((property) => [Number(property.name), property.value.objectId]))
  const out = []
  for (let index = 0; index < count; index += 1) {
    const { node } = await client.send("DOM.describeNode", { objectId: byIndex.get(index) })
    const { nodes } = await client.send("Accessibility.getAXNodeAndAncestors", { backendNodeId: node.backendNodeId })
    const self = nodes.findIndex((entry) => entry.backendDOMNodeId === node.backendNodeId)
    if (self === -1) throw new Error(`No accessibility node for control ${index}`)
    out.push({ backendNodeId: node.backendNodeId, ax: nodes[self], ancestors: nodes.slice(self + 1) })
  }
  return out
}

/** Whether a DOM node lies inside a region's root (the root itself excluded); cached per node. */
const regionMembership = (client) => {
  const cache = new Map()
  return async (region, backendNodeId) => {
    const cacheKey = `${region}:${backendNodeId}`
    if (!cache.has(cacheKey)) {
      const { object } = await client.send("DOM.resolveNode", { backendNodeId })
      const { result } = await client.send("Runtime.callFunctionOn", {
        objectId: object.objectId,
        functionDeclaration: "function (region) { return (window.__ocReferenceRoots[region] ?? []).some((root) => root !== this && root.contains(this)) }",
        arguments: [{ value: region }],
        returnByValue: true,
      })
      cache.set(cacheKey, result.value === true)
    }
    return cache.get(cacheKey)
  }
}

/**
 * Figures in accessible names that the same turn measures differently on
 * every run: elapsed times and the Turn stats rate. Each rule replaces only
 * the figure.
 */
const NAME_NORMALIZATION = [
  { rule: "rate", pattern: /~?\d+(?:\.\d+)?\s?tok\/s/g, replacement: "<rate> tok/s" },
  // A label and its value can share one text run ("Elapsed24.3s"), so the
  // figure may follow a letter; only a preceding digit rules a match out.
  { rule: "duration", pattern: /(?<!\d)\d+(?:\.\d+)?(?:ms|s|m|h)\b/g, replacement: "<duration>" },
]

/**
 * Rule 2 of the header: timing figures, then a name that starts with a seed
 * session's title cut to the title (a session row adds its status,
 * "Session active Active for 6s", to the title's own text).
 */
const normalizeName = (name, titles) => {
  const rules = []
  let out = name
  for (const { rule, pattern, replacement } of NAME_NORMALIZATION) {
    const next = out.replace(pattern, replacement)
    if (next !== out) rules.push(rule)
    out = next
  }
  const title = titles.find((candidate) => out.startsWith(`${candidate} `))
  if (title) {
    out = title
    rules.push("session-title")
  }
  return { name: out, rules }
}

const nameFromContents = (ax) => (ax.name?.sources ?? []).some((source) => source.type === "contents" && !source.superseded && source.value)
const roleDescription = (ax) => ax.properties?.find((property) => property.name === "roledescription")?.value?.value

/**
 * Names and keys every control; the rules are in the header comment.
 * Returns the controls with role, name, key and the bookkeeping for
 * `accessibility` and `normalized`.
 */
const nameAndKey = async (client, collected, axData, titles) => {
  const { controls: pageControls } = collected
  const indexOfNode = new Map(axData.map(({ backendNodeId }, index) => [backendNodeId, index]))
  const controls = pageControls.map((control, index) => {
    const { ax } = axData[index]
    const role = ax.ignored ? "ignored" : String(ax.role?.value ?? "")
    const aria = collapse(ax.name?.value)
    // Text only for a control holding no other control: an unnamed container's
    // text is its children's (the transcript, for the chat's scroller).
    const fromText = control.text && control.nested.length === 0
    const [nameSource, raw] = aria
      ? ["aria name", aria]
      : fromText && control.label ? ["text label", control.label]
      : fromText ? ["text", control.text]
      : ["role", role]
    const { name, rules } = normalizeName(raw, titles)
    return { ...control, role, nameSource, name, rules, ax }
  })

  // Rule 3: a control named from its contents that holds other controls
  // takes the name of the first control inside it, not its children's names.
  const finalName = (index, seen = new Set()) => {
    const control = controls[index]
    if (control.resolved) return control.name
    if (control.nameSource === "aria name" && nameFromContents(control.ax) && control.nested.length > 0 && !seen.has(index)) {
      seen.add(index)
      const inner = finalName(control.nested[0], seen)
      if (inner !== control.name) {
        control.name = inner
        control.rules = [...control.rules, "container-name"]
      }
    }
    control.resolved = true
    return control.name
  }
  controls.forEach((_, index) => finalName(index))

  const inside = regionMembership(client)
  for (const [index, control] of controls.entries()) {
    // A control holding another control of the same name carries its role
    // description (or role) as a suffix; the inner control keeps the bare name.
    const sameNameInside = control.nested.some((other) => controls[other].name === control.name)
    const own = sameNameInside ? `${control.name} (${roleDescription(control.ax) ?? control.role})` : control.name
    const segments = []
    if (control.region) {
      for (const ancestor of axData[index].ancestors) {
        if (ancestor.ignored || !ancestor.backendDOMNodeId) continue
        const controlIndex = indexOfNode.get(ancestor.backendDOMNodeId)
        // Only accessible names make segments, never a fallback text or role.
        if (controlIndex !== undefined && controls[controlIndex].nameSource !== "aria name") continue
        const segment = controlIndex === undefined ? normalizeName(collapse(ancestor.name?.value), titles).name : controls[controlIndex].name
        if (!segment) continue
        if (!(await inside(control.region, ancestor.backendDOMNodeId))) break
        segments.unshift(segment)
      }
    }
    segments.push(own)
    // An ancestor segment equal to the one after it is not repeated.
    const path = segments.filter((segment, position) => segment !== segments[position + 1])
    control.key = [control.region ?? "outside", ...path].join("/")
  }

  // True repeats under the same parent: #n in DOM order.
  const keyCount = Map.groupBy(controls, (control) => control.key)
  const seen = new Map()
  for (const control of controls) {
    if (keyCount.get(control.key).length === 1) continue
    const ordinal = (seen.get(control.key) ?? 0) + 1
    seen.set(control.key, ordinal)
    control.repeated = true
    control.key = `${control.key}#${ordinal}`
  }
  return controls
}

/**
 * The first rows of an 8-bit, non-interlaced RGB/RGBA PNG, as unfiltered
 * channel bytes. A reference must not carry the dev window's tape strip.
 */
const pngTopRows = (buffer, rowCount) => {
  if (buffer.length < 8 || buffer.readUInt32BE(0) !== 0x89504e47 || buffer.readUInt32BE(4) !== 0x0d0a1a0a) {
    throw new Error("The screenshot is not a PNG")
  }
  let offset = 8
  let width = 0
  let height = 0
  let bitDepth = 0
  let colorType = 0
  let interlace = 0
  const idat = []
  while (offset + 12 <= buffer.length) {
    const length = buffer.readUInt32BE(offset)
    const type = buffer.toString("ascii", offset + 4, offset + 8)
    const data = buffer.subarray(offset + 8, offset + 8 + length)
    if (type === "IHDR") {
      width = data.readUInt32BE(0)
      height = data.readUInt32BE(4)
      bitDepth = data[8]
      colorType = data[9]
      interlace = data[12]
    } else if (type === "IDAT") {
      idat.push(data)
    } else if (type === "IEND") {
      break
    }
    offset += 12 + length
  }
  if (bitDepth !== 8 || interlace !== 0 || (colorType !== 2 && colorType !== 6)) {
    throw new Error(`Unexpected screenshot PNG: depth ${bitDepth}, color type ${colorType}, interlace ${interlace}`)
  }
  const channels = colorType === 6 ? 4 : 3
  const stride = width * channels
  const raw = inflateSync(Buffer.concat(idat))
  const rows = []
  const prior = Buffer.alloc(stride)
  let position = 0
  for (let y = 0; y < Math.min(rowCount, height); y += 1) {
    const filter = raw[position]
    position += 1
    const line = raw.subarray(position, position + stride)
    position += stride
    const out = Buffer.alloc(stride)
    for (let x = 0; x < stride; x += 1) {
      const left = x >= channels ? out[x - channels] : 0
      const above = prior[x]
      const upperLeft = x >= channels ? prior[x - channels] : 0
      let value = line[x]
      if (filter === 1) value += left
      else if (filter === 2) value += above
      else if (filter === 3) value += Math.floor((left + above) / 2)
      else if (filter === 4) {
        const estimate = left + above - upperLeft
        const toLeft = Math.abs(estimate - left)
        const toAbove = Math.abs(estimate - above)
        const toUpperLeft = Math.abs(estimate - upperLeft)
        value += toLeft <= toAbove && toLeft <= toUpperLeft ? left : toAbove <= toUpperLeft ? above : upperLeft
      }
      out[x] = value & 0xff
    }
    rows.push(out)
    out.copy(prior)
  }
  return { channels, rows }
}

const TAPE_RGB = [0xd6, 0xa2, 0x3c]

/** The dev window tape draws a solid #d6a23c stripe along the first 5 rows. */
const pngHasTape = (buffer) => {
  const { channels, rows } = pngTopRows(buffer, 5)
  return rows.some((row) => {
    for (let x = 0; x + channels <= row.length; x += channels) {
      if (row[x] === TAPE_RGB[0] && row[x + 1] === TAPE_RGB[1] && row[x + 2] === TAPE_RGB[2]) return true
    }
    return false
  })
}

/**
 * Every capture must look like the installed dark app: no dev tape, the dark
 * colour scheme, and openchamber-dark's surface background. A failure here
 * aborts the run before any fixture is written.
 */
const assertReferenceWindow = async (client, state, screenshot) => {
  const seen = JSON.parse(await evaluateValue(client, `JSON.stringify({
    tape: getComputedStyle(document.documentElement, "::after").display,
    dark: matchMedia("(prefers-color-scheme: dark)").matches,
    background: getComputedStyle(document.documentElement).getPropertyValue("--background").trim(),
  })`))
  if (seen.tape !== "none") throw new Error(`${state}: the dev window tape is visible (html::after display ${seen.tape})`)
  if (seen.dark !== true) throw new Error(`${state}: the renderer does not prefer a dark colour scheme`)
  if (seen.background !== "#120f0e") throw new Error(`${state}: --background is ${seen.background}, not openchamber-dark #120f0e`)
  if (pngHasTape(screenshot)) throw new Error(`${state}: the screenshot's top rows carry the dev window tape (#d6a23c)`)
}

const captureState = async (client, state, widths, titles, prelude = "", collectedValue = null) => {
  await restPointer(client)
  const collected = collectedValue ?? await evaluateValue(client, `${prelude};\n${collectInPage}`)
  if (!collected) throw new Error(`Collecting ${state} failed in the page`)
  const shot = await client.send("Page.captureScreenshot", { format: "png" })
  await assertReferenceWindow(client, state, Buffer.from(shot.data, "base64"))
  const controls = await nameAndKey(client, collected, await accessibilityOf(client, collected.controls.length), titles)
  // The same normalization the accessible names use turns a live figure in a
  // text run (an elapsed time, a rate) into its placeholder, so two runs record
  // the same text. Colors and bounds are the measured ones.
  for (const control of controls) {
    control.texts = control.texts.map((run) => ({ ...run, text: normalizeName(run.text, titles).name }))
  }
  await client.send("Runtime.evaluate", { expression: "delete window.__ocReferenceControls; delete window.__ocReferenceRoots" })
  const entry = ({ role, name, key, shown, bounds, icons, marks, texts }) => ({ role, name, key, shown, bounds, icons, marks, texts })
  const regions = Object.fromEntries(REGION_ORDER.flatMap((name) => {
    // A closed popover has no root and no bounds; leave it out of the region
    // list rather than claiming a box at zero.
    if (!collected.regions[name]) return []
    return [[name, {
      bounds: collected.regions[name],
      controls: controls.filter((control) => control.region === name).map(entry),
    }]]
  }))
  const reference = {
    state,
    viewport: VIEWPORT,
    widths,
    selectors: Object.fromEntries(REGIONS.map(({ name, ...selector }) => [name, selector])),
    regions,
    outside: controls.filter((control) => control.region === null).map(entry),
    accessibility: controls.filter((control) => control.nameSource !== "aria name" || control.repeated).map(({ region, key, role, nameSource, repeated }) => ({
      region: region ?? "outside",
      key,
      role,
      problem: nameSource !== "aria name" ? "missing-name" : "repeated-key",
      nameSource,
    })),
    normalized: controls.filter((control) => control.rules.length > 0).map(({ region, key, rules }) => ({ region: region ?? "outside", key, rules })),
  }
  const counts = Object.entries(regions).map(([name, region]) => `${name} ${region.controls.length}`).join(", ")
  const bySource = (source) => controls.filter((control) => control.nameSource === source).length
  log(`${state}: ${counts}, outside ${reference.outside.length}; names from text ${bySource("text")}, from text label ${bySource("text label")}, from role ${bySource("role")}; repeated keys ${controls.filter((control) => control.repeated).length}`)
  return { state, json: `${JSON.stringify(reference, null, 2)}\n`, png: Buffer.from(shot.data, "base64"), chatCodeBlocks: collected.chatCodeBlocks }
}

// ---------------------------------------------------------------------------
// Popup states
// ---------------------------------------------------------------------------

const POPUP_TRIGGERS = {
  "project-popover": { selector: "button[aria-haspopup=dialog]", popup: "[role=dialog]" },
  "model-popover": { selector: ".model-controls__model-trigger", popup: "[role=menu]" },
  "agent-menu": { selector: ".model-controls__agent-label", popup: "[role=menu]" },
  "thinking-menu": { selector: ".model-controls__variant-trigger", popup: "[role=menu]" },
}

/** Opens a popup by clicking its trigger through CDP, captures it, then closes it with Escape. */
const capturePopup = async (client, name, widths, titles) => {
  const trigger = POPUP_TRIGGERS[name]
  const center = await waitFor(`the ${name} trigger`, () => centerOf(client, renderedExpression(trigger.selector)))
  await click(client, center)
  await waitFor(`the ${name} popup`, () => evaluateValue(client, `(() => {
    const roots = ${JSON.stringify(REGIONS.filter((region) => !region.popup).map((region) => region.selector))}
      .flatMap((selector) => [...document.querySelectorAll(selector)])
    return [...document.querySelectorAll(${JSON.stringify(trigger.popup)})]
      .some((element) => element.checkVisibility({ visibilityProperty: true })
        && element.getBoundingClientRect().width > 0
        && !roots.some((root) => root !== element && root.contains(element)))
  })()`), { timeoutMs: 10_000 })
  await wait(500)
  const capture = await captureState(client, name, widths, titles)
  await pressEscape(client)
  await waitFor(`the ${name} popup to close`, () => evaluateValue(client, `![...document.querySelectorAll(${JSON.stringify(trigger.popup)})]
    .some((element) => element.checkVisibility({ visibilityProperty: true }) && element.getBoundingClientRect().width > 0)`), { timeoutMs: 10_000 })
  await wait(300)
  return capture
}

// ---------------------------------------------------------------------------
// State fixtures (version 2)
// ---------------------------------------------------------------------------

const MESSAGE_PAGE = (id) => `/api/session/${id}/message`

const fetchMessages = async (origin, id, directory) => {
  const response = await retryApi(origin, MESSAGE_PAGE(id), {}, directory, `messages for ${id}`)
  return Array.isArray(response) ? response : response.data ?? []
}

const CONNECTED_STEP = { type: "connection", status: { state: "connected" } }
const FAILED_STEP = { type: "connection", status: { state: "failed", ...SERVER_ERROR } }

const stateOf = ({ steps, open, base, snapshots, ui }) => {
  const state = {
    version: 2,
    home: base.home,
    settings: base.settings,
    directories: structuredClone(base.directories),
    draft: base.draft,
    steps,
  }
  if (open) state.open = open
  // The startup reads that no `/api/event` frame carries: applied after the
  // replay, like the live app reads them on every control-stream ready frame.
  if (snapshots) state.snapshots = snapshots
  // Local view choices only (the shown permission, the expanded queue, the
  // composer text); the fixture loader applies them after the replay.
  if (ui) state.ui = ui
  return state
}

const ID_PATTERN = /\b(?:ses|msg|evt|prt|per|frm)_[A-Za-z0-9_]+/g
// Queue item ids are `queued-<ms>-<random>`, and two runs never share one.
const QUEUE_ITEM_PATTERN = /queued-\d+-[a-z0-9]+/g
const TIME_KEYS = new Set(["created", "updated", "busy", "idle", "viewed", "archived", "completed", "streamed", "started", "ended", "ran", "openedAt", "doneAt", "suggestDoneAt", "endedAt", "createdAt"])
const FIXED_TIME = 1_794_300_000_000
const FIXED_DURATION_MS = 2_500

/** JSON has six kinds; name them once so the tree walk branches on a kind, not on a runtime type. */
const jsonKind = (value) => {
  if (value === null) return "null"
  if (Array.isArray(value)) return "array"
  const tag = Object.prototype.toString.call(value)
  if (tag === "[object String]") return "string"
  if (tag === "[object Number]") return "number"
  if (tag === "[object Object]") return "object"
  return "other"
}

/**
 * Rewrites the values that differ between two runs of the same seed: session,
 * message, event and part ids to first-appearance ordinals, timestamps to a
 * fixed ladder (a `time` object keeps its fields' real order; objects take
 * their document order, so a millisecond race between two records cannot swap
 * ranks), durations to a fixed value, and the server's pid-named instance
 * folder to one name. Everything else is the response.
 */
const stabilize = (value) => {
  const ids = new Map()
  const counters = new Map()
  const id = (match) => {
    if (!ids.has(match)) {
      const prefix = match.slice(0, match.indexOf("_"))
      const next = (counters.get(prefix) ?? 0) + 1
      counters.set(prefix, next)
      ids.set(match, `${prefix}_reference_${next}`)
    }
    return ids.get(match)
  }
  const queueIds = new Map()
  const queueId = (match) => {
    if (!queueIds.has(match)) queueIds.set(match, `queued_reference_${queueIds.size + 1}`)
    return queueIds.get(match)
  }

  // Each object holding timestamp fields is one group. Its fields keep their
  // real order (a session's busy after idle while a turn runs), and the groups
  // take their document order. The sessions are written in reverse sidebar
  // order first (see `readDirectoryState`), so a document-order ladder still
  // sorts a replay into the order the live app showed, while unrelated
  // timestamps that race in one run and not the next cannot swap ranks.
  const groups = []
  const groupTimes = new WeakMap()
  const collectTimes = (node, key, newestFirst) => {
    const kind = jsonKind(node)
    if (kind === "array") {
      // A message list arrives newest first and the app reverses it to draw
      // it, so its times must descend along the array. Ranking those groups in
      // reverse keeps the transcript's duration and clock monotonic.
      const descending = newestFirst || key === "messages"
      for (const item of node) collectTimes(item, key, descending)
      return
    }
    if (kind !== "object") return
    const entries = Object.entries(node)
      .map(([entryKey, entryValue], index) => ({ key: entryKey, value: entryValue, index }))
      .filter((entry) => jsonKind(entry.value) === "number" && TIME_KEYS.has(entry.key))
    if (entries.length > 0) {
      groups.push({ node, entries, index: groups.length, newestFirst })
    }
    for (const [childKey, child] of Object.entries(node)) collectTimes(child, childKey, newestFirst)
  }
  collectTimes(value, "", false)
  const ordered = [
    ...groups.filter((group) => !group.newestFirst),
    ...groups.filter((group) => group.newestFirst).reverse(),
  ]
  ordered.forEach((group, rank) => {
    const sorted = [...group.entries].sort((left, right) => left.value - right.value || left.index - right.index)
    groupTimes.set(group.node, new Map(sorted.map((entry, offset) => [entry.key, FIXED_TIME + rank * 1000 + offset])))
  })

  const rewrite = (node, key, timeMap = null) => {
    const kind = jsonKind(node)
    if (kind === "string") {
      return node
        .replace(/\/run\/instances\/\d+\//g, "/run/instances/reference/")
        .replace(QUEUE_ITEM_PATTERN, (match) => queueId(match))
        .replace(ID_PATTERN, (match) => id(match))
    }
    if (kind === "array") return node.map((item) => rewrite(item, key, timeMap))
    if (kind === "object") {
      const map = groupTimes.get(node) ?? null
      return Object.fromEntries(Object.entries(node).map(([childKey, child]) => [childKey, rewrite(child, childKey, map)]))
    }
    if (kind === "number" && TIME_KEYS.has(key)) return timeMap?.get(key) ?? node
    if (kind === "number" && key === "durationMs") return FIXED_DURATION_MS
    return node
  }
  return rewrite(value, "")
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

const preparePage = async (client) => {
  // Hide the dev window's construction tape on the current document and every
  // document the app loads afterwards (the seeded reload, a relaunch).
  await client.send("Page.enable")
  await client.send("Page.addScriptToEvaluateOnNewDocument", { source: TAPE_HIDE_SOURCE })
  await evaluateValue(client, `(() => {
    if (document.getElementById("oc-capture-hide-tape")) return true
    const style = document.createElement("style")
    style.id = "oc-capture-hide-tape"
    style.textContent = ${JSON.stringify(TAPE_HIDE_CSS)}
    ;(document.head || document.documentElement).appendChild(style)
    return true
  })()`)
  await client.send("Emulation.setDeviceMetricsOverride", { ...VIEWPORT, deviceScaleFactor: 1, mobile: false })
  // Focus emulation keeps the composer focusable and typing working without
  // the page having real focus in an offscreen window.
  await client.send("Emulation.setFocusEmulationEnabled", { enabled: true })
  await client.send("Accessibility.enable")
  await client.send("Input.setIgnoreInputEvents", { ignore: true })
  // The layout settles at the new viewport.
  await wait(1500)
}

const run = async ({ client, origin, paths, seed, options }) => {
  const captures = []
  const states = []
  await preparePage(client)
  const want = (name) => !options.states || options.states.includes(name)

  const { home, settings, ids, chatDirectories } = await seedFixtures(origin, paths, seed)
  await reloadSeededApp(client)
  const widths = await resetWidths(client)

  const titles = [
    ...seed.sessions.map((session) => session.title),
    seed.workSession.title,
    seed.secondProjectSession.title,
    ...seed.chats.map((chat) => chat.title),
  ]

  const directories = await readDirectories(origin, [paths.project, paths.project2, ...chatDirectories])
  const base = { home, settings, directories, draft: { target: "chat" } }
  const connected = [CONNECTED_STEP]

  const save = (capture, { steps = connected, open } = {}) => {
    captures.push(capture)
    states.push({ name: capture.state, json: `${JSON.stringify(stabilize(stateOf({ steps, open, base })), null, 2)}\n` })
  }

  for (const session of seed.sessions) {
    await waitFor(`the sidebar row "${session.title}"`, () => centerOf(client, sidebarRowExpression(session.title)))
  }
  await wait(1500)
  if (want("empty")) save(await captureState(client, "empty", widths, titles))
  if (want("projects")) save(await captureState(client, "projects", widths, titles))

  for (const name of ["project-popover", "model-popover", "agent-menu"]) {
    if (want(name)) save(await capturePopup(client, name, widths, titles), { open: name })
  }

  // Electron runs the server in its own process, so it has no shell in which
  // the sidecar is down: with every data route aborted, the React app replaces
  // the window with its connection screen and none of the regions exist. The
  // spike design therefore gives server-error a state fixture but no
  // reference (design.md "Parity reference", archived spike). The fixture
  // still carries the seeded setup and the fixed failure the Rust app shows.
  if (want("server-error")) {
    states.push({ name: "server-error", json: `${JSON.stringify(stabilize(stateOf({ steps: [FAILED_STEP], base })), null, 2)}\n` })
  }

  if (!["session", "thinking-menu", "streaming"].some(want)) return { captures, states }

  // The finished turn feeds the session and thinking-menu states; a run that
  // captures only the streaming state must not spend its turn.
  if (want("session") || want("thinking-menu")) {
    const finished = seed.sessions.find((session) => session.state === "session")
    await sendFromComposer(client, { origin, project: paths.project, id: ids.session, title: finished.title, prompt: seed.prompt })
    await waitFor("the finished turn", async () => !isBusy(await sessionTime(origin, ids.session, paths.project)), { timeoutMs: 180_000, intervalMs: 500 })
    await waitFor("the end of the document in the chat column", () => evaluateValue(client, `${renderedExpression("[data-workspace-chat]")}?.innerText.includes("restored from the saved return address")`), { timeoutMs: 30_000 })
    // At rest: the transcript at its end, the way a session opens, with the
    // turn's meta row and the sidebar's busy indicator settled.
    await scrollChatToEnd(client)
    await wait(2000)
    // The Work status telemetry arrives after the turn's stats settle; its
    // Average TTFT row appears a moment later than the rest, and a capture on
    // either side of it changes the control list.
    await waitFor("the Work status telemetry", () => evaluateValue(client, `document.querySelector("[data-right-slot]")?.innerText.includes("Average TTFT")`), { timeoutMs: 60_000 })
    await refreshSessions(origin, directories)
    const finishedMessages = await fetchMessages(origin, ids.session, paths.project)
    const sessionSteps = [CONNECTED_STEP, { type: "openSession", session: ids.session }, { type: "messagesLoaded", session: ids.session, messages: finishedMessages }]
    if (want("session")) save(await captureState(client, "session", await checkWidths(client), titles), { steps: sessionSteps })
    if (want("thinking-menu")) save(await capturePopup(client, "thinking-menu", await checkWidths(client), titles), { steps: sessionSteps, open: "thinking-menu" })
  }

  if (!want("streaming")) return { captures, states }

  const streaming = seed.sessions.find((session) => session.state === "streaming")
  // The stable streaming screen: this state's model runs the same 20 tool calls,
  // streams the document's first heading and its whole opening paragraph, then
  // holds the response open. The text cannot advance while the capture runs, so
  // the transcript and the control set are the same on every run. The held
  // text ends on a finished markdown block: the app reveals a streaming reply a
  // block at a time, and a paragraph cut mid-sentence stays hidden.
  await openSession(client, streaming.title)
  await api(origin, `/api/session/${ids.streaming}/model`, {
    method: "POST",
    // The seed's effort, so the composer keeps the reference's Thinking trigger.
    body: JSON.stringify({ model: { providerID: "perf", id: "agent-20tools-hold", variant: seed.variant } }),
  })
  await api(origin, `/api/session/${ids.streaming}/prompt`, {
    method: "POST",
    body: JSON.stringify({ id: messageId(), text: seed.prompt }),
  })
  await waitFor("the held streaming turn to start", async () => isBusy(await sessionTime(origin, ids.streaming, paths.project)), { timeoutMs: 15_000, intervalMs: 200 })
  const chat = renderedExpression("[data-workspace-chat]")
  // The paragraph's last line is the visible proof the whole held block landed.
  const heldParagraph = FIXTURE_HELD_TEXT.trimEnd()
  const heldTail = heldParagraph.slice(heldParagraph.lastIndexOf("\n") + 1)
  await waitFor("the held document", () => evaluateValue(client, `(() => {
    const chat = ${chat}
    const rendered = (selector) => chat ? [...chat.querySelectorAll(selector)].filter((element) => element.checkVisibility()) : []
    return rendered("h2").some((heading) => heading.textContent.includes("Executing a call"))
      && chat.innerText.includes(${JSON.stringify(heldTail)})
      && rendered("pre").length === 0
  })()`), { timeoutMs: 180_000, intervalMs: 50 })
  // The chrome arrives after the session's first message; the text is held, so
  // waiting cannot change it.
  await waitFor("the streaming chrome", () => evaluateValue(client, `(() => {
    const label = (root) => [...(root?.querySelectorAll("button") ?? [])].map((button) => button.getAttribute("aria-label") ?? "")
    return label(document.querySelector("[data-page-scroll-lock] > header")).includes("Context usage")
      && label(document.querySelector("[data-right-slot]")).includes("Open Git panel")
  })()`), { timeoutMs: 120_000, intervalMs: 500 })
  // Pin the heading near the top of the scroller, the committed reference's
  // view: the transcript is stable (the response is held), so the frozen
  // scroll position is only needed to keep the virtualizer's mounted rows the
  // same. It is released when the turn is interrupted below.
  await evaluateValue(client, `(() => {
    const chat = ${chat}
    const scroller = chat?.querySelector(".chat-scroll")
    const heading = [...(chat?.querySelectorAll("h2") ?? [])].find((element) => element.textContent.includes("Executing a call"))
    if (!scroller || !heading) return false
    const descriptor = Object.getOwnPropertyDescriptor(Element.prototype, "scrollTop")
    const offset = heading.getBoundingClientRect().top - scroller.getBoundingClientRect().top + descriptor.get.call(scroller)
    descriptor.set.call(scroller, offset - 80)
    Object.defineProperty(scroller, "scrollTop", {
      configurable: true,
      get() { return descriptor.get.call(this) },
      set() {},
    })
    scroller.scrollTo = () => {}
    scroller.scrollBy = () => {}
    return true
  })()`)
  // The sidebar row's elapsed label is normalized to <duration>, but its
  // measured width still changes when a value crosses ten seconds ("9s" to
  // "12s"). Wait for two digits; from there to the next minute the label is
  // three tabular characters wide, so the run's bounds are stable.
  await waitFor("the streaming row's two-digit elapsed label", () => evaluateValue(client, `(() => {
    const title = ${sidebarRowExpression(streaming.title)}
    const row = title?.closest("button, [role=button]") ?? title?.parentElement
    return /\\b\\d{2}s\\b/.test(row?.innerText ?? "")
  })()`), { timeoutMs: 120_000, intervalMs: 200 })
  await wait(600)
  const capture = await captureState(client, "streaming", await checkWidths(client), titles)
  if (capture.chatCodeBlocks > 0) throw new Error(`A code block reached the chat column before the streaming capture (${capture.chatCodeBlocks})`)
  if (!isBusy(await sessionTime(origin, ids.streaming, paths.project))) throw new Error("The held streaming turn had ended before the capture")
  await refreshSessions(origin, directories)
  // The held turn has not finished, so the server's copy of its last message
  // carries an empty text part. The screen shows the streamed block, and the
  // model streams exactly FIXTURE_HELD_TEXT, so the replay's message gets that
  // text: the same deterministic block the reference captured.
  const partialMessages = await fetchMessages(origin, ids.streaming, paths.project)
  const newestAssistant = partialMessages.find((message) => message.type === "assistant")
  const newestText = newestAssistant?.content?.find((part) => part.type === "text")
  if (!newestAssistant || !newestText || newestText.text !== "") {
    throw new Error("The streaming session's newest assistant message is not an unfinished empty text")
  }
  newestText.text = FIXTURE_HELD_TEXT
  save(capture, {
    steps: [CONNECTED_STEP, { type: "openSession", session: ids.streaming }, { type: "messagesLoaded", session: ids.streaming, messages: partialMessages }],
  })
  // The capture is done: close the held response.
  await api(origin, `/api/session/${ids.streaming}/interrupt`, { method: "POST" })
  return { captures, states }
}

// ---------------------------------------------------------------------------
// Turn states
// ---------------------------------------------------------------------------

/** The one rendered floating dock above the composer (the permission, form or consent panel). */
const RENDERED_DOCK = `[...document.querySelectorAll("form[data-prompt-history-search-owner] [role=dialog]")]
  .find((element) => element.checkVisibility({ visibilityProperty: true }) && element.getBoundingClientRect().width > 0)`

/** The rendered queue strip's top-row group inside the composer. */
const QUEUE_GROUP = `[...document.querySelectorAll("[data-dictation-chain]:not([data-dictation-chain] [data-dictation-chain]) > div > div[role=group]")]
  .find((element) => element.checkVisibility({ visibilityProperty: true }) && element.getBoundingClientRect().width > 0)`

const dockText = (client) => evaluateValue(client, `(() => { const dock = ${RENDERED_DOCK}; return dock ? dock.innerText : null })()`)

const waitForDockText = (client, label, text) => waitFor(label, async () => {
  const current = await dockText(client)
  return current?.includes(text) ? current : null
})

const waitForDockGone = (client) => waitFor("the dock to close", async () => (await dockText(client)) === null, { timeoutMs: 20_000 })

/** Toasts sit outside every region and auto-dismiss; a capture never holds one. */
const waitForNoToasts = (client) => waitFor("the toasts to clear", () => evaluateValue(client, `document.querySelectorAll("[data-sonner-toast]").length === 0`), { timeoutMs: 20_000 })

/** A session the app can hold while a captured session is deleted. */
const NEUTRAL_SESSION = "Reference: finished turn"

const leaveCapturedSession = async (client) => {
  await openSession(client, NEUTRAL_SESSION)
}

/**
 * Waits for the open session's turn to end before its session is deleted. A
 * session deleted while its turn runs leaves the app's current session stale
 * and the next session's Stop button missing.
 */
const waitForTurnSettled = (client) => waitFor(
  "the turn to settle",
  () => evaluateValue(client, `!document.querySelector('button[aria-label="Stop generating"]')`),
  { timeoutMs: 30_000 },
)

/** A dock button or link by its accessible name, else its whole text. */
const dockControlExpression = (label) => `(() => {
  const dock = ${RENDERED_DOCK}
  if (!dock) return null
  const wanted = ${JSON.stringify(label)}
  return [...dock.querySelectorAll("button, a[href]")].find((element) =>
    element.checkVisibility({ visibilityProperty: true })
    && ((element.getAttribute("aria-label") ?? "").trim() === wanted
      || (element.textContent ?? "").replace(/\\s+/g, " ").trim() === wanted))
})()`

const clickDockControl = async (client, label) => {
  const target = await waitFor(`the dock control "${label}"`, () => centerOf(client, dockControlExpression(label)))
  await click(client, target)
  await wait(400)
}

const pressEnter = (client) => sendInput(client, ["keyDown", "keyUp"].map((type) => [
  "Input.dispatchKeyEvent",
  { type, key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 },
]))

/** Opens a seeded session from its sidebar row and waits for its column to switch. */
const openSession = async (client, title) => {
  const rowExpression = `(() => { const element = ${sidebarRowExpression(title)}; element?.scrollIntoView({ block: "center", behavior: "instant" }); return element })()`
  const row = await waitFor(`the sidebar row "${title}"`, () => centerOf(client, rowExpression))
  await wait(300)
  await click(client, row)
  await waitFor(`"${title}" in the header`, () => evaluateValue(client, `document.querySelector("[data-page-scroll-lock] > header")?.textContent.includes(${JSON.stringify(title)})`))
  await wait(1500)
}

/** Types into the open composer and presses Enter, then waits for the turn to start. */
const sendInOpenSession = async (client, { origin, project, id, prompt }) => {
  const editor = await waitFor("the composer", () => centerOf(client, renderedExpression("[role=textbox][contenteditable=true]")))
  await click(client, editor)
  await sendInput(client, [["Input.insertText", { text: prompt }]])
  await waitFor("the prompt in the composer", () => evaluateValue(client, `${renderedExpression("[role=textbox][contenteditable=true]")}?.textContent.includes(${JSON.stringify(prompt)})`), { timeoutMs: 5000 })
  await pressEnter(client)
  await waitFor("the turn to start", async () => isBusy(await sessionTime(origin, id, project)), { timeoutMs: 15_000, intervalMs: 200 })
}

const typeInComposer = async (client, text) => {
  const editor = await waitFor("the composer", () => centerOf(client, renderedExpression("[role=textbox][contenteditable=true]")))
  await click(client, editor)
  await sendInput(client, [["Input.insertText", { text }]])
  await waitFor("the typed text in the composer", () => evaluateValue(client, `${renderedExpression("[role=textbox][contenteditable=true]")}?.innerText.includes(${JSON.stringify(text)})`), { timeoutMs: 5000 })
}

const clearComposer = async (client) => {
  const editor = await waitFor("the composer", () => centerOf(client, renderedExpression("[role=textbox][contenteditable=true]")))
  await click(client, editor)
  await sendInput(client, [
    ...["keyDown", "keyUp"].map((type) => ["Input.dispatchKeyEvent", { type, key: "a", code: "KeyA", windowsVirtualKeyCode: 65, modifiers: 2 }]),
    ...["keyDown", "keyUp"].map((type) => ["Input.dispatchKeyEvent", { type, key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8 }]),
  ])
  await waitFor("the composer to clear", () => evaluateValue(client, `${renderedExpression("[role=textbox][contenteditable=true]")}?.innerText.trim() === ""`), { timeoutMs: 5000 })
}

const deleteSession = async (origin, id) => {
  try {
    await api(origin, `/api/session/${encodeURIComponent(id)}`, { method: "DELETE" })
  } catch (error) {
    // Deleting a parent can cascade to its children, so a later delete of the
    // child finds it already gone; that is the state the capture wants.
    if (!String(error.message).includes("answered 404")) throw error
  }
}

/**
 * The startup reads a turn state shows beside the session and its messages.
 * Pending permissions and forms replay as the `permission.asked` and
 * `form.created` events the reducer already handles; the queue, auto-accept
 * and routing snapshots have no event, so they go in the fixture's `snapshots`
 * block.
 */
const stateSnapshot = async (origin, { directory, session }) => {
  const location = encodeURIComponent(directory)
  const [permissions, forms, queue, autoAccept, routing] = await Promise.all([
    api(origin, `/api/permission/request?location[directory]=${location}`),
    api(origin, `/api/form?location[directory]=${location}`),
    api(origin, "/api/message-queue"),
    api(origin, "/api/permission-auto-accept"),
    api(origin, "/api/routing").catch(() => null),
  ])
  return {
    permissions: permissions?.data ?? [],
    forms: forms?.data ?? [],
    snapshots: {
      messageQueue: {
        revision: queue?.revision ?? 0,
        sessions: (queue?.sessions ?? []).filter((entry) => entry.sessionId === session),
      },
      permissionAutoAccept: { revision: autoAccept?.revision ?? 0, modes: autoAccept?.modes ?? {}, sessions: autoAccept?.sessions ?? {} },
      routing: {
        available: Boolean(routing?.available),
        jevAvailable: Boolean(routing?.jevAvailable),
        heldPermissions: routing?.heldPermissions ?? [],
      },
    },
  }
}

/** The events that put the pending requests back into the reducer. */
const snapshotEvents = ({ permissions, forms }) => {
  const created = (index) => 1_794_300_000_000 + index
  return [
    ...permissions.map((request, index) => ({
      id: `evt_permission_${index + 1}`,
      created: created(index),
      type: "permission.asked",
      data: request,
    })),
    ...forms.map((form, index) => ({
      id: `evt_form_${index + 1}`,
      created: created(permissions.length + index),
      type: "form.created",
      data: { form },
    })),
  ]
}

/** A session-scoped form is the question model's; a location-scoped one is seeded through the route. */
const seedGlobalForm = async (origin, directory, body) => {
  const created = await api(origin, "/api/session/global/form", { method: "POST", body: JSON.stringify(body) }, directory)
  if (created?.data?.sessionID !== "global") throw new Error(`The seeded form is not location-scoped: ${JSON.stringify(created).slice(0, 300)}`)
  return created.data.id
}

const cancelForm = async (origin, directory, form) => {
  await api(origin, `/api/session/${encodeURIComponent(form.sessionID)}/form/${encodeURIComponent(form.id)}`, { method: "DELETE" }, directory)
}

const enqueueItem = async (origin, { session, directory, text, sendConfig }) => {
  const result = await api(origin, `/api/message-queue/sessions/${encodeURIComponent(session)}/items`, {
    method: "POST",
    body: JSON.stringify({ directory, item: { content: text, text, sendConfig } }),
  })
  if (!result?.itemId) throw new Error(`Enqueueing "${text}" failed: ${JSON.stringify(result).slice(0, 300)}`)
  return result.itemId
}

/**
 * Adds a child session the directory list does not carry. React loads a
 * session's children through the same `parentID` pages, so the fixture's
 * session list is the one the tree and the sidebar read.
 */
const includeChild = async (origin, directories, project, parent) => {
  const entry = directories[project]
  if (!entry) return
  const known = new Set(entry.sessions.map((session) => session.id))
  const response = await api(origin, `/api/session?parentID=${encodeURIComponent(parent)}&limit=50`, {}, project)
  const children = Array.isArray(response) ? response : response.data ?? []
  for (const child of children) {
    if (known.has(child.id)) continue
    entry.sessions.push(child)
    known.add(child.id)
  }
}

/** Re-reads the session lists, then restores the child sessions a refresh drops. */
const refreshTurnSessions = async (origin, directories, project, parents) => {
  await refreshSessions(origin, directories)
  for (const parent of parents) await includeChild(origin, directories, project, parent)
}

const runTurn = async ({ client, origin, paths, seed, options }) => {
  const captures = []
  const states = []
  await preparePage(client)
  const selected = new Set(options.states ?? TURN_STATES)
  const want = (name) => selected.has(name)

  const { home, settings, ids, chatDirectories } = await seedFixtures(origin, paths, seed)
  await reloadSeededApp(client)
  await resetWidths(client)
  const titles = [
    ...seed.sessions.map((session) => session.title),
    seed.workSession.title,
    seed.secondProjectSession.title,
    ...seed.chats.map((chat) => chat.title),
  ]
  const directories = await readDirectories(origin, [paths.project, paths.project2, ...chatDirectories])
  const base = { home, settings, directories, draft: { target: "chat" } }
  const childParents = []

  const saveTurn = async (name, { session, snapshot, ui }) => {
    await wait(600)
    await waitForNoToasts(client)
    const capture = await captureState(client, name, await checkWidths(client), titles)
    const messages = await fetchMessages(origin, session, paths.project)
    await refreshTurnSessions(origin, directories, paths.project, childParents)
    const events = snapshotEvents(snapshot)
    const steps = [
      CONNECTED_STEP,
      { type: "openSession", session },
      { type: "messagesLoaded", session, messages },
    ]
    if (events.length > 0) steps.push({ type: "events", events })
    captures.push(capture)
    states.push({
      name,
      json: `${JSON.stringify(stabilize(stateOf({ steps, snapshots: snapshot.snapshots, ui, base })), null, 2)}\n`,
    })
  }

  const displayName = (id) => {
    const name = fixtureProviderConfig(FIXTURE_PORT).provider.perf.models[id]?.name ?? id
    return name.length <= 40 ? name : `${name.slice(0, 37)}...`
  }
  // The composer keeps the previous session's model until the new session's
  // record arrives; a prompt sent before that runs the wrong model (a
  // silently queued turn ran the question model and left a form open).
  const waitForComposerModel = (modelId) => waitFor(`the composer model "${displayName(modelId)}"`, () => evaluateValue(client, `(() => {
    const label = document.querySelector(".model-controls__model-label")?.textContent?.trim() ?? ""
    return label.includes(${JSON.stringify(displayName(modelId))})
  })()`), { timeoutMs: 20_000 })
  const openTurn = async (title, modelId) => {
    const project = paths.project
    const id = await createSession(origin, { title, directory: project, model: { providerID: "perf", id: modelId } })
    titles.push(title)
    await openSession(client, title)
    await waitForComposerModel(modelId)
    return id
  }

  // `permission-bash`, `permission-edit`, `permission-write`: one tool asks
  // before it runs; the dock shows the request, then a reply ends the turn.
  const permissionCases = [
    { state: "permission-bash", title: "Turn: bash permission", model: `${FIXTURE_TOOL_RATE}cps`, tool: "bash", prompt: "Run the fixture command.", shown: "echo fixture-permission", decision: "once" },
    { state: "permission-edit", title: "Turn: edit permission", model: `${FIXTURE_TOOL_RATE}cps`, tool: "edit", prompt: "Edit the fixture file.", shown: FIXTURE_EDIT_FILE, decision: "reject" },
    { state: "permission-write", title: "Turn: write permission", model: `${FIXTURE_TOOL_RATE}cps`, tool: "write", prompt: "Write the fixture file.", shown: "fixture-write.txt", decision: "once" },
  ]
  if (permissionCases.some((entry) => want(entry.state)) || want("permission-two")) {
    writeFileSync(join(paths.project, FIXTURE_EDIT_FILE), FIXTURE_EDIT_CONTENT)
  }
  for (const entry of permissionCases) {
    if (!want(entry.state)) continue
    const session = await openTurn(entry.title, `${entry.tool}-${entry.model}`)
    await sendInOpenSession(client, { origin, project: paths.project, id: session, prompt: entry.prompt })
    await waitForDockText(client, `the ${entry.state} dock`, entry.shown)
    const snapshot = await stateSnapshot(origin, { directory: paths.project, session })
    if (snapshot.permissions.length !== 1) throw new Error(`The ${entry.state} state has ${snapshot.permissions.length} pending permissions, not one`)
    await saveTurn(entry.state, { session, snapshot })
    await api(origin, `/api/session/${session}/permission/${snapshot.permissions[0].id}/reply`, {
      method: "POST",
      body: JSON.stringify({ decision: entry.decision }),
    })
    await waitForDockGone(client)
    await waitForTurnSettled(client)
    // A rejected permission ends the turn without a `time.idle` stamp on the
    // session record, so the session would read busy forever in every later
    // fixture. Leave it first: deleting the open session leaves the app
    // polling a 404 and its next session's status stale.
    await leaveCapturedSession(client)
    await deleteSession(origin, session)
  }

  // `permission-two`: the open session's own request and a child session's,
  // so the dock shows the dots and the subagent mark. The child's turn starts
  // first from its own composer, then the parent's request joins it.
  if (want("permission-two")) {
    const title = "Turn: two permissions"
    const parent = await createSession(origin, { title, directory: paths.project, model: { providerID: "perf", id: `bash-${FIXTURE_TOOL_RATE}cps` } })
    titles.push(title)
    const child = await createSession(origin, {
      title: "Turn: child edit permission",
      directory: paths.project,
      parentID: parent,
      model: { providerID: "perf", id: `edit-${FIXTURE_TOOL_RATE}cps` },
    })
    childParents.push(parent)
    await includeChild(origin, directories, paths.project, parent)
    await openSession(client, title)
    await waitForComposerModel(`bash-${FIXTURE_TOOL_RATE}cps`)
    // The parent's prompt goes first, and its request must be pending before
    // the child's turn starts: a composer send dismisses the tree's pending
    // requests, and the order of the two asks keeps the session activity (and
    // so the fixture's times and list order) deterministic.
    await sendInOpenSession(client, { origin, project: paths.project, id: parent, prompt: "Run the fixture command." })
    await waitForDockText(client, "the parent's shell request", "echo fixture-permission")
    // The child has no sidebar row; its prompt goes through the server, with
    // the message id the app's composer mints.
    await api(origin, `/api/session/${child}/prompt`, { method: "POST", body: JSON.stringify({ id: messageId(), text: "Edit the fixture file." }) })
    await waitFor("the child turn to start", async () => isBusy(await sessionTime(origin, child, paths.project)), { timeoutMs: 15_000, intervalMs: 200 })
    const twoDock = await waitForDockText(client, "the dock with two requests", "of 2")
    if (!twoDock.includes("shell")) throw new Error("The two-request dock is not showing the parent's shell request")
    // The reference shows the second request (the child's edit) current; select
    // its dot. The subagent mark belongs to that request.
    await clickDockControl(client, "Request 2: edit")
    const childDock = await waitForDockText(client, "the edit request in the dock", FIXTURE_EDIT_FILE)
    if (!childDock.includes("From subagent")) throw new Error("The second permission dock does not mark the subagent request")
    if (!childDock.includes("2 of 2")) throw new Error("The second permission dot did not become current")
    const snapshot = await stateSnapshot(origin, { directory: paths.project, session: parent })
    if (snapshot.permissions.length !== 2) throw new Error(`The two-permission state has ${snapshot.permissions.length} pending permissions, not two`)
    await saveTurn("permission-two", { session: parent, snapshot, ui: { permissionIndex: 1 } })
    for (const request of snapshot.permissions) {
      await api(origin, `/api/session/${request.sessionID}/permission/${request.id}/reply`, { method: "POST", body: JSON.stringify({ decision: "reject" }) })
    }
    await waitForDockGone(client)
    await waitForTurnSettled(client)
    await leaveCapturedSession(client)
    await deleteSession(origin, child)
    await deleteSession(origin, parent)
    childParents.length = 0
  }

  // `form-choice` uses the question model's three-field form at its first
  // step. The question form's fields are the definition the other form states
  // reuse, reordered so the wanted type is the first step (a state fixture has
  // no click to reach a later step).
  const questionTitle = "Turn: question form"
  let questionFields = null
  if (want("form-choice")) {
    const questionSession = await openTurn(questionTitle, `question-${FIXTURE_TOOL_RATE}cps`)
    await sendInOpenSession(client, { origin, project: paths.project, id: questionSession, prompt: "Ask the fixture questions." })
    await waitForDockText(client, "the question form dock", "Fixture choice")
    const snapshot = await stateSnapshot(origin, { directory: paths.project, session: questionSession })
    if (snapshot.forms.length !== 1 || snapshot.forms[0].fields?.length !== 3) throw new Error(`The question form is not the fixture's three-field form: ${JSON.stringify(snapshot.forms).slice(0, 300)}`)
    questionFields = snapshot.forms[0].fields
    await saveTurn("form-choice", { session: questionSession, snapshot })
    await cancelForm(origin, paths.project, snapshot.forms[0])
    await waitForDockGone(client)
    await waitForTurnSettled(client)
    await leaveCapturedSession(client)
    await deleteSession(origin, questionSession)
  }

  // The form states after the first use their own seeded form, so each one
  // opens on its wanted field. A dedicated empty session keeps the chat column
  // out of the question turn's tool errors.
  if (["form-multiselect", "form-text", "form-external", "websearch-consent"].some(want)) {
    if (!questionFields) {
      // `form-choice` was not captured in this run: read the question form's
      // fields from the unit 1 fixture instead.
      const fixture = JSON.parse(readFileSync(join(PACKAGE_ROOT, "fixtures/api/form-list.json"), "utf8"))
      const form = (fixture.data ?? []).find((entry) => entry.metadata?.kind === "question")
      if (!form) throw new Error("fixtures/api/form-list.json has no question form")
      questionFields = form.fields
    }
    const formsSession = await openTurn("Turn: forms", "stream-300cps")
    const rotation = (keys) => keys.map((key) => questionFields.find((field) => field.key === key))
    const seeded = [
      { state: "form-multiselect", body: { id: "frm_fixture_multiselect", title: "Questions", metadata: { kind: "question" }, fields: rotation(["q1", "q0", "q2"]) }, shown: "Fixture multiselect" },
      { state: "form-text", body: { id: "frm_fixture_text", title: "Questions", metadata: { kind: "question" }, fields: rotation(["q2", "q0", "q1"]) }, shown: "Fixture free text" },
      {
        state: "form-external",
        body: {
          id: "frm_fixture_external",
          title: "Questions",
          metadata: { kind: "question" },
          fields: [
            { key: "url", title: "Fixture external link", description: "Open the fixture link", type: "external", url: "https://example.com/fixture", required: false },
            { key: "q0", title: "Fixture choice", description: "Which fixture option should the capture answer?", type: "string", options: questionFields[0]?.options ?? [], custom: true },
          ],
        },
        shown: "Fixture external link",
      },
      {
        state: "websearch-consent",
        body: {
          id: "frm_fixture_websearch",
          title: "Web search",
          metadata: { kind: "websearch.provider" },
          fields: [{
            key: "choice",
            title: "Web search provider",
            type: "string",
            options: [
              { value: "allow", label: "Allow" },
              { value: "choose", label: "Choose provider" },
              { value: "disable", label: "Turn off web search" },
            ],
          }],
        },
        shown: "Allow",
      },
    ]
    for (const entry of seeded) {
      if (!want(entry.state)) continue
      const formId = await seedGlobalForm(origin, paths.project, entry.body)
      await waitForDockText(client, `the ${entry.state} dock`, entry.shown)
      const snapshot = await stateSnapshot(origin, { directory: paths.project, session: formsSession })
      const form = snapshot.forms.find((candidate) => candidate.id === formId)
      if (!form) throw new Error(`The ${entry.state} form is not in the pending list`)
      await saveTurn(entry.state, { session: formsSession, snapshot })
      await cancelForm(origin, paths.project, form)
      await waitForDockGone(client)
    }
    await leaveCapturedSession(client)
    await deleteSession(origin, formsSession)
  }

  // `recovered-question`: the question turn is interrupted while its form is
  // pending, so the tool part errors and the dock offers the question again.
  if (want("recovered-question")) {
    const session = await openTurn("Turn: recovered question", `question-${FIXTURE_TOOL_RATE}cps`)
    await sendInOpenSession(client, { origin, project: paths.project, id: session, prompt: "Ask the fixture questions." })
    await waitForDockText(client, "the question form dock", "Fixture choice")
    await api(origin, `/api/session/${session}/interrupt`, { method: "POST" })
    await waitForDockText(client, "the recovered question dock", "The turn stopped while waiting")
    const snapshot = await stateSnapshot(origin, { directory: paths.project, session })
    if (snapshot.forms.length > 0) throw new Error("The interrupted turn still has a pending form")
    await saveTurn("recovered-question", { session, snapshot })
    await clickDockControl(client, "Dismiss")
    await waitForDockGone(client)
    await waitForTurnSettled(client)
    await leaveCapturedSession(client)
    await deleteSession(origin, session)
  }

  // Queue states: a silent fixture turn holds the session busy while items
  // wait behind it. The strip's rows are the dock region.
  if (["queue-one", "queue-three-expanded", "busy-with-text"].some(want)) {
    const silentModel = `think-30s`
    if (want("queue-one") || want("queue-three-expanded")) {
      const session = await openTurn("Turn: queue", silentModel)
      const sendConfig = { providerID: "perf", modelID: "stream-1200cps" }
      await sendInOpenSession(client, { origin, project: paths.project, id: session, prompt: "Start the turn the queue waits behind." })
      const items = ["First queued fixture message.", "Second queued fixture message.", "Third queued fixture message."]
      await enqueueItem(origin, { session, directory: paths.project, text: items[0], sendConfig })
      await waitFor("the one queued row", () => evaluateValue(client, `${QUEUE_GROUP}?.innerText.includes(${JSON.stringify(items[0])})`), { timeoutMs: 20_000 })
      if (want("queue-one")) {
        const snapshot = await stateSnapshot(origin, { directory: paths.project, session })
        await saveTurn("queue-one", { session, snapshot })
      }
      await enqueueItem(origin, { session, directory: paths.project, text: items[1], sendConfig })
      await enqueueItem(origin, { session, directory: paths.project, text: items[2], sendConfig })
      await waitFor("the collapsed queue header", () => evaluateValue(client, `${QUEUE_GROUP}?.innerText.includes("Queued messages 3")`), { timeoutMs: 20_000 })
      const expand = `[...document.querySelectorAll("[data-dictation-chain]:not([data-dictation-chain] [data-dictation-chain]) button[aria-expanded]")]
        .find((element) => element.checkVisibility({ visibilityProperty: true }) && element.textContent.includes("Queued messages"))`
      const toggle = await waitFor("the queue expand toggle", () => centerOf(client, expand))
      await click(client, toggle)
      await waitFor("the expanded queue rows", () => evaluateValue(client, `document.querySelectorAll('[aria-label="Remove from queue"]').length >= 3`), { timeoutMs: 10_000 })
      if (want("queue-three-expanded")) {
        const snapshot = await stateSnapshot(origin, { directory: paths.project, session })
        await saveTurn("queue-three-expanded", { session, snapshot, ui: { queueExpanded: true } })
      }
      await api(origin, `/api/message-queue/sessions/${session}/take`, { method: "POST" })
      await api(origin, `/api/session/${session}/interrupt`, { method: "POST" })
      await waitFor("the queue strip to clear", () => evaluateValue(client, `!${QUEUE_GROUP} || !${QUEUE_GROUP}.innerText.includes("queued fixture message")`), { timeoutMs: 20_000 })
      await waitForTurnSettled(client)
      await leaveCapturedSession(client)
      await deleteSession(origin, session)
    }

    if (want("busy-with-text")) {
      const session = await openTurn("Turn: busy with text", silentModel)
      await sendInOpenSession(client, { origin, project: paths.project, id: session, prompt: "Start the turn the message waits behind." })
      const draft = "A message that waits for the turn."
      await typeInComposer(client, draft)
      await waitFor("the Stop button", () => evaluateValue(client, `Boolean(document.querySelector('button[aria-label="Stop generating"]'))`), { timeoutMs: 10_000 })
      await waitFor("the follow-up button", () => evaluateValue(client, `Boolean(document.querySelector('button[aria-label="Queue message"]'))`), { timeoutMs: 10_000 })
      const snapshot = await stateSnapshot(origin, { directory: paths.project, session })
      await saveTurn("busy-with-text", { session, snapshot, ui: { composer: draft } })
      await clearComposer(client)
      await api(origin, `/api/session/${session}/interrupt`, { method: "POST" })
      await waitForTurnSettled(client)
      await leaveCapturedSession(client)
      await deleteSession(origin, session)
    }
  }

  return { captures, states }
}

// ---------------------------------------------------------------------------
// Output checks
// ---------------------------------------------------------------------------

/**
 * The fixtures are committed: no output may name the user. Checked on every
 * JSON before any file is written; the PNGs are checked by eye (the seeded
 * project, HOME and data dir all live under ROOT, which holds no user name).
 */
const personalStrings = () => [homedir(), REPO_ROOT, userInfo().username].filter((value) => value && value.length > 0)

const assertNothingPersonal = (outputs) => {
  const found = outputs.flatMap(({ name, json }) => [
    ...personalStrings().filter((value) => json.includes(value)).map((value) => `${name} contains ${JSON.stringify(value)}`),
    ...(json.includes("/../") || json.includes('"../') ? [`${name} contains a ../ path segment`] : []),
  ])
  if (found.length > 0) throw new Error(`Refusing to write the fixtures: ${found.join("; ")}`)
}

const main = async () => {
  const options = parseArgs(process.argv.slice(2))
  const seed = JSON.parse(readFileSync(join(PACKAGE_ROOT, "fixtures/seed.json"), "utf8"))
  const turnRun = options.turn || (options.states?.some((name) => TURN_STATES.includes(name)) ?? false)
  const unknown = (options.states ?? []).filter((name) => turnRun ? !TURN_STATES.includes(name) : !LEGACY_STATES.includes(name))
  if (unknown.length > 0) throw new Error(`Unknown state${unknown.length > 1 ? "s" : ""}: ${unknown.join(", ")}`)
  const output = turnRun ? join(options.output, TURN_DIRECTORY) : options.output
  const statesOutput = turnRun ? join(options.statesOutput, TURN_DIRECTORY) : options.statesOutput
  for (const [label, port] of [["CDP", CDP_PORT], ["fixture provider", FIXTURE_PORT]]) {
    if (await portInUse(port)) throw new Error(`Port ${port} (${label}) is already in use; stop that process first`)
  }
  const personalInRoot = personalStrings().filter((value) => ROOT.includes(value))
  if (personalInRoot.length > 0) throw new Error(`The temporary root ${ROOT} names the user; set TMPDIR to a neutral directory`)

  ensureElectron()
  const paths = prepareRoot(seed)
  const fixture = await startFixture({ port: FIXTURE_PORT, logFile: join(ROOT, "fixture.log") })
  let child = null
  let client = null
  let completed = false
  let result = { captures: [], states: [] }
  try {
    const [uiPort, apiPort] = [await reservePort(), await reservePort()]
    child = startWindow(windowEnv({ paths, seed, fixturePort: FIXTURE_PORT, uiPort, apiPort, turn: turnRun }))
    log(`headless Electron pid ${child.electron?.pid}, UI port ${uiPort}, API port ${apiPort}, root ${ROOT}`)
    const page = await connectPage(uiPort, child)
    client = page.client
    result = turnRun
      ? await runTurn({ client, origin: page.origin, paths, seed, options })
      : await run({ client, origin: page.origin, paths, seed, options })
    completed = true
  } finally {
    client?.close()
    if (child) {
      const left = await stopWindow(child)
      if (left.length > 0) log(`warning: processes ${left.join(", ")} under ${ROOT} needed SIGKILL`)
    }
    await fixture.stop()
    // A failed run keeps its logs; the next run removes the root first.
    if (completed) rmSync(ROOT, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 })
    else log(`kept ${ROOT} with electron-dev.log and fixture.log`)
  }
  assertNothingPersonal([
    { name: "seed.json", json: JSON.stringify(seed) },
    ...result.captures.map(({ state, json }) => ({ name: `${state}.json`, json })),
    ...result.states.map(({ name, json }) => ({ name: `states/${name}.json`, json })),
  ])
  mkdirSync(output, { recursive: true })
  for (const { state, json, png } of result.captures) {
    writeFileSync(join(output, `${state}.json`), json)
    writeFileSync(join(output, `${state}.png`), png)
  }
  mkdirSync(statesOutput, { recursive: true })
  for (const { name, json } of result.states) writeFileSync(join(statesOutput, `${name}.json`), json)
  log(`wrote ${result.captures.length} reference states to ${output} and ${result.states.length} state fixtures to ${statesOutput}; no JSON names ${personalStrings().length} personal strings (home, repo path, user name)`)
}

main().catch((error) => {
  console.error(`[capture-reference] failed: ${error.stack ?? error.message}`)
  process.exitCode = 1
})
