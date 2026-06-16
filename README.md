# agent-repl-kit

A token-driven terminal REPL UI for coding agents. Drop it into a Rust
project, emit events, and get a polished interactive interface with
themeable colors, tool blocks, message blocks, and an input composer —
without writing any ratatui yourself.

The whole look recomposes from four enums:

| Enum         | Values                                          |
| ------------ | ----------------------------------------------- |
| `Vibe`       | `Phosphor` · `Slate` · `Spectrum` · `Ember`     |
| `Mode`       | `Dark` · `Light`                                |
| `ToolStyle`  | `Inline` · `Card` · `Collapsed`                 |
| `Density`    | `Comfortable` · `Compact`                       |

That's 48 combinations off a single `Theme` builder. Switching vibes /
modes at runtime is just a key press.

The design comes from a React/HTML reference under
[`docs/`](docs/DESIGN_AND_USAGE.md); this crate is a Rust port using
[ratatui](https://docs.rs/ratatui) +
[crossterm](https://docs.rs/crossterm). A future
[`agent-repl` npm package](#future) mirrors the same event + theme
model in React Ink.

## Crates

- **[`agent-repl-core`](crates/agent-repl-core)** — pure data: event
  model, theme tokens, the four enums, palette conversion (oklch →
  sRGB). No terminal dependency. Reusable from any renderer.
- **[`agent-repl`](crates/agent-repl)** — ratatui renderer + the
  `AgentRepl` app, `ReplHandle` channel API, blocks, and the
  `LiveComposer`.

## Quickstart

```rust
use agent_repl::{AgentRepl, Event, Theme, ToolCall, ToolKind};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let theme = Theme::slate().dark().card().comfortable();
    let (app, handle) = AgentRepl::new(theme);
    let app = app
        .with_assistant_sigil("●")     // optional sigil before "assistant"
        .with_user_sigil("●")
        .with_model("claude-opus-4-7")
        .with_cwd("~/project")
        .with_branch("main")
        .with_file_completions(vec![
            "src/main.rs".into(),
            "src/lib.rs".into(),
            "Cargo.toml".into(),
        ]);

    tokio::spawn(async move {
        handle.set_working(true);

        // emit a streaming run
        handle.emit(Event::assistant("On it."));
        let bash = handle.start_tool(ToolCall::new(
            "cargo test",
            ToolKind::Bash {
                cmd: "cargo test".into(),
                output: String::new(),
                exit: None,
            },
        ));
        // ... actually run the command ...
        handle.finish_tool(bash, ToolCall::new(
            "cargo test",
            ToolKind::Bash {
                cmd: "cargo test".into(),
                output: "test result: ok. 47 passed".into(),
                exit: Some(0),
            },
        ));

        // wait for the next user message
        while let Some(line) = handle.recv_input().await {
            handle.set_working(true);
            handle.emit(Event::assistant(format!("got it: {line}")));
        }
    });

    app.run().await
}
```

## Demo

```bash
cargo run -p agent-repl --example demo
```

Plays a scripted transcript (the same one used in the JSX reference),
then lets you talk to an echo agent through the composer. Try:

- **F1 / F2 / F3 / F4** — cycle vibe, mode, tool style, density.
- **F5 / F6** — switch between live transcript and the block-type
  Gallery.
- **Tab / Shift-Tab** — focus next / previous tool block (visible in
  collapsed style).
- **Ctrl-E** — toggle the focused tool's expansion.
- **PgUp / PgDn** — scroll the stream.
- **Type `/`** — slash menu (commands).
- **Type `@`** — file menu (`@file` completion).
- **Shift-Enter** — newline (composer goes multi-line up to 10 rows).
- **Esc** — clear the composer, then quit.

## Event model

A transcript is a flat sequence of `Event`s emitted through
`ReplHandle`. Shapes match `docs/DESIGN_AND_USAGE.md` §3:

```rust
Event::User { text }
Event::Assistant { text }
Event::Reasoning { text, ms, default_open }
Event::Status { text }
Event::Alert { level, title, detail }
Event::Tool(ToolCall {
    title, running, run_label,
    kind: ToolKind::{
        Search | Read | List | Edit | Write | Bash | Todo | Web,
    },
})
```

Each tool kind has its own body renderer (diff for edit/write, ANSI
output for bash, hits grouped by file for search, etc.) and its own
hue token from the active palette.

## Composer

A fixed input at the bottom of the screen. Frame style follows the
active `ToolStyle` (hairlines for `Inline`; bordered box for `Card` /
`Collapsed`). Features:

- Multi-line buffer; auto-grows up to 10 rows then scrolls within.
- Slash menu popup (`/clear`, `/compact`, `/model`, `/diff`, `/undo`,
  `/run`).
- `@file` menu popup wired to a caller-supplied completion list.
- Accepted `@tokens` render as colored chips inside the field.
- Footer: model pill, cwd, branch on the left; key hints on the right;
  swaps to `Working… esc interrupt` while the agent is busy.

## Status

| Slice | Scope                                                        | State |
| ----- | ------------------------------------------------------------ | ----- |
| 1     | `agent-repl-core` — events + themes + palettes               | done  |
| 2     | `agent-repl` renderer + demo                                 | done  |
| 3     | Collapsed-style focus/expand + Gallery view                  | done  |
| 4     | LiveComposer (multi-line, slash menu, @file menu, chips)     | done  |

Test coverage: 66 tests (35 composer + 25 render + 5 core + 1 doctest).

## Future

A React Ink package (`agent-repl` on npm) that mirrors this crate's
event + theme model is planned; the data layer in `agent-repl-core`
is intentionally I/O-free so the TS port can reuse the same shapes.

## License

MIT — see [LICENSE](LICENSE).
