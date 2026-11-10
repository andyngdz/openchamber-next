#!/usr/bin/env node
/**
 * Captures the layout reference the GPUI parity test compares against:
 * `fixtures/reference/v2/<state>.json` and `<state>.png`, taken from the Electron
 * dev window at a 1440x900 content area, and writes the same run's server
 * responses as version 2 state fixtures to `fixtures/states/v2/<state>.json`.
 * The new reference set is staged under `reference/v2` until the parity loader
 * reads it; `fixtures/reference/` keeps the set the test reads today.
 *
 * The window runs isolated from the user's own app: its own HOME (and XDG
 * directories, so OpenCode's database lands there too), OPENCHAMBER_DATA_DIR,
 * Electron profile (OPENCHAMBER_DESKTOP_USER_DATA_DIR, which also avoids the
 * dev profile's single-instance lock), HMR ports, and an OpenCode config that
 * registers the fixture provider (`scripts/perf/fixture-provider.mjs`) twice,
 * under `perf` and `perf-alt`, with several models each. `perf`'s agent model
 * carries two variants so the Thinking menu has content. Display variables
 * (DISPLAY, WAYLAND_DISPLAY, XDG_RUNTIME_DIR, DBUS_SESSION_BUS_ADDRESS) pass
 * through, so the window opens on the user's session. Stub `gh`, `glab` and
 * `gt` executables come first on PATH, since `gh auth token` reads the system
 * keyring even with an empty HOME. Everything lives under one fixed temporary
 * root, removed at the end (kept, with its logs, when a run fails). A fresh
 * Electron profile relaunches the app once shortly after its first start; the
 * script waits for a page that stays mounted.
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
 *   popover          the open Base UI popup: the rendered element with role
 *                    `dialog` or `menu` that is inside none of the other
 *                    regions. Both pickers use `@base-ui/react`, which portals
 *                    the popup under body, so no other selector can find it
 *                    without also matching closed menus. One of the first six
 *                    regions is found before a popup is looked for, so the
 *                    rule is "inside no other region", not a fixed ancestor.
 *
 * A control belongs to the first region in the order composer,
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
 * State fixtures (version 2, `fixtures/states/v2/<state>.json`): `home` is the
 * `/api/fs/home` response, `settings` the `surface=desktop` settings document,
 * each `directories` entry the config documents and the provider, model, agent
 * and session payloads for that directory, `draft` the composer target, `steps`
 * the reducer steps (connection, the open session and its messages), and `open`
 * the popup a state shows, when it shows one. Session and message ids and every
 * timestamp are rewritten to stable values, and instance paths and durations
 * to fixed ones, so two runs produce byte-identical JSON while the records keep
 * their shape and order.
 *
 * The fixtures are committed, so no JSON may name the user: the run fails and
 * writes nothing when one contains the home directory, the repository path or
 * the user name. No path carries a `../` segment.
 *
 * Usage: node packages/desktop-rs/tools/capture-reference.mjs [--output <dir>]
 */

