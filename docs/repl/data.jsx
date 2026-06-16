// data.jsx — a realistic agent run + a gallery sampler. Exposed on window.

(function () {
  // ---- TRANSCRIPT: "Add a dark-mode toggle to Settings, persist to localStorage"
  const TRANSCRIPT = [
    { type: 'user', text: 'Add a dark-mode toggle to the **Settings** page and persist the choice to `localStorage`. Match the existing toggle styling.' },

    { type: 'assistant', text: "I'll look at how theming and the Settings page are wired up, then add a toggle that reuses the existing component." },

    { type: 'reasoning', ms: 1400, text: "There's probably a `ThemeProvider` exposing `theme` + `setTheme`. I should:\n- grep for `theme` to find the provider and consumers\n- check `src/components/ui` for a reusable Toggle to match styling\n- read the Settings page to see where to slot the row\nThen wire persistence by reading the initial value from localStorage and writing on change." },

    { type: 'tool', kind: 'search', title: 'theme', wait: 700,
      result: { count: 5, groups: [
        { file: 'src/theme/ThemeProvider.tsx', hits: [
          { line: 12, text: "const [theme, setTheme] = useState<Theme>('light')" },
          { line: 31, text: 'return <Ctx.Provider value={{ theme, setTheme }}>' },
        ]},
        { file: 'src/pages/Settings.tsx', hits: [
          { line: 4, text: "import { useTheme } from '../theme/ThemeProvider'" },
        ]},
        { file: 'src/components/ui/Toggle.tsx', hits: [
          { line: 8, text: 'export function Toggle({ checked, onChange, label }: ToggleProps)' },
        ]},
      ] } },

    { type: 'tool', kind: 'read', title: 'src/theme/ThemeProvider.tsx', wait: 600, lines: 38,
      preview: [
        { n: 10, text: 'export function ThemeProvider({ children }: Props) {' },
        { n: 12, text: "  const [theme, setTheme] = useState<Theme>('light')" },
        { n: 13, text: '  useEffect(() => {' },
        { n: 14, text: "    document.documentElement.dataset.theme = theme" },
        { n: 15, text: '  }, [theme])' },
      ] },

    { type: 'tool', kind: 'list', title: 'src/components/ui', wait: 500,
      entries: [
        { name: 'Button.tsx', type: 'file', meta: '1.2 KB' },
        { name: 'Toggle.tsx', type: 'file', meta: '0.8 KB' },
        { name: 'Select.tsx', type: 'file', meta: '1.6 KB' },
        { name: 'icons', type: 'dir', meta: '7 files' },
      ] },

    { type: 'assistant', text: "Found a `ThemeProvider` (`theme` / `setTheme`) and a reusable `Toggle`. Plan:" },

    { type: 'tool', kind: 'todo', title: 'plan', wait: 400,
      items: [
        { state: 'done', text: 'Locate theme provider + Settings page' },
        { state: 'active', text: 'Add a dark-mode row to Settings using <Toggle>' },
        { state: 'pending', text: 'Persist theme to localStorage' },
        { state: 'pending', text: 'Verify typecheck + tests pass' },
      ] },

    { type: 'tool', kind: 'edit', title: 'src/pages/Settings.tsx', wait: 900,
      diff: [
        { t: 'ctx', a: 4, b: 4, text: "import { useTheme } from '../theme/ThemeProvider'" },
        { t: 'add', b: 5, text: "import { Toggle } from '../components/ui/Toggle'" },
        { t: 'ctx', a: 5, b: 6, text: '' },
        { t: 'ctx', a: 18, b: 19, text: 'export function Settings() {' },
        { t: 'add', b: 20, text: '  const { theme, setTheme } = useTheme()' },
        { t: 'ctx', a: 19, b: 21, text: '  return (' },
        { t: 'ctx', a: 20, b: 22, text: '    <section className="settings">' },
        { t: 'add', b: 23, text: '      <Toggle' },
        { t: 'add', b: 24, text: '        label="Dark mode"' },
        { t: 'add', b: 25, text: "        checked={theme === 'dark'}" },
        { t: 'add', b: 26, text: "        onChange={(on) => setTheme(on ? 'dark' : 'light')}" },
        { t: 'add', b: 27, text: '      />' },
        { t: 'ctx', a: 21, b: 28, text: '      <AccountPanel />' },
      ] },

    { type: 'tool', kind: 'edit', title: 'src/theme/ThemeProvider.tsx', wait: 800,
      diff: [
        { t: 'del', a: 12, text: "  const [theme, setTheme] = useState<Theme>('light')" },
        { t: 'add', b: 12, text: '  const [theme, setTheme] = useState<Theme>(' },
        { t: 'add', b: 13, text: "    () => localStorage.getItem('theme') as Theme" },
        { t: 'add', b: 14, text: '  )' },
        { t: 'ctx', a: 13, b: 15, text: '  useEffect(() => {' },
        { t: 'add', b: 16, text: "    localStorage.setItem('theme', theme)" },
        { t: 'ctx', a: 14, b: 17, text: '    document.documentElement.dataset.theme = theme' },
        { t: 'ctx', a: 15, b: 18, text: '  }, [theme])' },
      ] },

    { type: 'tool', kind: 'bash', title: 'pnpm typecheck', wait: 1500, exit: 2,
      cmd: 'pnpm typecheck',
      output: 'src/theme/ThemeProvider.tsx:12:10 - error TS2345:\n  Type \'Theme | null\' is not assignable to type \'Theme\'.\n\n  12   const [theme, setTheme] = useState<Theme>(\n              ~~~~~\n\nFound 1 error in src/theme/ThemeProvider.tsx:12' },

    { type: 'alert', level: 'error', title: 'Typecheck failed \u2014 1 error (TS2345)',
      detail: '`localStorage.getItem` returns `string | null`, but `useState<Theme>` expects a non-null `Theme`. Default to `\'light\'` when nothing is stored.' },

    { type: 'assistant', text: "The stored value can be `null` on first load. I'll fall back to `'light'`." },

    { type: 'tool', kind: 'edit', title: 'src/theme/ThemeProvider.tsx', wait: 700,
      diff: [
        { t: 'del', a: 13, text: "    () => localStorage.getItem('theme') as Theme" },
        { t: 'add', b: 13, text: "    () => (localStorage.getItem('theme') as Theme) ?? 'light'" },
      ] },

    { type: 'tool', kind: 'bash', title: 'pnpm typecheck', wait: 1300, exit: 0,
      cmd: 'pnpm typecheck', output: '\u2713 No type errors. (1.8s)' },

    { type: 'tool', kind: 'bash', title: 'pnpm test settings', wait: 1400, exit: 0,
      cmd: 'pnpm test settings',
      output: 'PASS  src/pages/Settings.test.tsx\n  Settings\n    \u2713 renders a dark mode toggle (24 ms)\n    \u2713 persists theme to localStorage (9 ms)\n    \u2713 reads stored theme on mount (6 ms)\n\nTests: 3 passed, 3 total' },

    { type: 'tool', kind: 'todo', title: 'plan', wait: 400,
      items: [
        { state: 'done', text: 'Locate theme provider + Settings page' },
        { state: 'done', text: 'Add a dark-mode row to Settings using <Toggle>' },
        { state: 'done', text: 'Persist theme to localStorage' },
        { state: 'done', text: 'Verify typecheck + tests pass' },
      ] },

    { type: 'assistant', text: "Done. The **Settings** page now has a *Dark mode* row using the existing `<Toggle>`, and the choice persists across reloads.\n\n- `Settings.tsx` \u2014 toggle wired to `useTheme()`\n- `ThemeProvider.tsx` \u2014 reads from `localStorage` on init, writes on change (defaults to `light`)\n- Typecheck clean, 3 tests passing" },
  ];

  // ---- GALLERY: one of each block type, for the static spec view ----------
  const GALLERY = [
    { id: 'user', label: 'User message', ev: { type: 'user', text: 'Refactor the auth middleware to use the new `verifyToken` helper.' } },
    { id: 'assistant', label: 'Assistant (markdown)', ev: { type: 'assistant', text: "Here's the approach:\n\n1. Swap the inline JWT check for `verifyToken`\n2. Return `401` on failure\n\n> Heads up: this touches every protected route.\n\n```ts\nconst user = await verifyToken(req.headers.authorization)\n```" } },
    { id: 'reasoning', label: 'Reasoning / thinking', ev: { type: 'reasoning', defaultOpen: true, ms: 2100, text: 'The middleware currently decodes the JWT manually. `verifyToken` already handles expiry + signature, so I can delete ~15 lines and just await it. Need to keep the `req.user` shape identical to avoid breaking downstream handlers.' } },
    { id: 'search', label: 'Search / grep', ev: { type: 'tool', kind: 'search', title: 'verifyToken', result: { count: 3, groups: [{ file: 'src/auth/token.ts', hits: [{ line: 22, text: 'export async function verifyToken(raw?: string)' }] }, { file: 'src/middleware/auth.ts', hits: [{ line: 9, text: 'const payload = jwt.decode(token)' }, { line: 14, text: 'req.user = payload' }] }] } } },
    { id: 'read', label: 'Read file', ev: { type: 'tool', kind: 'read', title: 'src/middleware/auth.ts', lines: 27, preview: [{ n: 7, text: 'export function auth(req, res, next) {' }, { n: 8, text: '  const token = req.headers.authorization' }, { n: 9, text: '  const payload = jwt.decode(token)' }] } },
    { id: 'list', label: 'File listing', ev: { type: 'tool', kind: 'list', title: 'src/middleware', entries: [{ name: 'auth.ts', type: 'file', meta: '0.9 KB' }, { name: 'cors.ts', type: 'file', meta: '0.4 KB' }, { name: 'rateLimit.ts', type: 'file', meta: '1.1 KB' }, { name: '__tests__', type: 'dir', meta: '4 files' }] } },
    { id: 'edit', label: 'File diff (edit)', ev: { type: 'tool', kind: 'edit', title: 'src/middleware/auth.ts', diff: [{ t: 'ctx', a: 7, b: 7, text: 'export async function auth(req, res, next) {' }, { t: 'del', a: 8, text: '  const token = req.headers.authorization' }, { t: 'del', a: 9, text: '  const payload = jwt.decode(token)' }, { t: 'add', b: 8, text: '  const user = await verifyToken(req.headers.authorization)' }, { t: 'add', b: 9, text: '  if (!user) return res.status(401).end()' }, { t: 'ctx', a: 10, b: 10, text: '  req.user = user' }] } },
    { id: 'bash', label: 'Shell command', ev: { type: 'tool', kind: 'bash', title: 'pnpm test auth', exit: 0, cmd: 'pnpm test auth', output: 'PASS  src/middleware/__tests__/auth.test.ts\n  \u2713 rejects missing token (12 ms)\n  \u2713 attaches req.user (8 ms)\n\nTests: 2 passed, 2 total' } },
    { id: 'todo', label: 'Todo / task list', ev: { type: 'tool', kind: 'todo', title: 'plan', items: [{ state: 'done', text: 'Read current middleware' }, { state: 'active', text: 'Swap in verifyToken' }, { state: 'pending', text: 'Update tests' }] } },
    { id: 'error', label: 'Error / warning', ev: { type: 'alert', level: 'error', title: 'Command failed \u2014 exit 1', detail: '`verifyToken` is async but `auth` was not awaited. Mark the handler `async` and `await` the call.' } },
    { id: 'warn', label: 'Warning', ev: { type: 'alert', level: 'warning', title: 'Deprecation: jwt.decode() does not verify signatures', detail: 'Left in 1 call site outside this change \u2014 consider migrating `src/jobs/cron.ts` separately.' } },
    { id: 'status', label: 'Status / streaming', ev: { type: 'status', text: 'Running test suite\u2026' } },
  ];

  Object.assign(window, { TRANSCRIPT, GALLERY });
})();
