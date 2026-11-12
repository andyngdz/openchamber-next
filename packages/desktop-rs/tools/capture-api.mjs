#!/usr/bin/env node
/**
 * Captures the raw HTTP and SSE traffic the Rust desktop spike's client has to
 * decode, from a real OpenChamber server started the way the sidecar starts it:
 * `node packages/web/bin/cli.js serve --foreground --quiet --api-only ...`.
 *
 * Everything runs against the deterministic fixture provider with an isolated
 * HOME, OPENCHAMBER_DATA_DIR and OpenCode config, so no token is spent and the
 * user's own OpenChamber is never touched. The OpenCode config asks before every
 * `glob` call; the sessions of the non-permission scenarios allow it themselves.
 *
 * The run lives in `<os tmpdir>/oc-desktop-rs-capture` (project, HOME and data dirs), which
 * is removed at the end. Output is held in memory and written only after a check that no
 * file contains the home directory, the repository root or the user name as a path segment.
 *
 * Writes `fixtures/api/` (one JSON body per call, `<name>.request.json` for a
 * request body, `index.json`) and `fixtures/events/` (one JSON array of parsed
 * `data:` payloads per scenario, `index.json`, `framing.json`, `summary.json`).
 *
 * The turn-control scenarios at the end answer permissions for the fixture tool
 * models, seed a location-scoped form through the server's own form route
 * (OpenCode raises one itself only from a location-level MCP elicitation), queue
 * messages behind a running turn, and read the control stream. They write
 * `fixtures/events/turn-control.json` and `control.json`.
 *
 * Usage: node packages/desktop-rs/tools/capture-api.mjs [--out <dir>] [--root <dir>]
 */

import { spawn } from "node:child_process"
import { createWriteStream, existsSync, mkdirSync, readdirSync, realpathSync, rmSync, writeFileSync } from "node:fs"
import { createServer } from "node:net"
import { homedir, tmpdir, userInfo } from "node:os"
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
const importPerf = (name) => import(pathToFileURL(join(REPO_ROOT, "scripts/perf", name)).href)
const { ensureProject, isolatedEnv, startFixture } = await importPerf("isolated-server.mjs")
const { FIXTURE_EDIT_CONTENT, FIXTURE_EDIT_FILE, FIXTURE_TOOL_RATE, fixtureProviderConfig } = await importPerf("fixture-provider.mjs")

const PROVIDER = "perf"
// The same fixture provider is registered twice so the model picker has two
// provider sections. Both ids share one base URL; the fixture answers any model
// name, so the second id only changes the catalog the app sees.
const SECOND_PROVIDER = "perf-second"
// Fixed so the base URL carried in the config, provider and model fixtures is
// the same in every run. The server keeps its random free port: it never
// appears in a captured body.
const FIXTURE_PORT = 4611
const TURN_TIMEOUT_MS = 180_000
const SETTLE_MS = 1_500
// The Chat draft in `packages/ui/src/lib/chatDirectories.ts` names its folder
// `<chatsRoot>/<local date>/session-<uuid v4>`; the fixture pins both parts so
// the mkdir path is identical between runs.
const CHAT_FOLDER_DATE = "2026-01-02"
const CHAT_SESSION_UUID = "00000000-0000-4000-8000-000000000000"
// Fixed timestamps for seeded settings and metadata, so only ids and the
// server's own clocks vary between runs.
const FIXTURE_EPOCH = 1_800_000_000_000

const parseArgs = (argv) => {
  const options = { out: join(PACKAGE_ROOT, "fixtures"), root: join(tmpdir(), "oc-desktop-rs-capture") }
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index]
    if (value === "--out") options.out = resolve(argv[++index])
    else if (value === "--root") options.root = resolve(argv[++index])
    else throw new Error(`Unknown option: ${value}`)
  }
  return options
}

/**
 * The OpenCode config the capture's server runs with. OpenCode 2.x reads the
 * native `providers` shape (`package`, `settings`), not the 1.x
 * `provider`/`npm`/`options` one, so the fixture provider's model list is
 * rebuilt from `fixtureProviderConfig` in the spelling this OpenCode accepts.
 *
 * Every model except `stream-600cps` and `alt-thinking` clears its variants:
 * the OpenAI-compatible package gives a model the `low`/`medium`/`high`
 * reasoning levels by default, and those two are the only ones the pickers
 * should show a Thinking menu for.
 */
const captureProviderConfig = (fixturePort) => {
  const models = Object.fromEntries(Object.entries(fixtureProviderConfig(fixturePort).provider[PROVIDER].models)
    .map(([id, model]) => [id, { ...model, variants: [] }]))
  models["stream-600cps"] = { ...models["stream-600cps"], variants: [{ id: "low" }, { id: "high" }] }
  const options = { baseURL: `http://127.0.0.1:${fixturePort}/v1`, apiKey: "unused" }
  return {
    providers: {
      [PROVIDER]: {
        package: "aisdk:@ai-sdk/openai-compatible",
        name: "Performance fixture",
        settings: options,
        models,
      },
      [SECOND_PROVIDER]: {
        package: "aisdk:@ai-sdk/openai-compatible",
        name: "Performance fixture, second id",
        settings: options,
        models: {
          "alt-chat-300cps": { name: "Second fixture chat, 300 characters/s", variants: [] },
          "alt-code-1200cps": { name: "Second fixture code block, 1200 characters/s", variants: [] },
          "alt-thinking": { name: "Second fixture with variants", variants: [{ id: "low" }, { id: "medium" }, { id: "high" }] },
        },
      },
    },
    agents: {
      "capture-primary": { description: "Primary fixture agent", mode: "primary", model: { providerID: PROVIDER, model: "stream-300cps" } },
      "capture-secondary": { description: "Second primary fixture agent", mode: "primary" },
      "capture-hidden": { description: "Hidden fixture agent", mode: "primary", hidden: true },
      "capture-subagent": { description: "Fixture subagent", mode: "subagent" },
    },
    model: `${PROVIDER}/stream-300cps`,
    default_agent: "capture-secondary",
    small_model: `${PROVIDER}/stream-1200cps`,
    autoupdate: false,
    share: "disabled",
    // The turn-control scenarios need the request bodies of these tools, not
    // a decision the session rule already made.
    permission: { glob: "ask", bash: "ask", edit: "ask", webfetch: "ask" },
  }
}

const projectId = (path) => `path_${Buffer.from(path).toString("base64url")}`

/**
 * The shared settings document the server starts on. React reads the same
 * keys, so the fixture pins the shapes its pickers and sidebar decode: two
 * projects (one with a colour and icon), the active project, favourites,
 * recents, `lastUsedEfforts`, the hidden model and the sidebar sections.
 */