import { spawn } from "node:child_process"
import { createWriteStream, existsSync, mkdirSync, readFileSync, readdirSync, readlinkSync, rmSync, writeFileSync } from "node:fs"
import { connect } from "node:net"
import { homedir, tmpdir, userInfo } from "node:os"
import { basename, dirname, join, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import process from "node:process"

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
const { fixtureProviderConfig } = await importPerf("fixture-provider.mjs")
const { ensureProject, isolatedEnv, startFixture } = await importPerf("isolated-server.mjs")
// A fixed root, so paths the UI shows are the same on every run. The CDP and
// fixture ports are fixed too, so two runs cannot overlap and provider
// settings do not carry a new port each time; both must be free before a run.
const ROOT = join(tmpdir(), "oc-desktop-rs-reference")
const CDP_PORT = 9222
const FIXTURE_PORT = 4601
const VIEWPORT = { width: 1440, height: 900 }
const SIDEBAR_WIDTH = 280
// WORK_STATUS_COLUMN_WIDTH in packages/ui/src/components/chat/work-status/useWorkStatusVisibility.ts:
// the 300 px card and its 24 px gutter. The card has no persisted width.
const WORK_STATUS_CARD_WIDTH = 300
const WORK_STATUS_COLUMN_WIDTH = 324
// Over the right slot, under the Work status card: no control, no hover reveal.
const POINTER_REST = { x: 1230, y: 820 }
// Long enough past the heading to show its first paragraph, short of the
// first code fence; the text is cut here so two runs write the same state.
const STREAMING_TEXT_CHARACTERS = 400
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

Starts the Electron dev window isolated from the user's app, seeds two projects,
their sessions, two Chat sessions and an In work session from fixtures/seed.json,
and writes <state>.json and <state>.png for empty, projects, project-popover,
model-popover, agent-menu, session, thinking-menu and streaming to --output
(default fixtures/reference/v2). It also writes the same run's responses as version
2 state fixtures to fixtures/states/v2, including server-error, which has no
Electron reference.
Needs ports ${CDP_PORT} (CDP) and ${FIXTURE_PORT} (fixture provider) free and a graphical session.`

const parseArgs = (argv) => {
  const options = { output: join(PACKAGE_ROOT, "fixtures/reference/v2"), statesOutput: join(PACKAGE_ROOT, "fixtures/states/v2") }
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index]
    if (value === "--help") { console.log(HELP); process.exit(0) }
    else if (value === "--output") options.output = resolve(argv[++index])
    else if (value === "--states-output") options.statesOutput = resolve(argv[++index])
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

// ---------------------------------------------------------------------------
// Isolated window
// ---------------------------------------------------------------------------

/** The fixture provider registered under two ids, as the capture's OpenCode config. */
const referenceProviderConfig = (port) => {
  const base = fixtureProviderConfig(port).provider.perf
  const models = (ids) => Object.fromEntries(ids.map((id) => [id, base.models[id]]))
  return {
    provider: {
      perf: {
        ...base,
        models: {
          ...models(["stream-300cps", "code-1200cps"]),
          // v1 spelling: a keyed map. An array drops the whole provider in OpenCode v2.
          "agent-20tools-300cps": {
            ...base.models["agent-20tools-300cps"],
            variants: { high: { reasoningEffort: "high" }, low: { reasoningEffort: "low" } },
          },
        },
      },
      "perf-alt": {
        ...base,
        name: "Performance fixture (second)",
        models: models(["stream-1200cps", "unicode-300cps"]),
      },
    },
  }
}

const prepareRoot = (seed) => {
  rmSync(ROOT, { recursive: true, force: true })
  const paths = {
    home: join(ROOT, "home"),
    dataDir: join(ROOT, "oc-data"),
    userData: join(ROOT, "electron-userdata"),
    project: join(ROOT, seed.project.name),
    project2: join(ROOT, seed.secondProject.name),
    bin: join(ROOT, "bin"),
  }
  for (const dir of [paths.home, paths.dataDir, paths.userData, paths.bin]) mkdirSync(dir, { recursive: true })
  // `gh auth token` reads the system keyring even with an empty HOME, and the
  // server would then show the user's GitHub account. Stubs that fail first
  // on PATH keep every forge account out of the window.
  for (const cli of FORGE_CLIS) writeFileSync(join(paths.bin, cli), "#!/bin/sh\nexit 1\n", { mode: 0o755 })
  ensureProject(paths.project)
  ensureProject(paths.project2)
  return paths
}

const windowEnv = ({ paths, seed, fixturePort, uiPort, apiPort }) => isolatedEnv({
  HOME: paths.home,
  PATH: `${paths.bin}:${process.env.PATH ?? ""}`,
  XDG_CONFIG_HOME: join(paths.home, ".config"),
  XDG_DATA_HOME: join(paths.home, ".local/share"),
  XDG_CACHE_HOME: join(paths.home, ".cache"),
  XDG_STATE_HOME: join(paths.home, ".local/state"),
  OPENCHAMBER_DATA_DIR: paths.dataDir,
  OPENCHAMBER_DESKTOP_USER_DATA_DIR: paths.userData,
  OPENCHAMBER_ELECTRON_REMOTE_DEBUGGING_PORT: String(CDP_PORT),
  OPENCHAMBER_HMR_UI_PORT: String(uiPort),
  OPENCHAMBER_HMR_API_PORT: String(apiPort),
  OPENCODE_CONFIG_CONTENT: JSON.stringify({
    ...referenceProviderConfig(fixturePort),
    model: seed.model,
    small_model: "perf/stream-1200cps",
    autoupdate: false,
    share: "disabled",
  }),
})

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

/**
 * Stops the window by the pid of electron-dev.mjs, which tears down Electron
 * and the HMR servers itself; then its process group, then anything left
 * whose working directory is under ROOT (the OpenCode the server started runs
 * in its own process group with HOME as its working directory).
 */
const stopWindow = async (child) => {
  if (child.pid && isAlive(child.pid)) {
    process.kill(child.pid, "SIGTERM")
    for (let attempt = 0; attempt < 60 && isAlive(child.pid); attempt += 1) await wait(250)
  }
  try { process.kill(-child.pid, "SIGKILL") } catch { /* group already gone */ }
  let left = processesUnderRoot()
  for (const pid of left) { try { process.kill(pid, "SIGTERM") } catch { /* exited */ } }
  for (let attempt = 0; attempt < 20 && left.some(isAlive); attempt += 1) await wait(250)
  left = left.filter(isAlive)
  for (const pid of left) { try { process.kill(pid, "SIGKILL") } catch { /* exited */ } }
  return left
}

const startWindow = (env) => {
  const output = createWriteStream(join(ROOT, "electron-dev.log"))
  // electron:dev without its `extensions:build` step, which `bun install`
  // already ran: the pid is electron-dev.mjs itself, not a `bun run` shell.
  const child = spawn(process.execPath, ["./packages/electron/scripts/electron-dev.mjs"], {
    cwd: REPO_ROOT,
    env,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  })
  child.stdout.pipe(output)
  child.stderr.pipe(output)
  return child
}

const MOUNTED = `Boolean(document.querySelector("[data-left-sidebar]") && document.querySelector("[role=textbox][contenteditable=true]") && document.querySelector("[data-context-surface]"))`

/**
 * The page of the dev window once the app has mounted and stayed mounted on
 * the same target for STABLE_MS. A fresh Electron profile can relaunch the
 * app once shortly after its first start (electron-dev.mjs restarts it on
 * exit code 75), which replaces the page target.
 */
const connectPage = async (uiPort, child) => {
  const STABLE_MS = 15_000
  const deadline = Date.now() + 240_000
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`electron-dev.mjs exited with ${child.exitCode}; see ${join(ROOT, "electron-dev.log")}`)
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
  return JSON.parse(body)
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
  favoriteAgents: seed.favoriteAgents,
  recentAgents: seed.recentAgents,
  providerOrder: ["perf", "perf-alt"],
  collapsedModelProviders: ["perf-alt"],
  // The OpenCode update check runs a few seconds after mount and its toast
  // sits outside every region; a disabled check keeps the reference to the
  // window's own controls.
  showOpenCodeUpdateNotifications: false,
})

const createSession = async (origin, { title, directory }) => {
  const created = await api(origin, "/api/session", {
    method: "POST",
    body: JSON.stringify({ title, location: { directory } }),
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
  entry.sessions = Array.isArray(sessions) ? sessions : sessions.data ?? []
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
 */
const REGIONS = [
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
const REGION_ORDER = ["sidebar", "header", "chatColumn", "composer", "workStatusPanel", "iconRail", "popover"]

/**
 * Runs in the page in one task, so the regions and the control list come from
 * the same DOM. Leaves the control elements and the region roots on window
 * for the accessibility lookup.
 */
const collectInPage = `(() => {
  const CONTROL_SELECTOR = ${JSON.stringify(CONTROL_SELECTOR)}
  const REGIONS = ${JSON.stringify(REGIONS)}
  // Sessions kept alive in the background keep their own (hidden) chat
  // column and composer, so a region is the one rendered element that matches.
  const rendered = (element) => element.checkVisibility({ visibilityProperty: true }) && element.getBoundingClientRect().width > 0
  const exactlyOne = (selector) => {
    const found = [...document.querySelectorAll(selector)].filter(rendered)
    if (found.length !== 1) throw new Error(selector + " matched " + found.length + " rendered elements")
    return found[0]
  }
  const roots = {}
  for (const region of REGIONS) {
    if (region.popup) continue
    const found = exactlyOne(region.selector)
    roots[region.name] = [region.parent ? found.parentElement : found, region.also ? document.querySelector(region.also) : null].filter(Boolean)
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
  const elements = []
  for (const element of document.querySelectorAll(CONTROL_SELECTOR)) {
    if (element.closest("[inert]") || element.closest("[aria-hidden=true]")) continue
    if (!element.checkVisibility({ visibilityProperty: true })) continue
    const box = element.getBoundingClientRect()
    if (box.width === 0 || box.height === 0) continue
    elements.push(element)
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
    region: REGIONS.find((region) => roots[region.name].some((root) => root.contains(element)))?.name ?? null,
    shown: element.checkVisibility({ visibilityProperty: true, opacityProperty: true }),
    text: visibleText(element),
    label: labelOf(element),
    // Controls inside this one, in DOM order.
    nested: elements.flatMap((other, index) => other !== element && element.contains(other) ? [index] : []),
  }))
  window.__ocReferenceControls = elements
  window.__ocReferenceRoots = roots
  return {
    regions: Object.fromEntries(Object.entries(roots).flatMap(([name, [root]]) => root ? [[name, bounds(root)]] : [])),
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
  { rule: "duration", pattern: /\b\d+(?:\.\d+)?(?:ms|s|m|h)\b/g, replacement: "<duration>" },
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

const captureState = async (client, state, widths, titles) => {
  await restPointer(client)
  const collected = await evaluateValue(client, collectInPage)
  if (!collected) throw new Error(`Collecting ${state} failed in the page`)
  const shot = await client.send("Page.captureScreenshot", { format: "png" })
  const controls = await nameAndKey(client, collected, await accessibilityOf(client, collected.controls.length), titles)
  await client.send("Runtime.evaluate", { expression: "delete window.__ocReferenceControls; delete window.__ocReferenceRoots" })
  const entry = ({ role, name, key, shown }) => ({ role, name, key, shown })
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

/**
 * The streaming state's messages: the run's response, with the final text
 * part cut after the heading so its size does not depend on when the poll
 * landed. The other parts are complete once the heading appears.
 */
const streamingMessages = async (origin, id, directory) => {
  const deadline = Date.now() + 120_000
  while (Date.now() < deadline) {
    const messages = await fetchMessages(origin, id, directory)
    const part = messages.flatMap((message) => message.content ?? []).find((entry) => entry.type === "text"
      && entry.text?.includes("Executing a call"))
    if (part && part.text.length >= STREAMING_TEXT_CHARACTERS) {
      let cut = false
      return messages.map((message) => ({
        ...message,
        content: (message.content ?? []).map((entry) => {
          if (cut || entry !== part) return entry
          cut = true
          return { ...entry, text: entry.text.slice(0, STREAMING_TEXT_CHARACTERS) }
        }),
      }))
    }
    await wait(250)
  }
  throw new Error(`The streaming session never produced ${STREAMING_TEXT_CHARACTERS} characters of its document`)
}

const CONNECTED_STEP = { type: "connection", status: { state: "connected" } }
const FAILED_STEP = { type: "connection", status: { state: "failed", ...SERVER_ERROR } }

const stateOf = ({ steps, open, base }) => {
  const state = {
    version: 2,
    home: base.home,
    settings: base.settings,
    directories: structuredClone(base.directories),
    draft: base.draft,
    steps,
  }
  if (open) state.open = open
  return state
}

const ID_PATTERN = /\b(?:ses|msg|evt|prt)_[A-Za-z0-9]+/g
const TIME_KEYS = new Set(["created", "updated", "busy", "idle", "viewed", "archived", "completed", "streamed", "started", "ended", "ran", "openedAt", "doneAt", "suggestDoneAt", "endedAt"])
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
 * message, event and part ids to first-appearance ordinals, timestamps to
 * their rank in the run, durations to a fixed value, and the server's
 * pid-named instance folder to one name. Everything else is the response.
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

  const times = []
  const collectTimes = (node, key) => {
    const kind = jsonKind(node)
    if (kind === "array") { for (const item of node) collectTimes(item, key); return }
    if (kind === "object") {
      for (const [childKey, child] of Object.entries(node)) collectTimes(child, childKey)
      return
    }
    if (kind === "number" && TIME_KEYS.has(key)) times.push(node)
  }
  collectTimes(value, "")
  const uniqueTimes = [...new Set(times)].sort((left, right) => left - right)
  const time = new Map(uniqueTimes.map((entry, index) => [entry, FIXED_TIME + index * 1000]))

  const rewrite = (node, key) => {
    const kind = jsonKind(node)
    if (kind === "string") {
      return node
        .replace(/\/run\/instances\/\d+\//g, "/run/instances/reference/")
        .replace(ID_PATTERN, (match) => id(match))
    }
    if (kind === "array") return node.map((item) => rewrite(item, key))
    if (kind === "object") {
      return Object.fromEntries(Object.entries(node).map(([childKey, child]) => [childKey, rewrite(child, childKey)]))
    }
    if (kind === "number" && TIME_KEYS.has(key)) return time.get(node)
    if (kind === "number" && key === "durationMs") return FIXED_DURATION_MS
    return node
  }
  return rewrite(value, "")
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

const run = async ({ client, origin, paths, seed }) => {
  const captures = []
  const states = []
  await client.send("Emulation.setDeviceMetricsOverride", { ...VIEWPORT, deviceScaleFactor: 1, mobile: false })
  // The window may sit behind others; focus emulation keeps the composer
  // focusable and typing working.
  await client.send("Emulation.setFocusEmulationEnabled", { enabled: true })
  await client.send("Accessibility.enable")
  await client.send("Input.setIgnoreInputEvents", { ignore: true })
  // The layout settles at the new viewport.
  await wait(1500)

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
  save(await captureState(client, "empty", widths, titles))
  save(await captureState(client, "projects", widths, titles))

  for (const name of ["project-popover", "model-popover", "agent-menu"]) {
    save(await capturePopup(client, name, widths, titles), { open: name })
  }

  // Electron runs the server in its own process, so it has no shell in which
  // the sidecar is down: with every data route aborted, the React app replaces
  // the window with its connection screen and none of the regions exist. The
  // spike design therefore gives server-error a state fixture but no
  // reference (design.md "Parity reference", archived spike). The fixture
  // still carries the seeded setup and the fixed failure the Rust app shows.
  states.push({ name: "server-error", json: `${JSON.stringify(stabilize(stateOf({ steps: [FAILED_STEP], base })), null, 2)}\n` })

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
  await waitFor("the Work status telemetry", () => evaluateValue(client, `document.querySelector("[data-right-slot]")?.innerText.includes("Average TTFT")`), { timeoutMs: 30_000 })
  await refreshSessions(origin, directories)
  const finishedMessages = await fetchMessages(origin, ids.session, paths.project)
  const sessionSteps = [CONNECTED_STEP, { type: "openSession", session: ids.session }, { type: "messagesLoaded", session: ids.session, messages: finishedMessages }]
  save(await captureState(client, "session", await checkWidths(client), titles), { steps: sessionSteps })
  save(await capturePopup(client, "thinking-menu", await checkWidths(client), titles), { steps: sessionSteps, open: "thinking-menu" })

  const streaming = seed.sessions.find((session) => session.state === "streaming")
  await sendFromComposer(client, { origin, project: paths.project, id: ids.streaming, title: streaming.title, prompt: seed.prompt })
  const streamingWidths = await checkWidths(client)
  // Mid-stream at a fixed point: every tool call has run and the document's
  // first heading is on screen, before its first code block (which adds two
  // controls) arrives about two seconds later at 300 characters/s.
  const chat = renderedExpression("[data-workspace-chat]")
  await waitFor("the first heading of the streamed document", () => evaluateValue(client, `(() => {
    const chat = ${chat}
    // Other sessions' timelines stay in the column under display: none.
    const rendered = (selector) => chat ? [...chat.querySelectorAll(selector)].filter((element) => element.checkVisibility()) : []
    return rendered("h2").some((heading) => heading.textContent.includes("Executing a call")) && rendered("pre").length === 0
  })()`), { timeoutMs: 180_000, intervalMs: 50 })
  const capture = await captureState(client, "streaming", streamingWidths, titles)
  if (capture.chatCodeBlocks > 0) throw new Error(`A code block reached the chat column before the streaming capture (${capture.chatCodeBlocks}); rerun`)
  if (!isBusy(await sessionTime(origin, ids.streaming, paths.project))) throw new Error("The streaming turn had ended before the capture")
  await refreshSessions(origin, directories)
  const partialMessages = await streamingMessages(origin, ids.streaming, paths.project)
  save(capture, {
    steps: [CONNECTED_STEP, { type: "openSession", session: ids.streaming }, { type: "messagesLoaded", session: ids.streaming, messages: partialMessages }],
  })

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
  for (const [label, port] of [["CDP", CDP_PORT], ["fixture provider", FIXTURE_PORT]]) {
    if (await portInUse(port)) throw new Error(`Port ${port} (${label}) is already in use; stop that process first`)
  }
  const personalInRoot = personalStrings().filter((value) => ROOT.includes(value))
  if (personalInRoot.length > 0) throw new Error(`The temporary root ${ROOT} names the user; set TMPDIR to a neutral directory`)

  const paths = prepareRoot(seed)
  const fixture = await startFixture({ port: FIXTURE_PORT, logFile: join(ROOT, "fixture.log") })
  let child = null
  let client = null
  let completed = false
  let result = { captures: [], states: [] }
  try {
    const [uiPort, apiPort] = [await reservePort(), await reservePort()]
    child = startWindow(windowEnv({ paths, seed, fixturePort: FIXTURE_PORT, uiPort, apiPort }))
    log(`electron-dev.mjs pid ${child.pid}, UI port ${uiPort}, API port ${apiPort}, root ${ROOT}`)
    const page = await connectPage(uiPort, child)
    client = page.client
    result = await run({ client, origin: page.origin, paths, seed })
    completed = true
  } finally {
    client?.close()
    if (child) {
      const left = await stopWindow(child)
      if (left.length > 0) log(`warning: processes ${left.join(", ")} under ${ROOT} needed SIGKILL`)
    }
    await fixture.stop()
    // A failed run keeps its logs; the next run removes the root first.
    if (completed) rmSync(ROOT, { recursive: true, force: true })
    else log(`kept ${ROOT} with electron-dev.log and fixture.log`)
  }
  assertNothingPersonal([
    { name: "seed.json", json: JSON.stringify(seed) },
    ...result.captures.map(({ state, json }) => ({ name: `${state}.json`, json })),
    ...result.states.map(({ name, json }) => ({ name: `states/v2/${name}.json`, json })),
  ])
  mkdirSync(options.output, { recursive: true })
  for (const { state, json, png } of result.captures) {
    writeFileSync(join(options.output, `${state}.json`), json)
    writeFileSync(join(options.output, `${state}.png`), png)
  }
  mkdirSync(options.statesOutput, { recursive: true })
  for (const { name, json } of result.states) writeFileSync(join(options.statesOutput, `${name}.json`), json)
  log(`wrote ${result.captures.length} reference states to ${options.output} and ${result.states.length} state fixtures to ${options.statesOutput}; no JSON names ${personalStrings().length} personal strings (home, repo path, user name)`)
}

main().catch((error) => {
  console.error(`[capture-reference] failed: ${error.stack ?? error.message}`)
  process.exitCode = 1
})
