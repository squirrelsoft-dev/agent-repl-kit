//! Scripted-transcript demo. Mirrors `docs/repl/data.jsx` TRANSCRIPT so the
//! Rust REPL can be visually compared to the React/HTML reference.
//!
//! Keys: `v` cycle vibe · `m` toggle mode · `t` cycle tool style ·
//!       `d` toggle density · `↑↓ PgUp/PgDn g G` scroll · `q` / `Esc` quit.

use std::time::Duration;

use agent_repl::{
    AgentRepl, ApprovalChoice, ApprovalPrompt, DiffKind, DiffLine, EntryType, Event, ListEntry,
    ReadLine, ReplHandle, SearchGroup, SearchHit, SearchResult, Theme, TodoItem, TodoState,
    ToolCall, ToolKind,
};
use anyhow::Result;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    let theme = Theme::slate().dark().card().comfortable();
    let (app, handle) = AgentRepl::new(theme);

    // Optional — give assistant + user blocks their own sigil to match the
    // tool blocks' `●`. Swap for an emoji if you like (`"🤖"`, `"🍄"`, `"🦄"`).
    let app = app
        .with_assistant_sigil("●")
        .with_user_sigil("●")
        .with_model("agent-repl-demo")
        .with_cwd("~/agent-repl-kit")
        .with_branch("main")
        .with_file_completions(vec![
            "src/lib.rs".into(),
            "src/app.rs".into(),
            "src/composer/state.rs".into(),
            "src/composer/render.rs".into(),
            "src/blocks/tool.rs".into(),
            "src/blocks/msg.rs".into(),
            "Cargo.toml".into(),
            "README.md".into(),
            "docs/DESIGN_AND_USAGE.md".into(),
            "docs/repl/data.jsx".into(),
            "docs/repl/input.jsx".into(),
        ]);

    tokio::spawn(async move {
        // Mark working while the scripted transcript plays.
        handle.set_working(true);
        run_transcript(&handle).await;
        // Now flip to interactive mode: echo whatever the user types until
        // they quit. Each message is gated behind the permissions box so the
        // Yes / Always / No prompt is easy to see.
        loop {
            let Some(line) = handle.recv_input().await else { break };

            handle.request_approval(ApprovalPrompt::new(
                format!("run: {line}"),
                Some("the agent wants to act on this message".into()),
                Some("messages this session".into()),
            ));
            let approved = tokio::select! {
                choice = handle.recv_approval() => {
                    matches!(choice, Some(ApprovalChoice::Accept | ApprovalChoice::AcceptAll))
                }
                _ = handle.recv_abort() => false,
            };
            handle.clear_approval();
            if !approved {
                handle.emit(Event::assistant("Okay, I won't do that."));
                continue;
            }

            handle.set_working(true);
            handle.emit(Event::assistant(format!("got it: `{line}`")));
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    app.run().await
}

async fn run_transcript(h: &ReplHandle) {
    sleep(Duration::from_millis(250)).await;

    h.emit(Event::user(
        "Add a dark-mode toggle to the **Settings** page and persist the choice to \
         `localStorage`. Match the existing toggle styling.",
    ));
    sleep(Duration::from_millis(700)).await;

    h.emit(Event::assistant(
        "I'll look at how theming and the Settings page are wired up, then add a toggle \
         that reuses the existing component.",
    ));
    sleep(Duration::from_millis(500)).await;

    h.emit(Event::Reasoning {
        text: "There's probably a `ThemeProvider` exposing `theme` + `setTheme`. I should:\n\
               - grep for `theme` to find the provider and consumers\n\
               - check `src/components/ui` for a reusable Toggle to match styling\n\
               - read the Settings page to see where to slot the row\n\
               Then wire persistence by reading the initial value from localStorage and \
               writing on change.".to_string(),
        ms: Some(1400),
        default_open: false,
    });
    sleep(Duration::from_millis(400)).await;

    // ---- tool: search ----
    let search_call = ToolCall::new("theme", ToolKind::Search {
        result: SearchResult {
            count: 5,
            groups: vec![
                SearchGroup {
                    file: "src/theme/ThemeProvider.tsx".into(),
                    hits: vec![
                        SearchHit { line: 12, text: "const [theme, setTheme] = useState<Theme>('light')".into() },
                        SearchHit { line: 31, text: "return <Ctx.Provider value={{ theme, setTheme }}>".into() },
                    ],
                },
                SearchGroup {
                    file: "src/pages/Settings.tsx".into(),
                    hits: vec![SearchHit {
                        line: 4,
                        text: "import { useTheme } from '../theme/ThemeProvider'".into(),
                    }],
                },
                SearchGroup {
                    file: "src/components/ui/Toggle.tsx".into(),
                    hits: vec![SearchHit {
                        line: 8,
                        text: "export function Toggle({ checked, onChange, label }: ToggleProps)".into(),
                    }],
                },
            ],
        },
    });
    let h_search = h.start_tool(search_call.clone());
    sleep(Duration::from_millis(700)).await;
    h.finish_tool(h_search, search_call);

    // ---- tool: read ----
    let read_call = ToolCall::new(
        "src/theme/ThemeProvider.tsx",
        ToolKind::Read {
            path: "src/theme/ThemeProvider.tsx".into(),
            lines: 38,
            preview: vec![
                ReadLine { n: 10, text: "export function ThemeProvider({ children }: Props) {".into() },
                ReadLine { n: 12, text: "  const [theme, setTheme] = useState<Theme>('light')".into() },
                ReadLine { n: 13, text: "  useEffect(() => {".into() },
                ReadLine { n: 14, text: "    document.documentElement.dataset.theme = theme".into() },
                ReadLine { n: 15, text: "  }, [theme])".into() },
            ],
        },
    );
    let h_read = h.start_tool(read_call.clone());
    sleep(Duration::from_millis(600)).await;
    h.finish_tool(h_read, read_call);

    // ---- tool: list ----
    let list_call = ToolCall::new(
        "src/components/ui",
        ToolKind::List {
            entries: vec![
                ListEntry { name: "Button.tsx".into(), entry_type: EntryType::File, meta: Some("1.2 KB".into()) },
                ListEntry { name: "Toggle.tsx".into(), entry_type: EntryType::File, meta: Some("0.8 KB".into()) },
                ListEntry { name: "Select.tsx".into(), entry_type: EntryType::File, meta: Some("1.6 KB".into()) },
                ListEntry { name: "icons".into(), entry_type: EntryType::Dir, meta: Some("7 files".into()) },
            ],
        },
    );
    let h_list = h.start_tool(list_call.clone());
    sleep(Duration::from_millis(500)).await;
    h.finish_tool(h_list, list_call);

    h.emit(Event::assistant(
        "Found a `ThemeProvider` (`theme` / `setTheme`) and a reusable `Toggle`. Plan:",
    ));
    sleep(Duration::from_millis(300)).await;

    // ---- todo (initial) ----
    let todo_initial = ToolCall::new(
        "plan",
        ToolKind::Todo {
            items: vec![
                TodoItem { state: TodoState::Done, text: "Locate theme provider + Settings page".into() },
                TodoItem { state: TodoState::Active, text: "Add a dark-mode row to Settings using <Toggle>".into() },
                TodoItem { state: TodoState::Pending, text: "Persist theme to localStorage".into() },
                TodoItem { state: TodoState::Pending, text: "Verify typecheck + tests pass".into() },
            ],
        },
    );
    h.tool(todo_initial);
    sleep(Duration::from_millis(400)).await;

    // ---- edit: Settings.tsx ----
    let settings_edit = ToolCall::new(
        "src/pages/Settings.tsx",
        ToolKind::Edit {
            diff: vec![
                DiffLine { kind: DiffKind::Ctx, a: Some(4), b: Some(4), text: "import { useTheme } from '../theme/ThemeProvider'".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(5), text: "import { Toggle } from '../components/ui/Toggle'".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(5), b: Some(6), text: "".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(18), b: Some(19), text: "export function Settings() {".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(20), text: "  const { theme, setTheme } = useTheme()".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(19), b: Some(21), text: "  return (".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(20), b: Some(22), text: "    <section className=\"settings\">".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(23), text: "      <Toggle".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(24), text: "        label=\"Dark mode\"".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(25), text: "        checked={theme === 'dark'}".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(26), text: "        onChange={(on) => setTheme(on ? 'dark' : 'light')}".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(27), text: "      />".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(21), b: Some(28), text: "      <AccountPanel />".into() },
            ],
        },
    );
    let h_edit1 = h.start_tool(settings_edit.clone());
    sleep(Duration::from_millis(900)).await;
    h.finish_tool(h_edit1, settings_edit);

    // ---- edit: ThemeProvider.tsx (introduce localStorage) ----
    let theme_edit1 = ToolCall::new(
        "src/theme/ThemeProvider.tsx",
        ToolKind::Edit {
            diff: vec![
                DiffLine { kind: DiffKind::Del, a: Some(12), b: None, text: "  const [theme, setTheme] = useState<Theme>('light')".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(12), text: "  const [theme, setTheme] = useState<Theme>(".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(13), text: "    () => localStorage.getItem('theme') as Theme".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(14), text: "  )".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(13), b: Some(15), text: "  useEffect(() => {".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(16), text: "    localStorage.setItem('theme', theme)".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(14), b: Some(17), text: "    document.documentElement.dataset.theme = theme".into() },
                DiffLine { kind: DiffKind::Ctx, a: Some(15), b: Some(18), text: "  }, [theme])".into() },
            ],
        },
    );
    let h_edit2 = h.start_tool(theme_edit1.clone());
    sleep(Duration::from_millis(800)).await;
    h.finish_tool(h_edit2, theme_edit1);

    // ---- bash: failing typecheck ----
    let typecheck_fail = ToolCall::new(
        "pnpm typecheck",
        ToolKind::Bash {
            cmd: "pnpm typecheck".into(),
            output: "src/theme/ThemeProvider.tsx:12:10 - error TS2345:\n  \
                     Type 'Theme | null' is not assignable to type 'Theme'.\n\n  \
                     12   const [theme, setTheme] = useState<Theme>(\n              \
                     ~~~~~\n\nFound 1 error in src/theme/ThemeProvider.tsx:12"
                .into(),
            exit: Some(2),
        },
    );
    let h_bash1 = h.start_tool(typecheck_fail.clone());
    sleep(Duration::from_millis(1500)).await;
    h.finish_tool(h_bash1, typecheck_fail);

    h.emit(Event::error(
        "Typecheck failed \u{2014} 1 error (TS2345)",
        "`localStorage.getItem` returns `string | null`, but `useState<Theme>` expects \
         a non-null `Theme`. Default to `'light'` when nothing is stored.",
    ));
    sleep(Duration::from_millis(800)).await;

    h.emit(Event::assistant(
        "The stored value can be `null` on first load. I'll fall back to `'light'`.",
    ));
    sleep(Duration::from_millis(400)).await;

    // ---- edit: fallback ----
    let theme_edit2 = ToolCall::new(
        "src/theme/ThemeProvider.tsx",
        ToolKind::Edit {
            diff: vec![
                DiffLine { kind: DiffKind::Del, a: Some(13), b: None, text: "    () => localStorage.getItem('theme') as Theme".into() },
                DiffLine { kind: DiffKind::Add, a: None, b: Some(13), text: "    () => (localStorage.getItem('theme') as Theme) ?? 'light'".into() },
            ],
        },
    );
    let h_edit3 = h.start_tool(theme_edit2.clone());
    sleep(Duration::from_millis(700)).await;
    h.finish_tool(h_edit3, theme_edit2);

    // ---- bash: passing typecheck ----
    let typecheck_ok = ToolCall::new(
        "pnpm typecheck",
        ToolKind::Bash {
            cmd: "pnpm typecheck".into(),
            output: "\u{2713} No type errors. (1.8s)".into(),
            exit: Some(0),
        },
    );
    let h_bash2 = h.start_tool(typecheck_ok.clone());
    sleep(Duration::from_millis(1300)).await;
    h.finish_tool(h_bash2, typecheck_ok);

    // ---- bash: passing tests ----
    let test_ok = ToolCall::new(
        "pnpm test settings",
        ToolKind::Bash {
            cmd: "pnpm test settings".into(),
            output: "PASS  src/pages/Settings.test.tsx\n  Settings\n    \
                     \u{2713} renders a dark mode toggle (24 ms)\n    \
                     \u{2713} persists theme to localStorage (9 ms)\n    \
                     \u{2713} reads stored theme on mount (6 ms)\n\nTests: 3 passed, 3 total"
                .into(),
            exit: Some(0),
        },
    );
    let h_bash3 = h.start_tool(test_ok.clone());
    sleep(Duration::from_millis(1400)).await;
    h.finish_tool(h_bash3, test_ok);

    // ---- todo (final) ----
    let todo_final = ToolCall::new(
        "plan",
        ToolKind::Todo {
            items: vec![
                TodoItem { state: TodoState::Done, text: "Locate theme provider + Settings page".into() },
                TodoItem { state: TodoState::Done, text: "Add a dark-mode row to Settings using <Toggle>".into() },
                TodoItem { state: TodoState::Done, text: "Persist theme to localStorage".into() },
                TodoItem { state: TodoState::Done, text: "Verify typecheck + tests pass".into() },
            ],
        },
    );
    h.tool(todo_final);
    sleep(Duration::from_millis(400)).await;

    h.emit(Event::assistant(
        "Done. The **Settings** page now has a *Dark mode* row using the existing \
         `<Toggle>`, and the choice persists across reloads.\n\n\
         - `Settings.tsx` \u{2014} toggle wired to `useTheme()`\n\
         - `ThemeProvider.tsx` \u{2014} reads from `localStorage` on init, writes on \
         change (defaults to `light`)\n\
         - Typecheck clean, 3 tests passing",
    ));
}
