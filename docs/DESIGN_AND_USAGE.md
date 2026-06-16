# Agent REPL — Design & Usage

A token-driven system for rendering a coding agent's output stream — user
messages, assistant prose, reasoning, tool calls, diffs, shell output, search
results, file listings, todos, errors, and live status — in a terminal or a
React surface.

The whole look is recomposed from **four enums** chosen by the *harness
architect*. Nothing about an individual block is hard-coded to a color or size;
blocks read CSS custom properties that the enums set.

---

## 1. The four enums

| Enum | Values | What it controls |
|------|--------|------------------|
| `vibe` | `phosphor` · `slate` · `spectrum` · `ember` | Palette **and** color strategy. |
| `mode` | `dark` · `light` | Light/dark variant of the chosen vibe. |
| `toolStyle` | `inline` · `card` · `collapsed` | How tool calls are framed. |
| `density` | `comfortable` · `compact` | Font size, line-height, padding, gaps. |

### Vibes (each is also a *color strategy*)

- **phosphor** — Classic CRT terminal. All-monospace, minimal color (color is
  reserved for status: green/amber/red). Dark = green-on-black; light = dark
  green ink on warm paper.
- **slate** — Modern dev tool. Cool neutrals, a single indigo accent, restrained
  semantic color. Prose in a proportional sans; code in mono.
- **spectrum** — Rich & semantic. Every tool category gets its own vivid hue,
  with soft accent-tinted fills.
- **ember** — Warm & friendly. Amber accent, rounded corners, approachable.
  Dark = brown-black; light = cream paper.

`mode` swaps a vibe between its `dark` and `light` palette. Light palettes use
darker, slightly higher-chroma accents so tool hues, diff lines, and badges stay
legible on a light background.

### Tool styles

- **inline** — CLI-style log lines: a one-line header (colored sigil + tool name
  + target) and the body flush under it with a hue gutter. No box. Always shows
  the body.
- **card** — Each tool is a bordered, rounded card with a tinted header and an
  inset body. Always shows the body.
- **collapsed** — Same card chrome, but the body is hidden behind a one-line
  summary (e.g. `+12 −3`, `exit 0`, `5 matches`). Click the header to expand.

### Density

`comfortable` (readable, generous spacing) and `compact` (log-dense). Only
spacing and type scale change — colors are untouched.

---

## 2. Token system (CSS custom properties)

`buildVars(vibe, density, mode)` returns a style object of CSS variables. Apply
it to a container; every block inherits from there. Switching any enum just
re-runs `buildVars` and swaps the variables — no per-block logic.

**Color tokens**
```
--bg            --bg-raised      --bg-inset
--border        --border-strong
--text          --text-dim       --text-faint
--accent        --accent-soft
--success       --danger         --warning      --info
--t-read --t-edit --t-bash --t-search --t-list --t-todo --t-web   (tool hues)
```

**Type / shape tokens**
```
--font-mono   --font-sans   --font-prose   (prose = mono or sans per vibe)
--radius
--fs --fs-sm --fs-xs   --lh
--gap   --pad-y --pad-x   --block-pad-y   --head-gap
```

All colors are authored in `oklch()` so dark/light variants share hue and read
as the same family. Soft fills (badge backgrounds, diff line tints, alert
backgrounds) are derived at render time with `color-mix(in oklch, …)`, so they
adapt to the active background automatically.

---

## 3. The event model

A transcript is a flat array of events. Every event has a `type`; tool events
also have a `kind`.

```js
// Messages
{ type: 'user',      text }                         // markdown
{ type: 'assistant', text }                         // markdown
{ type: 'reasoning', text, ms, defaultOpen }        // collapsible thinking
{ type: 'status',    text }                         // streaming line + spinner
{ type: 'alert',     level: 'error'|'warning', title, detail }

// Tool calls — { type:'tool', kind, title, wait, ...payload }
//   `wait` (ms) is only used by the live-demo player to time the "running" phase.
{ kind: 'search', result: { count, groups: [ { file, hits: [ { line, text } ] } ] } }
{ kind: 'read',   path, lines, preview: [ { n, text } ] }
{ kind: 'list',   entries: [ { name, type: 'dir'|'file', meta } ] }
{ kind: 'edit',   diff: [ { t: 'add'|'del'|'ctx', a, b, text } ] }   // a=old line#, b=new line#
{ kind: 'write',  diff: [ … ] }
{ kind: 'bash',   cmd, output, exit }                                 // exit !== 0 → red badge
{ kind: 'todo',   items: [ { state: 'done'|'active'|'pending', text } ] }
{ kind: 'web',    … }                                                 // hue token: --t-web
```

Tool hue and label come from the `TOOLS` map (`repl/tokens.jsx`):
`read·write·edit·bash·search·list·todo·web → { label, hue }`.

---

## 4. Components

