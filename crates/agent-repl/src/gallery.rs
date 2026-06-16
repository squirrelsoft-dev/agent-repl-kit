//! Static sampler containing one of every event type. Ported from the
//! `GALLERY` array in `docs/repl/data.jsx`. Used by the demo's Gallery
//! view (`2` key) so you can eyeball every block in the current theme at
//! once.

use agent_repl_core::{
    AlertLevel, DiffKind, DiffLine, EntryType, Event, ListEntry, ReadLine, SearchGroup, SearchHit,
    SearchResult, TodoItem, TodoState, ToolCall, ToolKind,
};

use crate::stream::Stream;

/// Build a [`Stream`] populated with one of every block kind.
pub fn build() -> Stream {
    let mut s = Stream::default();
    for ev in events() {
        s.push(ev);
    }
    s
}

/// Just the events, in display order. Same content as the JSX `GALLERY`.
pub fn events() -> Vec<Event> {
    vec![
        Event::user("Refactor the auth middleware to use the new `verifyToken` helper."),

        Event::assistant(
            "Here's the approach:\n\n\
             1. Swap the inline JWT check for `verifyToken`\n\
             2. Return `401` on failure\n\n\
             > Heads up: this touches every protected route.\n\n\
             ```ts\n\
             const user = await verifyToken(req.headers.authorization)\n\
             ```",
        ),

        Event::Reasoning {
            text: "The middleware currently decodes the JWT manually. `verifyToken` \
                   already handles expiry + signature, so I can delete ~15 lines and \
                   just await it. Need to keep the `req.user` shape identical to avoid \
                   breaking downstream handlers."
                .into(),
            ms: Some(2100),
            default_open: true,
        },

        Event::Tool(ToolCall::new(
            "verifyToken",
            ToolKind::Search {
                result: SearchResult {
                    count: 3,
                    groups: vec![
                        SearchGroup {
                            file: "src/auth/token.ts".into(),
                            hits: vec![SearchHit {
                                line: 22,
                                text: "export async function verifyToken(raw?: string)".into(),
                            }],
                        },
                        SearchGroup {
                            file: "src/middleware/auth.ts".into(),
                            hits: vec![
                                SearchHit { line: 9, text: "const payload = jwt.decode(token)".into() },
                                SearchHit { line: 14, text: "req.user = payload".into() },
                            ],
                        },
                    ],
                },
            },
        )),

        Event::Tool(ToolCall::new(
            "src/middleware/auth.ts",
            ToolKind::Read {
                path: "src/middleware/auth.ts".into(),
                lines: 27,
                preview: vec![
                    ReadLine { n: 7, text: "export function auth(req, res, next) {".into() },
                    ReadLine { n: 8, text: "  const token = req.headers.authorization".into() },
                    ReadLine { n: 9, text: "  const payload = jwt.decode(token)".into() },
                ],
            },
        )),

        Event::Tool(ToolCall::new(
            "src/middleware",
            ToolKind::List {
                entries: vec![
                    ListEntry { name: "auth.ts".into(), entry_type: EntryType::File, meta: Some("0.9 KB".into()) },
                    ListEntry { name: "cors.ts".into(), entry_type: EntryType::File, meta: Some("0.4 KB".into()) },
                    ListEntry { name: "rateLimit.ts".into(), entry_type: EntryType::File, meta: Some("1.1 KB".into()) },
                    ListEntry { name: "__tests__".into(), entry_type: EntryType::Dir, meta: Some("4 files".into()) },
                ],
            },
        )),

        Event::Tool(ToolCall::new(
            "src/middleware/auth.ts",
            ToolKind::Edit {
                diff: vec![
                    DiffLine { kind: DiffKind::Ctx, a: Some(7), b: Some(7), text: "export async function auth(req, res, next) {".into() },
                    DiffLine { kind: DiffKind::Del, a: Some(8), b: None, text: "  const token = req.headers.authorization".into() },
                    DiffLine { kind: DiffKind::Del, a: Some(9), b: None, text: "  const payload = jwt.decode(token)".into() },
                    DiffLine { kind: DiffKind::Add, a: None, b: Some(8), text: "  const user = await verifyToken(req.headers.authorization)".into() },
                    DiffLine { kind: DiffKind::Add, a: None, b: Some(9), text: "  if (!user) return res.status(401).end()".into() },
                    DiffLine { kind: DiffKind::Ctx, a: Some(10), b: Some(10), text: "  req.user = user".into() },
                ],
            },
        )),

        Event::Tool(ToolCall::new(
            "pnpm test auth",
            ToolKind::Bash {
                cmd: "pnpm test auth".into(),
                output: "PASS  src/middleware/__tests__/auth.test.ts\n  \
                         \u{2713} rejects missing token (12 ms)\n  \
                         \u{2713} attaches req.user (8 ms)\n\nTests: 2 passed, 2 total"
                    .into(),
                exit: Some(0),
            },
        )),

        Event::Tool(ToolCall::new(
            "plan",
            ToolKind::Todo {
                items: vec![
                    TodoItem { state: TodoState::Done, text: "Read current middleware".into() },
                    TodoItem { state: TodoState::Active, text: "Swap in verifyToken".into() },
                    TodoItem { state: TodoState::Pending, text: "Update tests".into() },
                ],
            },
        )),

        Event::Alert {
            level: AlertLevel::Error,
            title: "Command failed \u{2014} exit 1".into(),
            detail: Some(
                "`verifyToken` is async but `auth` was not awaited. Mark the handler \
                 `async` and `await` the call."
                    .into(),
            ),
        },

        Event::Alert {
            level: AlertLevel::Warning,
            title: "Deprecation: jwt.decode() does not verify signatures".into(),
            detail: Some(
                "Left in 1 call site outside this change \u{2014} consider migrating \
                 `src/jobs/cron.ts` separately."
                    .into(),
            ),
        },

        Event::status("Running test suite\u{2026}"),
    ]
}