const seedSettings = ({ project, secondProject }) => {
  const firstId = projectId(project)
  return {
    projects: [
      {
        id: firstId,
        path: project,
        label: "Fixture project",
        color: "#4F46E5",
        icon: "rocket",
        iconBackground: "#E0E7FF",
        addedAt: FIXTURE_EPOCH - 2_000,
        lastOpenedAt: FIXTURE_EPOCH - 1_000,
        sidebarCollapsed: false,
      },
      {
        id: projectId(secondProject),
        path: secondProject,
        label: "Second fixture project",
        addedAt: FIXTURE_EPOCH - 4_000,
        lastOpenedAt: FIXTURE_EPOCH - 3_000,
        sidebarCollapsed: true,
      },
    ],
    activeProjectId: firstId,
    defaultModel: `${PROVIDER}/stream-600cps`,
    defaultVariant: "high",
    defaultAgent: "capture-primary",
    lastSelectedModel: `${PROVIDER}/stream-1200cps`,
    favoriteModels: [
      { providerID: PROVIDER, modelID: "stream-600cps" },
      { providerID: SECOND_PROVIDER, modelID: "alt-thinking" },
    ],
    recentModels: [
      { providerID: SECOND_PROVIDER, modelID: "alt-chat-300cps" },
      { providerID: PROVIDER, modelID: "stream-300cps" },
    ],
    hiddenModels: [{ providerID: PROVIDER, modelID: "stream-100cps" }],
    favoriteAgents: ["capture-primary"],
    recentAgents: ["capture-secondary", "capture-primary"],
    lastUsedEfforts: {
      [`${PROVIDER}/stream-600cps`]: "low",
      [`${SECOND_PROVIDER}/alt-thinking`]: null,
    },
    enterToSend: true,
    enterToSendConfigured: true,
    sidebarProjectSortOrder: "a-z",
    sidebarShowChatsSection: true,
    sidebarShowWorkSection: true,
    sidebarShowPinnedSection: true,
    sidebarSections: [
      { kind: "builtin", id: "chats" },
      { kind: "builtin", id: "pinned" },
      { kind: "builtin", id: "work" },
      { kind: "builtin", id: "projects" },
    ],
  }
}

const sleep = (ms) => new Promise((done) => setTimeout(done, ms))

const freePort = () => new Promise((resolvePort, reject) => {
  const server = createServer()
  server.once("error", reject)
  server.listen(0, "127.0.0.1", () => {
    const { port } = server.address()
    server.close(() => resolvePort(port))
  })
})

/** Fails instead of reusing a process this run did not start. */
const assertPortFree = (port) => new Promise((resolvePort, reject) => {
  const server = createServer()
  server.once("error", () => reject(new Error(`Port ${port} is already in use; stop that process and run the capture again.`)))
  server.listen(port, "127.0.0.1", () => server.close(() => resolvePort()))
})

const alive = (pid) => {
  try { process.kill(pid, 0); return true } catch { return false }
}

// OpenCode's id format, as `packages/ui/src/lib/opencode/ids.ts` mints message ids.
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

const pretty = (value) => `${JSON.stringify(value, null, 2)}\n`

// Every fixture is held in memory until the whole run has passed the path check,
// so a failing run writes nothing and leaves the previous fixtures alone.
const outputs = new Map()
const emit = (file, content) => outputs.set(file, content)

/**
 * Fixtures are committed, so none may carry the machine's home directory, this
 * checkout's path or the user name as a path segment. A hit aborts the run.
 */