All are exposed on `window` (each `<script type="text/babel">` has its own
scope; they're shared via `window`).

| Component | Props | Notes |
|-----------|-------|-------|
| `EventBlock` | `{ ev, running, streaming }` | Dispatches any event to the right block. Start here. |
| `Markdown` | `{ children }` | Tiny dependency-free md → React (headings, bold/italic, `code`, fenced blocks, lists, links, quotes). |
| `UserMsg` / `AssistantMsg` | `{ text }` | Markdown messages. |
| `Reasoning` | `{ text, ms, defaultOpen }` | Collapsible thinking block. |
| `StatusLine` | `{ children }` | Spinner + dim text. |
| `Alert` | `{ level, title, children }` | Error / warning. |
| `ToolCall` | `{ ev, running }` | Renders header + body per `toolStyle`; dispatches body by `kind`. |
| `Badge` | `{ kind: 'ok'|'err'|'run'|'warn' }` | Status pill. |
| `Spinner` | — | Braille spinner. |
| `InputFrame` | `{ fieldNode, chips, slash, error, working, onSend, onStop }` | The composer shell: prompt sigil, field slot, send/stop, footer. Frame follows `toolStyle`. |
| `InputStates` | — | Gallery of all composer states (idle, typing, multiline, slash, chips, working, limit-warning). |
| `LiveComposer` | `{ working, onSubmit }` | Interactive composer: auto-grow textarea, `/`-triggered slash menu, ⏎ to submit, ⇧⏎ newline. |

Context: `ReplCtx` provides `{ vibe, mode, toolStyle, density, colors }`. Blocks
read `toolStyle` and `colors` from it (e.g. `ToolCall` looks up its hue via
`colors[TOOLS[kind].hue]`).

---

## 4b. The composer (input)

The input recomposes from the same enums. `vibe` / `mode` / `density` flow
through the shared tokens; **`toolStyle` shapes the frame** so the input matches
its output framing:

- **inline** — flush composer under a single hairline (matches CLI log lines).
- **card** / **collapsed** — a bordered, rounded composer box.

`InputFrame` is the presentational shell; you supply the editable `fieldNode`
(`LiveComposer` passes a real auto-growing `<textarea>`; the gallery passes a
static placeholder). Parts it renders:

- **prompt sigil** — `❯` (a spinner when `working`).
- **context chips** (`chips`) — `@file` mentions + attachments + "add context".
- **slash menu** (`slash = { query, sel, placement }`) — `/clear /compact
  /model /diff /undo /run`, placed above (`top`) or below (`below`) the field.
- **alert strip** (`error`) — e.g. an approaching-context-limit warning.
- **actions** — a **send** button, or a red **stop** button when `working`.
- **footer** — model · cwd · branch on the left; `⏎ send · ⇧⏎ newline · /
  cmds · @ files` on the right. Swaps to "Working… · esc to interrupt" when busy.

Wire it to your loop by passing `working` (true while the agent runs) and an
`onSubmit(text)` handler. In the demo, the docked `LiveComposer` appends the
typed message to the stream and flips `working` during the scripted run.

---

## 5. File structure

```
Agent REPL.html        Shell: fonts, the full stylesheet, script tags, mounts <App/>
repl/
  tokens.jsx           VIBES (dark+light), DENSITIES, TOOLS, buildVars, paletteFor, ReplCtx
  markdown.jsx         <Markdown>
  blocks.jsx           All output-block renderers + <EventBlock>
  data.jsx             TRANSCRIPT (demo run) + GALLERY (one of each block)
  input.jsx            Composer: <InputFrame>, <InputStates>, <LiveComposer>
  app.jsx              Toolbar (the four enums + tabs), Gallery, LiveDemo, <App>
```

Load order matters: React → ReactDOM → Babel → `tokens` → `markdown` → `blocks`
→ `data` → `input` → `app`.

---

## 6. Using it

### As-is (HTML)
Open `Agent REPL.html`. Use the toolbar to pick vibe / mode / tool style /
density and switch between the **Live demo** (a scripted streaming run) and the
**Gallery** (every block type at once).

### Embedding the renderer in your own React app
1. Apply tokens to a container:
   ```jsx
   const vars = buildVars(vibe, density, mode);
   const colors = paletteFor(vibe, mode);
   <ReplCtx.Provider value={{ vibe, mode, toolStyle, density, colors }}>
     <div className="app" style={vars} data-toolstyle={toolStyle}>
       <div className="stream">
         {events.map((ev, i) => <EventBlock key={i} ev={ev} />)}
       </div>
     </div>
   </ReplCtx.Provider>
   ```
2. Ship the stylesheet from `Agent REPL.html` (the `<style>` block). It is the
   structural CSS; it references only the token variables, so it works for every
   enum combination.
3. Feed it events in the shapes above as your agent emits them. Set
   `running` on the in-flight tool to show the spinner phase.
4. Dock the composer with `<LiveComposer working={isAgentBusy} onSubmit={send} />`
   (or build your own from `<InputFrame>`); set `data-mode` on the same root for
   light/dark.

The app exposes three tabs over this: **Live demo** (scripted run + docked
composer), **Input** (`<InputStates>` \u2014 the composer gallery), and **Gallery**
(every output block).

### Porting the *strategy* to a real terminal (ANSI)
You don't need this React layer to reuse the design. Map the same tokens to
ANSI:
- Tool hue → 256-color / truecolor SGR for the tool name + gutter.
- `--text-dim` / `--text-faint` → dim (SGR 2) for metadata, line numbers,
  summaries.
- Diff: green (`add`), red (`del`), default-dim (`ctx`); keep the `+ / −`
  signs and right-aligned line numbers.
- Badges → bracketed tags: `[done]` green, `[exit 1]` red, `[running]` with the
  braille spinner.
- `inline` tool style maps directly to terminal log lines; `collapsed` maps to
  a one-line summary you expand on demand in a TUI.

The single source of truth for every palette is the `VIBES` table in
`repl/tokens.jsx` — edit there and both modes update.

---

## 7. Extending

- **New tool kind:** add it to `TOOLS` (label + hue token), add a body renderer
  in `blocks.jsx`, and a `case` in `ToolBody` + `toolSummary`.
- **New vibe:** add an entry to `VIBES` with `dark` and `light` palettes (copy
  the token keys from an existing one). It appears in the toolbar automatically.
- **New block type:** add a renderer and a `case` in `EventBlock`.