const assertNoPersonalPaths = () => {
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

// ---------------------------------------------------------------------------
// The server under capture
// ---------------------------------------------------------------------------

/** Starts the CLI as the sidecar does, resolves with its base URL once the port line arrives. */
const startServer = ({ root, project, fixturePort, settings }) => new Promise((resolveServer, reject) => {
  freePort().then((port) => {
    const state = join(root, "state")
    mkdirSync(join(state, "home"), { recursive: true })
    mkdirSync(join(state, "oc-data"), { recursive: true })
    // Seeded before the server reads it, so the first settings GET already
    // carries the projects, favourites and recents the fixture exists for.
    writeFileSync(join(state, "oc-data", "settings.json"), pretty(settings))
    const config = captureProviderConfig(fixturePort)
    const env = isolatedEnv({
      HOME: join(state, "home"),
      OPENCHAMBER_DATA_DIR: join(state, "oc-data"),
      OPENCODE_CONFIG_CONTENT: JSON.stringify(config),
    })
    const child = spawn(process.execPath, [
      join(REPO_ROOT, "packages/web/bin/cli.js"), "serve", "--foreground", "--quiet", "--api-only", "--host", "127.0.0.1", "--port", String(port),
    ], { cwd: project, env, detached: true, stdio: ["ignore", "pipe", "pipe"] })
    const log = createWriteStream(join(root, "server.stderr.log"))
    child.stderr.pipe(log)
    let first = ""
    const timer = setTimeout(() => { stop().finally(() => reject(new Error("The server printed no port line within 60 s."))) }, 60_000)
    child.stdout.on("data", (chunk) => {
      first += chunk
      const line = first.split("\n")[0]
      if (!first.includes("\n")) return
      clearTimeout(timer)
      child.stdout.removeAllListeners("data")
      if (line.trim() !== String(port)) { stop().finally(() => reject(new Error(`Unexpected first stdout line: ${JSON.stringify(line)}`))); return }
      resolveServer({ url: `http://127.0.0.1:${port}`, port, pid: child.pid, stop })
    })
    child.once("exit", (code) => {
      clearTimeout(timer)
      reject(new Error(`The server exited early with code ${code}; see ${join(root, "server.stderr.log")}`))
    })
    // The CLI's own SIGTERM handler stops its OpenCode. OpenCode runs in a group of its
    // own, so the pids are read first and any survivor is stopped by pid afterwards.
    async function stop() {
      if (child.exitCode !== null) return
      const opencode = await childPids(child.pid)
      try { process.kill(child.pid, "SIGTERM") } catch { /* already gone */ }
      for (let attempt = 0; attempt < 40 && alive(child.pid); attempt += 1) await sleep(250)
      if (alive(child.pid)) { try { process.kill(child.pid, "SIGKILL") } catch { /* gone */ } }
      for (const pid of opencode) {
        for (let attempt = 0; attempt < 20 && alive(pid); attempt += 1) await sleep(250)
        if (alive(pid)) { try { process.kill(pid, "SIGTERM") } catch { /* gone */ } }
      }
    }
  }, reject)
})

const childPids = (pid) => new Promise((resolvePids) => {
  const pgrep = spawn("pgrep", ["-P", String(pid)], { stdio: ["ignore", "pipe", "ignore"] })
  let out = ""
  pgrep.stdout.on("data", (chunk) => { out += chunk })
  pgrep.on("close", () => resolvePids(out.split("\n").map(Number).filter(Boolean)))
  pgrep.on("error", () => resolvePids([]))
})

// ---------------------------------------------------------------------------
// HTTP capture
// ---------------------------------------------------------------------------

const createRecorder = ({ url, directory, apiDir }) => {
  const header = encodeURIComponent(directory)
  const index = {}

  /** One request; the raw body goes to `<name>.json`, a request body to `<name>.request.json`. */
  const call = async (name, method, path, { body, directoryHeader = true, save = true } = {}) => {
    const headers = {}
    if (directoryHeader) headers["x-opencode-directory"] = header
    if (body !== undefined) headers["content-type"] = "application/json"
    const response = await fetch(`${url}${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) })
    const text = await response.text()
    const contentType = response.headers.get("content-type") ?? ""
    let json
    try { json = text ? JSON.parse(text) : undefined } catch { json = undefined }
    if (save && name) {
      if (body !== undefined) emit(join(apiDir, `${name}.request.json`), pretty(body))
      if (json !== undefined) emit(join(apiDir, `${name}.json`), pretty(json))
      index[name] = { method, path, status: response.status, contentType, requestBody: body !== undefined, responseBody: json !== undefined }
    }
    return { status: response.status, contentType, text, json }
  }
  return { call, index, writeIndex: () => emit(join(apiDir, "index.json"), pretty(index)) }
}

// ---------------------------------------------------------------------------
// SSE capture
// ---------------------------------------------------------------------------

/** One SSE connection that keeps every frame in arrival order. */
const openEventTap = async ({ url, directory, path = "/api/event", directoryHeader = true }) => {
  const controller = new AbortController()
  const headers = { accept: "text/event-stream" }
  if (directoryHeader) headers["x-opencode-directory"] = encodeURIComponent(directory)
  const response = await fetch(`${url}${path}`, {
    headers,
    signal: controller.signal,
  })
  const tap = {
    status: response.status,
    contentType: response.headers.get("content-type") ?? "",
    frames: [],
    events: [],
    waiters: [],
  }
  const decoder = new TextDecoder()
  let buffer = ""
  const flush = () => {
    for (const waiter of [...tap.waiters]) waiter.check()
  }
  void (async () => {
    try {
      for await (const chunk of response.body) {
        buffer += decoder.decode(chunk, { stream: true }).replaceAll("\r\n", "\n")
        let end
        while ((end = buffer.indexOf("\n\n")) !== -1) {
          const raw = buffer.slice(0, end)
          buffer = buffer.slice(end + 2)
          tap.frames.push(raw)
          const data = raw.split("\n").filter((line) => line.startsWith("data:")).map((line) => line.slice(5).replace(/^ /, "")).join("\n")
          if (data) {
            try { tap.events.push(JSON.parse(data)) } catch { tap.events.push({ unparsed: data }) }
          }
        }
        flush()
      }
    } catch { /* aborted */ }
  })()
  /** Resolves with the first event after `from` that satisfies `test`. */
  tap.waitFor = (test, { from = 0, timeoutMs = TURN_TIMEOUT_MS, label = "event" } = {}) => new Promise((resolveWait, reject) => {
    const waiter = {
      check: () => {
        const hit = tap.events.slice(from).find(test)
        if (!hit) return
        clearTimeout(timer)
        tap.waiters.splice(tap.waiters.indexOf(waiter), 1)
        resolveWait(hit)
      },
    }
    const timer = setTimeout(() => {
      tap.waiters.splice(tap.waiters.indexOf(waiter), 1)
      reject(new Error(`Timed out waiting for ${label}; saw ${tap.events.length - from} events (last types: ${tap.events.slice(-5).map((event) => event.type).join(", ")})`))
    }, timeoutMs)
    tap.waiters.push(waiter)
    waiter.check()
  })
  tap.close = () => controller.abort()
  return tap
}

/** The control stream: the same `data:` framing, no directory scoping. */
const openControlTap = ({ url }) => openEventTap({ url, path: "/api/openchamber/events", directoryHeader: false })

const sessionOf = (event) => event?.data?.sessionID
const TURN_END = new Set(["session.execution.succeeded", "session.execution.interrupted", "session.execution.failed"])
const isTurnEnd = (sessionID) => (event) => TURN_END.has(event.type) && sessionOf(event) === sessionID

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

const main = async () => {
  const options = parseArgs(process.argv.slice(2))
  const { root } = options
  const apiDir = join(options.out, "api")
  const eventsDir = join(options.out, "events")
  rmSync(root, { recursive: true, force: true })
  mkdirSync(root, { recursive: true })
  const projectPath = join(root, "project")
  const secondProjectPath = join(root, "project-two")
  ensureProject(projectPath)
  ensureProject(secondProjectPath)
  // Settings store real paths (the server resolves them), so the capture uses
  // the same identity for every session and for the seeded projects.
  const project = realpathSync(projectPath)
  const secondProject = realpathSync(secondProjectPath)
  const settings = seedSettings({ project, secondProject })

  await assertPortFree(FIXTURE_PORT)
  const fixture = await startFixture({ port: FIXTURE_PORT, logFile: join(root, "fixture.log") })
  let server
  const shutdown = async () => {
    await server?.stop()
    await fixture.stop()
    rmSync(root, { recursive: true, force: true })
  }
  process.once("SIGINT", () => { shutdown().finally(() => process.exit(130)) })
  process.once("SIGTERM", () => { shutdown().finally(() => process.exit(143)) })

  try {
    server = await startServer({ root, project, fixturePort: FIXTURE_PORT, settings })
    console.log(`Server ${server.url} (pid ${server.pid}), fixture provider on ${FIXTURE_PORT}, project ${project}`)
    const recorder = createRecorder({ url: server.url, directory: project, apiDir })
    const { call } = recorder
    const eventIndex = {}
    const framing = {}

    /** Waits for the location's plugins, which catalog routes 503 on until then. */
    const waitForCatalogs = async () => {
      const deadline = Date.now() + 60_000
      let status = 0
      while (Date.now() < deadline) {
        const response = await fetch(`${server.url}/api/agent`, { headers: { "x-opencode-directory": encodeURIComponent(project) } })
        status = response.status
        await response.body?.cancel().catch(() => undefined)
        if (response.ok) return
        await sleep(500)
      }
      throw new Error(`The catalog for ${project} did not become ready in 60 s (last status ${status}).`)
    }

    const saveEvents = (name, tap, meta) => {
      emit(join(eventsDir, `${name}.json`), pretty(tap.events))
      eventIndex[name] = { count: tap.events.length, ...meta }
      framing[name] = { status: tap.status, contentType: tap.contentType, frameCount: tap.frames.length, firstFrames: tap.frames.slice(0, 3), lastFrame: tap.frames.at(-1) }
    }

    // 2.2: every route the client needs answers under --api-only.
    const routeChecks = []
    const expectRoute = (label, result, { allow } = {}) => {
      const answered = result.status !== 404 && !result.contentType.includes("text/html")
      routeChecks.push({ route: label, status: result.status, contentType: result.contentType, answered })
      if (!answered && !allow) throw new Error(`Route ${label} does not answer under --api-only: ${result.status} ${result.contentType}`)
    }

    const auth = await call("auth-status", "GET", "/auth/session", { directoryHeader: false })
    expectRoute("GET /auth/session", auth)
    if (auth.json?.authenticated !== true) throw new Error(`Auth status is not authenticated: ${auth.text}`)

    // Home and settings: the two reads startup needs before it can pick a
    // project or a model. Both ignore the directory header.
    const home = await call("fs-home", "GET", "/api/fs/home", { directoryHeader: false })
    expectRoute("GET /api/fs/home", home)
    if (!home.json?.home || !home.json?.chatsRoot) throw new Error(`fs/home answered without home or chatsRoot: ${home.text}`)

    const settingsRead = await call("settings", "GET", "/api/config/settings?surface=desktop")
    expectRoute("GET /api/config/settings", settingsRead)
    if (settingsRead.json?.projects?.length !== 2) throw new Error(`Seeded settings did not reach the server: ${settingsRead.text.slice(0, 400)}`)

    // A partial write, as React sends: the response is the whole document.
    const settingsWrite = await call("settings-update", "PUT", "/api/config/settings?surface=desktop", {
      body: {
        activeProjectId: projectId(secondProject),
        recentModels: [
          { providerID: PROVIDER, modelID: "stream-600cps" },
          { providerID: SECOND_PROVIDER, modelID: "alt-chat-300cps" },
        ],
        lastSelectedModel: `${PROVIDER}/stream-600cps`,
        lastUsedEfforts: { [`${PROVIDER}/stream-600cps`]: "high", [`${SECOND_PROVIDER}/alt-thinking`]: null },
      },
    })
    expectRoute("PUT /api/config/settings", settingsWrite)
    if (settingsWrite.json?.activeProjectId !== projectId(secondProject)) throw new Error(`Settings PUT did not keep the write: ${settingsWrite.text.slice(0, 400)}`)

    // Catalogs wait for the location's plugins; asking too early answers 503.
    await waitForCatalogs()
    expectRoute("GET /api/config", await call("config", "GET", "/api/config"))
    expectRoute("GET /api/provider", await call("providers", "GET", "/api/provider"))
    expectRoute("GET /api/model", await call("models", "GET", "/api/model?types=chat&types=image"))
    expectRoute("GET /api/agent", await call("agents", "GET", "/api/agent"))

    const emptyList = await call("session-list-empty", "GET", "/api/session")
    expectRoute("GET /api/session", emptyList)

    // The list the UI asks for scopes by directory in the query as well as the header.
    await call("session-list-empty-query", "GET", `/api/session?limit=50&directory=${encodeURIComponent(project)}`)

    const createSession = async (name, { allowGlob, directory = project, agent, model } = {}) => {
      const body = { location: { directory } }
      if (allowGlob) body.permissions = [{ action: "glob", resource: "*", effect: "allow" }]
      if (agent) body.agent = agent
      if (model) body.model = model
      const created = await call(name, "POST", "/api/session", { body })
      if (created.status !== 200 || !created.json) throw new Error(`Creating a session failed: ${created.status} ${created.text}`)
      return created.json
    }
    const idOf = (session) => session.id ?? session.data?.id ?? session.sessionID

    const switchModel = (name, sessionID, modelId) => call(name, "POST", `/api/session/${sessionID}/model`, { body: { model: { providerID: PROVIDER, id: modelId } } })
    const sendPrompt = (name, sessionID, text) => call(name, "POST", `/api/session/${sessionID}/prompt`, { body: { id: messageId(), text } })

    // Scenario A: a plain text reply. The create call is saved as the "create session" fixture.
    const sessionA = await createSession("session-create")
    const sessionAId = idOf(sessionA)
    if (!sessionAId) throw new Error(`Cannot find the id in the created session: ${JSON.stringify(sessionA)}`)
    await call("session-list-nonempty", "GET", "/api/session")

    // Route checks for message list, interrupt (nothing running) and event stream before any turn.
    expectRoute("GET /api/session/:id/message", await call("messages-empty", "GET", `/api/session/${sessionAId}/message`))
    expectRoute("POST /api/session/:id/interrupt", await call("interrupt-idle", "POST", `/api/session/${sessionAId}/interrupt`))

    const textTap = await openEventTap({ url: server.url, directory: project })
    expectRoute("GET /api/event", { status: textTap.status, contentType: textTap.contentType })
    await sleep(500)
    const textFrom = textTap.events.length
    const textPrompt = await sendPrompt("prompt-text", sessionAId, "Reply with the fixture text.")
    expectRoute("POST /api/session/:id/prompt", textPrompt)
    await textTap.waitFor(isTurnEnd(sessionAId), { from: textFrom, label: "end of the text turn" })
    await sleep(SETTLE_MS)
    textTap.close()
    saveEvents("text-reply-stream-300cps", textTap, { model: "stream-300cps", sessions: [sessionAId], note: "plain text reply" })
    await call("messages-text-turn", "GET", `/api/session/${sessionAId}/message`)
    await call("session-list-after-turn", "GET", "/api/session")

    // Scenario B: an agent turn with tool calls. The session allows `glob` itself.
    const agentSession = idOf(await createSession("session-create-allow-glob", { allowGlob: true }))
    await switchModel("switch-model", agentSession, "agent-20tools-300cps")
    const agentTap = await openEventTap({ url: server.url, directory: project })
    await sleep(500)
    await sendPrompt("prompt-agent", agentSession, "List the files.")
    await agentTap.waitFor(isTurnEnd(agentSession), { label: "end of the agent turn" })
    await sleep(SETTLE_MS)
    agentTap.close()
    saveEvents("agent-20tools-300cps", agentTap, { model: "agent-20tools-300cps", sessions: [agentSession], note: "agent turn, 20 glob calls, then text; glob allowed by a session rule" })
    await call("messages-agent-turn", "GET", `/api/session/${agentSession}/message`)

    // Scenario C: a turn interrupted through POST /api/session/:id/interrupt.
    const interruptSession = idOf(await createSession("session-create-interrupt", { allowGlob: true }))
    await switchModel("switch-model-interrupt", interruptSession, "agent-20tools-300cps")
    const interruptTap = await openEventTap({ url: server.url, directory: project })
    await sleep(500)
    await sendPrompt("prompt-interrupt", interruptSession, "List the files, then I stop you.")
    await interruptTap.waitFor((event) => event.type === "session.tool.success" && sessionOf(event) === interruptSession, { label: "the first finished tool call" })
    const interrupted = await call("interrupt-running", "POST", `/api/session/${interruptSession}/interrupt`)
    await interruptTap.waitFor(isTurnEnd(interruptSession), { label: "end of the interrupted turn" })
    await sleep(SETTLE_MS)
    interruptTap.close()
    saveEvents("interrupted-agent-turn", interruptTap, { model: "agent-20tools-300cps", sessions: [interruptSession], note: `interrupted after the first tool result; interrupt answered ${interrupted.status}` })
    await call("messages-interrupted-turn", "GET", `/api/session/${interruptSession}/message`)

    // Scenario D: a `glob` call asks for permission and the script answers each request.
    const permissionSession = idOf(await createSession("session-create-ask-glob"))
    await switchModel("switch-model-permission", permissionSession, "agent-20tools-300cps")
    const permissionTap = await openEventTap({ url: server.url, directory: project })
    await sleep(500)
    await sendPrompt("prompt-permission", permissionSession, "List the files, ask first.")
    let repliedTo = 0
    let replies = 0
    while (true) {
      const next = await permissionTap.waitFor(
        (event) => (event.type === "permission.asked" && sessionOf(event) === permissionSession) || isTurnEnd(permissionSession)(event),
        { from: repliedTo, label: "a permission request or the end of the turn" },
      )
      repliedTo = permissionTap.events.indexOf(next, repliedTo) + 1
      if (next.type !== "permission.asked") break
      // `permission.asked` names the request `data.id`; `permission.replied` calls it `requestID`.
      const requestID = next.data.id
      // The first reply is the fixture; later requests (a new glob pattern) only keep the turn going.
      const name = replies === 0 ? "permission-reply" : null
      const reply = await call(name, "POST", `/api/session/${permissionSession}/permission/${requestID}/reply`, { body: { decision: "always" } })
      if (reply.status >= 300) throw new Error(`Permission reply failed: ${reply.status} ${reply.text}`)
      replies += 1
    }
    await sleep(SETTLE_MS)
    permissionTap.close()
    const asked = permissionTap.events.filter((event) => event.type === "permission.asked").length
    if (asked === 0) throw new Error("No permission.asked event arrived: the glob permission rule did not apply.")
    saveEvents("permission-asked-agent-turn", permissionTap, { model: "agent-20tools-300cps", sessions: [permissionSession], note: `glob asks; ${asked} request(s) answered with decision "always"` })
    await call("messages-permission-turn", "GET", `/api/session/${permissionSession}/message`)

    // Scenario E: two sessions of the directory prompted together, one stream.
    const sessionB1 = idOf(await createSession("session-create-two-a"))
    const sessionB2 = idOf(await createSession("session-create-two-b"))
    const twoTap = await openEventTap({ url: server.url, directory: project })
    await sleep(500)
    await Promise.all([
      sendPrompt("prompt-two-a", sessionB1, "First session."),
      sendPrompt("prompt-two-b", sessionB2, "Second session."),
    ])
    await Promise.all([
      twoTap.waitFor(isTurnEnd(sessionB1), { label: "end of the first session's turn" }),
      twoTap.waitFor(isTurnEnd(sessionB2), { label: "end of the second session's turn" }),
    ])
    await sleep(SETTLE_MS)
    twoTap.close()
    saveEvents("two-sessions-stream-300cps", twoTap, { model: "stream-300cps", sessions: [sessionB1, sessionB2], note: "two sessions of the same directory in one stream" })

    // Scenario F: a session created and deleted while the stream is open.
    const lifecycleTap = await openEventTap({ url: server.url, directory: project })
    await sleep(500)
    const lifecycleSession = idOf(await createSession("session-create-lifecycle"))
    await lifecycleTap.waitFor((event) => event.type === "session.created" && sessionOf(event) === lifecycleSession, { label: "session.created" })
    const removed = await call("session-delete", "DELETE", `/api/session/${lifecycleSession}`)
    await lifecycleTap.waitFor((event) => event.type === "session.deleted" && sessionOf(event) === lifecycleSession, { label: "session.deleted" })
    await sleep(SETTLE_MS)
    lifecycleTap.close()
    saveEvents("session-lifecycle", lifecycleTap, { model: null, sessions: [lifecycleSession], note: `session created, then deleted (delete answered ${removed.status})` })

    // The picker's session: created with an agent and a model, once with a
    // variant and once without, matching `POST /api/session` as the composer
    // sends it (`packages/ui/src/lib/opencode/client.ts:1146-1165`).
    const variantSession = idOf(await createSession("session-create-agent-model", {
      agent: "capture-primary",
      model: { providerID: SECOND_PROVIDER, id: "alt-thinking", variant: "medium" },
    }))
    await createSession("session-create-agent-model-default", {
      agent: "capture-secondary",
      model: { providerID: PROVIDER, id: "stream-300cps" },
    })
    const sessionRecord = await call("session-get", "GET", `/api/session/${variantSession}`)
    expectRoute("GET /api/session/:id", sessionRecord)
    if (sessionRecord.json?.data?.model?.variant !== "medium") throw new Error(`The created session lost its variant: ${sessionRecord.text.slice(0, 400)}`)

    // Switching an open session: the variant alone is a change, and Default
    // sends a model without `variant` (`resolveSendSelection`).
    expectRoute("POST /api/session/:id/model", await call("switch-model-variant", "POST", `/api/session/${variantSession}/model`, {
      body: { model: { providerID: PROVIDER, id: "stream-600cps", variant: "high" } },
    }))
    expectRoute("POST /api/session/:id/model", await call("switch-model-default", "POST", `/api/session/${variantSession}/model`, {
      body: { model: { providerID: PROVIDER, id: "stream-300cps" } },
    }))
    expectRoute("POST /api/session/:id/agent", await call("switch-agent", "POST", `/api/session/${variantSession}/agent`, {
      body: { agent: "build" },
    }))

    // The four session kinds the sidebar split must tell apart. They are
    // created after the scenario sessions so the newest four are these, and 25
    // ms apart so the list's `time.updated` order cannot tie.
    await sleep(25)
    const archivedSession = idOf(await createSession(null))
    expectRoute("POST /api/openchamber/sessions/archive", await call("seed-archive", "POST", "/api/openchamber/sessions/archive", {
      body: { directory: project, ids: [archivedSession], archivedAt: FIXTURE_EPOCH },
    }))
    await sleep(25)
    // `/btw` marker: `packages/ui/src/lib/sessionBtwMetadata.ts:55-57` reads
    // `openchamber.kind === "btw"` plus its `originalSessionID`.
    const btwSession = idOf(await createSession(null))
    expectRoute("POST /api/openchamber/sessions/:id/metadata", await call("seed-metadata-btw", "POST", `/api/openchamber/sessions/${btwSession}/metadata`, {
      body: { patch: { openchamber: { kind: "btw", originalSessionID: sessionAId, btwBoundaryMessageID: null } } },
    }))
    await sleep(25)
    // Pinned: `openchamber.pinned.at`, as `withSessionPinned` writes it
    // (`packages/ui/src/lib/sessionPins.ts:47-60`).
    const pinnedSession = idOf(await createSession(null))
    expectRoute("POST /api/openchamber/sessions/:id/metadata", await call("seed-metadata-pinned", "POST", `/api/openchamber/sessions/${pinnedSession}/metadata`, {
      body: { patch: { openchamber: { pinned: { at: FIXTURE_EPOCH } } } },
    }))
    await sleep(25)

    // A Chat session under the server's chats root. `mkdir` then create is
    // what `createChatDirectory` does (`packages/ui/src/lib/chatDirectories.ts:109-118`).
    const chatDirectory = join(home.json.chatsRoot, CHAT_FOLDER_DATE, `session-${CHAT_SESSION_UUID}`)
    expectRoute("POST /api/fs/mkdir", await call("fs-mkdir", "POST", "/api/fs/mkdir", { body: { path: chatDirectory } }))
    const chatSession = idOf(await createSession(null, { directory: chatDirectory }))

    // Global pages: no directory header and no query, the way
    // `packages/ui/src/stores/globalSessions.ts:51-120` walks `cursor.next`.
    const globalFirst = await call("session-list-global-page-1", "GET", "/api/session?limit=2", { directoryHeader: false })
    expectRoute("GET /api/session (global)", globalFirst)
    const nextCursor = globalFirst.json?.cursor?.next
    if (!nextCursor) throw new Error(`The first global page has no cursor.next: ${globalFirst.text.slice(0, 400)}`)
    const globalSecond = await call("session-list-global-page-2", "GET", `/api/session?limit=2&cursor=${encodeURIComponent(nextCursor)}`, { directoryHeader: false })
    expectRoute("GET /api/session?cursor (global)", globalSecond)
    const firstTwoPages = [...(globalFirst.json?.data ?? []), ...(globalSecond.json?.data ?? [])]
    for (const [label, id] of [["archived", archivedSession], ["btw", btwSession], ["pinned", pinnedSession], ["chat", chatSession]]) {
      if (!firstTwoPages.some((session) => session.id === id)) throw new Error(`The ${label} session is not on the first two global pages.`)
    }
    if (!firstTwoPages.find((session) => session.id === archivedSession)?.time?.archived) throw new Error("The archived session carries no time.archived.")

    // -------------------------------------------------------------------------
    // Scenario G: turn control. Permissions, forms, the queue, auto-accept and
    // the control stream, every one through the route design.md's client table
    // names. The control stream is opened once for the whole scenario, so its
    // ready frame, heartbeats and broadcasts arrive in the order a client sees.
    // -------------------------------------------------------------------------
    const controlTap = await openControlTap({ url: server.url })
    expectRoute("GET /api/openchamber/events", { status: controlTap.status, contentType: controlTap.contentType })
    const turnTap = await openEventTap({ url: server.url, directory: project })
    await sleep(500)
    const turnEnd = (sessionID, from, label) => turnTap.waitFor(isTurnEnd(sessionID), { from, label })
    const waitPermission = (sessionID, from, label) => turnTap.waitFor(
      (event) => event.type === "permission.asked" && sessionOf(event) === sessionID,
      { from, label },
    )
    const replyPermission = async (name, sessionID, requestID, decision) => {
      const reply = await call(name, "POST", `/api/session/${sessionID}/permission/${requestID}/reply`, { body: { decision } })
      if (reply.status >= 300) throw new Error(`${name ?? "permission reply"} failed: ${reply.status} ${reply.text}`)
      return reply
    }

    // A bash request and a question form are pending at the same time, so both
    // list routes are saved with a live request and a live form.
    const bashModel = `bash-${FIXTURE_TOOL_RATE}cps`
    const questionModel = `question-${FIXTURE_TOOL_RATE}cps`
    const questionSession = idOf(await createSession("session-create-question"))
    await switchModel("switch-model-question", questionSession, questionModel)
    const bashSession = idOf(await createSession("session-create-bash-ask"))
    await switchModel("switch-model-bash", bashSession, bashModel)

    const formFrom = turnTap.events.length
    await sendPrompt("prompt-question", questionSession, "Ask the fixture questions.")
    const formCreated = await turnTap.waitFor((event) => event.type === "form.created", { from: formFrom, label: "form.created for the question turn" })
    const questionFormID = formCreated.data.form.id
    if (formCreated.data.form.fields?.length !== 3) throw new Error(`The question form has ${formCreated.data.form.fields?.length} fields, not three.`)

    // The location-scoped form has no session. OpenCode's form route accepts
    // the `global` sentinel, which is how a server-side MCP elicitation lands
    // in the same list.
    const globalForm = {
      id: "frm_fixture_global",
      title: "Fixture global form",
      metadata: { kind: "fixture.global" },
      fields: [{ key: "note", type: "string", title: "Fixture note", description: "A location-scoped form", required: false }],
    }
    const globalFormSeed = await call("form-create-global", "POST", "/api/session/global/form", { body: globalForm })
    expectRoute("POST /api/session/global/form", globalFormSeed)
    if (globalFormSeed.status !== 200) throw new Error(`Seeding the global form failed: ${globalFormSeed.status} ${globalFormSeed.text}`)
    if (globalFormSeed.json?.data?.sessionID !== "global") throw new Error(`The seeded form is not location-scoped: ${globalFormSeed.text.slice(0, 400)}`)
    const globalFormID = globalFormSeed.json.data.id

    const bashFrom = turnTap.events.length
    await sendPrompt("prompt-bash", bashSession, "Run the fixture command.")
    const bashAsked = await waitPermission(bashSession, bashFrom, "the bash permission request")
    const bashRequestID = bashAsked.data.id

    const permissionList = await call("permission-request-list", "GET", `/api/permission/request?location[directory]=${encodeURIComponent(project)}`)
    expectRoute("GET /api/permission/request", permissionList)
    if (!(permissionList.json?.data ?? []).some((request) => request.id === bashRequestID)) {
      throw new Error(`The pending permission list does not carry ${bashRequestID}: ${permissionList.text.slice(0, 400)}`)
    }
    const formList = await call("form-list", "GET", `/api/form?location[directory]=${encodeURIComponent(project)}`)
    expectRoute("GET /api/form", formList)
    for (const formID of [questionFormID, globalFormID]) {
      if (!(formList.json?.data ?? []).some((form) => form.id === formID)) {
        throw new Error(`The pending form list does not carry ${formID}: ${formList.text.slice(0, 400)}`)
      }
    }

    // Bash: allow once. The other two decisions are answered further down.
    const bashReplied = turnTap.events.length
    await replyPermission("permission-reply-once", bashSession, bashRequestID, "once")
    await turnEnd(bashSession, bashReplied, "the end of the bash turn")
    await call("messages-bash-turn", "GET", `/api/session/${bashSession}/message`)

    // The global form is cancelled; the question form is answered with one
    // answer per field, a list for the multiselect and text for the free field.
    const globallyCancelled = await call("form-cancel", "DELETE", `/api/session/global/form/${globalFormID}`)
    if (globallyCancelled.status !== 204) throw new Error(`Cancel of the global form failed: ${globallyCancelled.status} ${globallyCancelled.text}`)
    const questionReplied = turnTap.events.length
    const formReply = await call("form-reply", "POST", `/api/session/${questionSession}/form/${questionFormID}/reply`, {
      body: { answer: { q0: "First option", q1: ["Alpha detail", "Beta detail"], q2: "A free text fixture answer." } },
    })
    if (formReply.status !== 204) throw new Error(`The form reply failed: ${formReply.status} ${formReply.text}`)
    await turnEnd(questionSession, questionReplied, "the end of the question turn")
    await call("messages-question-turn", "GET", `/api/session/${questionSession}/message`)

    // A missing reply or cancel is the client's "already settled" answer.
    const permissionReply404 = await call("permission-reply-404", "POST", `/api/session/${bashSession}/permission/per_fixture_missing/reply`, { body: { decision: "once" } })
    if (permissionReply404.status !== 404) throw new Error(`A missing permission reply answered ${permissionReply404.status}, not 404.`)
    const formReply404 = await call("form-reply-404", "POST", `/api/session/${questionSession}/form/frm_fixture_missing/reply`, { body: { answer: {} } })
    if (formReply404.status !== 404) throw new Error(`A missing form reply answered ${formReply404.status}, not 404: ${formReply404.text}`)
    const formCancel404 = await call("form-cancel-404", "DELETE", `/api/session/${questionSession}/form/frm_fixture_missing`)
    if (formCancel404.status !== 404) throw new Error(`A missing form cancel answered ${formCancel404.status}, not 404: ${formCancel404.text}`)

    // Edit: denied. The fixed patch targets the file seeded above.
    writeFileSync(join(project, FIXTURE_EDIT_FILE), FIXTURE_EDIT_CONTENT)
    const editSession = idOf(await createSession("session-create-edit-ask"))
    await switchModel("switch-model-edit", editSession, `edit-${FIXTURE_TOOL_RATE}cps`)
    const editFrom = turnTap.events.length
    await sendPrompt("prompt-edit", editSession, "Edit the fixture file.")
    const editAsked = await waitPermission(editSession, editFrom, "the edit permission request")
    const editReplied = turnTap.events.length
    await replyPermission("permission-reply-reject", editSession, editAsked.data.id, "reject")
    await turnEnd(editSession, editReplied, "the end of the edit turn")
    await call("messages-edit-turn", "GET", `/api/session/${editSession}/message`)

    // Webfetch: allowed always. The URL is the fixture provider itself, which
    // answers the page with a 404, so the capture needs no network.
    const webfetchSession = idOf(await createSession("session-create-webfetch-ask"))
    await switchModel("switch-model-webfetch", webfetchSession, `webfetch-${FIXTURE_TOOL_RATE}cps`)
    const webfetchFrom = turnTap.events.length
    await sendPrompt("prompt-webfetch", webfetchSession, "Fetch the fixture page.")
    const webfetchAsked = await waitPermission(webfetchSession, webfetchFrom, "the webfetch permission request")
    const webfetchReplied = turnTap.events.length
    await replyPermission("permission-reply-always", webfetchSession, webfetchAsked.data.id, "always")
    await turnEnd(webfetchSession, webfetchReplied, "the end of the webfetch turn")
    await call("messages-webfetch-turn", "GET", `/api/session/${webfetchSession}/message`)

    // Write: the write tool shares the edit permission action, so the config
    // asks for this one too. The model exists for the states that show a
    // write preview.
    const writeSession = idOf(await createSession("session-create-write"))
    await switchModel("switch-model-write", writeSession, `write-${FIXTURE_TOOL_RATE}cps`)
    const writeFrom = turnTap.events.length
    await sendPrompt("prompt-write", writeSession, "Write the fixture file.")
    const writeOutcome = await turnTap.waitFor(
      (event) => (event.type === "permission.asked" && sessionOf(event) === writeSession) || isTurnEnd(writeSession)(event),
      { from: writeFrom, label: "the write turn or a permission request" },
    )
    if (writeOutcome.type === "permission.asked") {
      const writeReplied = turnTap.events.length
      await replyPermission("permission-reply-write", writeSession, writeOutcome.data.id, "once")
      await turnEnd(writeSession, writeReplied, "the end of the write turn")
    }
    await call("messages-write-turn", "GET", `/api/session/${writeSession}/message`)

    // A question whose turn is interrupted: the form is pending when the
    // interrupt arrives, and the question tool part ends in an error.
    const interruptedQuestionSession = idOf(await createSession("session-create-question-interrupt"))
    await switchModel("switch-model-question-interrupt", interruptedQuestionSession, questionModel)
    const interruptedQuestionFrom = turnTap.events.length
    await sendPrompt("prompt-question-interrupt", interruptedQuestionSession, "Ask a question, then I stop you.")
    await turnTap.waitFor((event) => event.type === "form.created", { from: interruptedQuestionFrom, label: "the interrupted question form" })
    const questionInterrupted = await call("interrupt-question-turn", "POST", `/api/session/${interruptedQuestionSession}/interrupt`)
    if (questionInterrupted.json?.interrupted !== true) throw new Error(`The question turn was not interrupted: ${questionInterrupted.text}`)
    await turnEnd(interruptedQuestionSession, interruptedQuestionFrom, "the end of the interrupted question turn")
    await call("messages-question-interrupted-turn", "GET", `/api/session/${interruptedQuestionSession}/message`)

    // The queue: messages wait behind a running turn, then the server sends
    // them itself. The send config is the composer's model, agent and variant.
    const queueSession = idOf(await createSession("session-create-queue", { model: { providerID: PROVIDER, id: "stream-100cps" } }))
    const queueStarted = turnTap.events.length
    await sendPrompt("prompt-queue-busy", queueSession, "Start the fixture turn the queue waits behind.")
    await turnTap.waitFor(
      (event) => event.type === "session.execution.started" && sessionOf(event) === queueSession,
      { from: queueStarted, label: "the queue session's turn start" },
    )
    // A plain prompt with steering delivery while the session is busy.
    await call("prompt-steer", "POST", `/api/session/${queueSession}/prompt`, {
      body: { id: messageId(), text: "Steer this into the running turn.", delivery: "steer" },
    })

    const queueItem = (text) => ({
      content: text,
      text,
      sendConfig: { providerID: PROVIDER, modelID: "stream-1200cps", agent: "capture-primary" },
    })
    const enqueue = (name, item) => call(name, "POST", `/api/message-queue/sessions/${queueSession}/items`, {
      body: { directory: project, item },
    })
    const first = await enqueue("message-queue-enqueue-1", queueItem("First queued fixture message."))
    const second = await enqueue("message-queue-enqueue-2", queueItem("Second queued fixture message."))
    const third = await enqueue("message-queue-enqueue-3", queueItem("Third queued fixture message."))
    for (const [name, result] of [["first", first], ["second", second], ["third", third]]) {
      if (result.status !== 200 || !result.json?.itemId) throw new Error(`Enqueueing the ${name} item failed: ${result.status} ${result.text}`)
    }
    await call("message-queue-list", "GET", "/api/message-queue")
    const reorder = await call("message-queue-reorder", "PUT", `/api/message-queue/sessions/${queueSession}/order`, {
      body: { itemIds: [third.json.itemId, first.json.itemId, second.json.itemId] },
    })
    if (reorder.status !== 200) throw new Error(`The queue reorder failed: ${reorder.status} ${reorder.text}`)
    const takeOne = await call("message-queue-take-one", "POST", `/api/message-queue/sessions/${queueSession}/items/${second.json.itemId}/take`)
    if (takeOne.status !== 200) throw new Error(`Taking one queued item failed: ${takeOne.status} ${takeOne.text}`)
    const takeAll = await call("message-queue-take-all", "POST", `/api/message-queue/sessions/${queueSession}/take`)
    if (takeAll.status !== 200) throw new Error(`Taking the queue failed: ${takeAll.status} ${takeAll.text}`)
    const fourth = await enqueue("message-queue-enqueue-4", queueItem("Fourth queued fixture message."))
    if (fourth.status !== 200) throw new Error(`Enqueueing the fourth item failed: ${fourth.status} ${fourth.text}`)
    const removedItem = await call("message-queue-remove", "DELETE", `/api/message-queue/sessions/${queueSession}/items/${fourth.json.itemId}`)
    if (removedItem.status !== 200) throw new Error(`Removing a queued item failed: ${removedItem.status} ${removedItem.text}`)

    // A remove while the item is being sent answers 409. The item carries a
    // context part, which makes the send admit a synthetic message first and
    // widens the window a client can catch; that item is not saved.
    const inFlightItem = {
      ...queueItem("Queued message that is in flight."),
      context: [{ kind: "synthetic", text: "Fixture context for the in-flight message." }],
    }
    const waitForSending = async (itemId, timeoutMs = 20_000) => {
      const deadline = Date.now() + timeoutMs
      while (Date.now() < deadline) {
        const snapshot = await call(null, "GET", "/api/message-queue")
        const session = (snapshot.json?.sessions ?? []).find((entry) => entry.sessionId === queueSession)
        if (session?.sendingId === itemId) return true
        if (!session) return false
        await sleep(1)
      }
      return false
    }
    const queueInterrupted = await call("interrupt-queue-turn", "POST", `/api/session/${queueSession}/interrupt`)
    if (queueInterrupted.json?.interrupted !== true) throw new Error(`The queue turn was not interrupted: ${queueInterrupted.text}`)
    let inFlight = await call(null, "POST", `/api/message-queue/sessions/${queueSession}/items`, {
      body: { directory: project, item: inFlightItem },
    })
    let inFlightStatus = 0
    for (let attempt = 1; attempt <= 3 && inFlightStatus !== 409; attempt += 1) {
      if (attempt > 1) {
        await turnTap.waitFor(isTurnEnd(queueSession), { from: turnTap.events.length, timeoutMs: 60_000, label: "the queued turn before the next attempt" }).catch(() => undefined)
        inFlight = await call(null, "POST", `/api/message-queue/sessions/${queueSession}/items`, {
          body: { directory: project, item: inFlightItem },
        })
      }
      if (!inFlight.json?.itemId) break
      if (!(await waitForSending(inFlight.json.itemId))) continue
      const conflict = await call(attempt === 1 ? "message-queue-remove-in-flight" : null, "DELETE", `/api/message-queue/sessions/${queueSession}/items/${inFlight.json.itemId}`)
      inFlightStatus = conflict.status
    }
    if (inFlightStatus !== 409) throw new Error(`The queue never answered 409 for an item in flight (last status ${inFlightStatus}).`)
    // The item the 409 kept is sent once the remove failed; let that turn end.
    await turnTap.waitFor(isTurnEnd(queueSession), { from: turnTap.events.length, timeoutMs: 60_000, label: "the queued turn after the in-flight reply" }).catch(() => undefined)

    // Auto-accept: the snapshot, then a session with an explicit safety mode.
    // The safety net has no routing provider here, so the bash request is held
    // and the server reports it left for the user on the control stream.
    const autoAccept = await call("permission-auto-accept", "GET", "/api/permission-auto-accept", { directoryHeader: false })
    expectRoute("GET /api/permission-auto-accept", autoAccept)
    const safetySession = idOf(await createSession("session-create-safety"))
    await switchModel("switch-model-safety", safetySession, bashModel)
    const safetyUpdate = await call("permission-auto-accept-set", "PUT", `/api/permission-auto-accept/sessions/${safetySession}`, {
      body: { mode: "safety", enabled: true, directory: project },
    })
    expectRoute("PUT /api/permission-auto-accept/sessions/:id", safetyUpdate)
    if (safetyUpdate.json?.modes?.[safetySession] !== "safety") throw new Error(`The safety mode did not stick: ${safetyUpdate.text.slice(0, 400)}`)
    const safetyFrom = turnTap.events.length
    await sendPrompt("prompt-safety", safetySession, "Run the fixture command under the safety net.")
    const safetyAsked = await waitPermission(safetySession, safetyFrom, "the safety permission request")
    const leftForUser = await controlTap.waitFor(
      (event) => event.type === "openchamber:permission-auto-accept.left-for-user" && event.properties?.permissionId === safetyAsked.data.id,
      { label: "the safety net leaving the request for the user" },
    )
    if (leftForUser.properties?.sessionId !== safetySession) throw new Error(`left-for-user named ${leftForUser.properties?.sessionId}, not the safety session.`)
    const safetyReplied = turnTap.events.length
    await replyPermission("permission-reply-safety", safetySession, safetyAsked.data.id, "once")
    await turnEnd(safetySession, safetyReplied, "the end of the safety turn")
    await call("messages-safety-turn", "GET", `/api/session/${safetySession}/message`)

    // Routing is an optional server feature: a 404 is the "unavailable" answer.
    const routing = await call("routing", "GET", "/api/routing", { directoryHeader: false })
    expectRoute("GET /api/routing", routing, { allow: true })

    // A heartbeat arrives every 25 seconds; keep the control stream open until
    // one is in the file, so the saved frames show the keep-alive.
    if (!controlTap.events.some((event) => event.type === "openchamber:heartbeat")) {
      await controlTap.waitFor((event) => event.type === "openchamber:heartbeat", { label: "the control heartbeat", timeoutMs: 35_000 })
    }
    await sleep(SETTLE_MS)
    controlTap.close()
    turnTap.close()
    saveEvents("turn-control", turnTap, {
      model: null,
      sessions: [bashSession, questionSession, editSession, webfetchSession, writeSession, interruptedQuestionSession, queueSession, safetySession],
      note: "permission requests and replies, the question and global forms, the interrupted question, a steer prompt, the message queue, and the safety-net turn",
    })
    saveEvents("control", controlTap, {
      model: null,
      sessions: [bashSession, questionSession, editSession, webfetchSession, writeSession, interruptedQuestionSession, queueSession, safetySession],
      note: "the control stream: ready, a heartbeat, queue and auto-accept updates, and a request left for the user",
    })

    // Summary of every event type seen: count and the keys of `data`.
    const summary = {}
    for (const [name] of Object.entries(eventIndex)) {
      const events = JSON.parse(outputs.get(join(eventsDir, `${name}.json`)))
      const byType = {}
      for (const event of events) {
        const entry = (byType[event.type] ??= { count: 0, envelopeKeys: new Set(), dataKeys: new Set() })
        entry.count += 1
        for (const key of Object.keys(event)) entry.envelopeKeys.add(key)
        for (const key of Object.keys(event.data ?? {})) entry.dataKeys.add(key)
      }
      summary[name] = Object.fromEntries(Object.entries(byType).sort(([a], [b]) => a.localeCompare(b)).map(([type, entry]) => [
        type, { count: entry.count, envelopeKeys: [...entry.envelopeKeys].sort(), dataKeys: [...entry.dataKeys].sort() },
      ]))
    }
    emit(join(eventsDir, "index.json"), pretty(eventIndex))
    emit(join(eventsDir, "framing.json"), pretty(framing))
    emit(join(eventsDir, "summary.json"), pretty(summary))
    recorder.writeIndex()
    emit(join(apiDir, "routes.json"), pretty(routeChecks))
    assertNoPersonalPaths()
    for (const dir of [apiDir, eventsDir]) {
      // Only the captured JSON goes; the READMEs beside it are written by hand.
      mkdirSync(dir, { recursive: true })
      for (const file of readdirSync(dir)) if (file.endsWith(".json")) rmSync(join(dir, file))
    }
    for (const [file, content] of outputs) writeFileSync(file, content)
    console.log(`Routes: ${routeChecks.map((check) => `${check.route} ${check.status}`).join("; ")}`)
    console.log(`Events: ${Object.entries(eventIndex).map(([name, meta]) => `${name} ${meta.count}`).join("; ")}`)
  } finally {
    await shutdown()
  }
}

main().then(() => process.exit(0), (error) => {
  console.error(`capture-api failed: ${error.message}`)
  process.exit(1)
})
