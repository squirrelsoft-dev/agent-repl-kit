// blocks.jsx — renderer components for every agent output type.
// Reads vibe/toolStyle/density from ReplCtx. Exposes block components on window.

(function () {
  const { useContext, useState } = React;

  // --- Spinner (braille) ----------------------------------------------------
  const FRAMES = ['\u280B', '\u2819', '\u2839', '\u2838', '\u283C', '\u2834', '\u2826', '\u2827', '\u2807', '\u280F'];
  function Spinner() {
    const [i, setI] = useState(0);
    React.useEffect(() => { const t = setInterval(() => setI(v => (v + 1) % FRAMES.length), 80); return () => clearInterval(t); }, []);
    return <span className="spinner">{FRAMES[i]}</span>;
  }

  function Badge({ kind, children }) { return <span className={'badge badge-' + kind}>{children}</span>; }

  function RoleLabel({ children }) { return <div className="role-label">{children}</div>; }

  // --- User message ---------------------------------------------------------
  function UserMsg({ text }) {
    return (
      <div className="block msg-user">
        <RoleLabel>user</RoleLabel>
        <div className="user-bubble"><window.Markdown>{text}</window.Markdown></div>
      </div>
    );
  }

  // --- Assistant message ----------------------------------------------------
  function AssistantMsg({ text, streaming }) {
    return (
      <div className="block msg-assistant">
        <RoleLabel>assistant</RoleLabel>
        <window.Markdown>{text}</window.Markdown>
        {streaming && <span className="caret" />}
      </div>
    );
  }

  // --- Reasoning / thinking -------------------------------------------------
  function Reasoning({ text, ms, defaultOpen }) {
    const [open, setOpen] = useState(!!defaultOpen);
    return (
      <div className={'block reasoning' + (open ? ' open' : '')}>
        <button className="reasoning-head" onClick={() => setOpen(o => !o)}>
          <span className="chev">{open ? '\u25BE' : '\u25B8'}</span>
          <span className="reasoning-title">Thought{ms ? ' for ' + (ms / 1000).toFixed(1) + 's' : ''}</span>
        </button>
        {open && <div className="reasoning-body"><window.Markdown>{text}</window.Markdown></div>}
      </div>
    );
  }

  // --- Status / progress line ----------------------------------------------
  function StatusLine({ children }) {
    return <div className="block status"><Spinner /><span className="status-text">{children}</span></div>;
  }

  // --- Error / warning ------------------------------------------------------
  function Alert({ level = 'error', title, children }) {
    const sym = level === 'warning' ? '\u26A0' : '\u2715';
    return (
      <div className={'block alert alert-' + level}>
        <div className="alert-head"><span className="alert-sym">{sym}</span><span className="alert-title">{title}</span></div>
        {children && <div className="alert-body">{children}</div>}
      </div>
    );
  }

  // --- Tool body renderers --------------------------------------------------
  function DiffBody({ diff }) {
    return (
      <div className="diff">
        {diff.map((l, i) => (
          <div key={i} className={'diff-line dl-' + l.t}>
            <span className="ln ln-a">{l.t === 'add' ? '' : (l.a ?? '')}</span>
            <span className="ln ln-b">{l.t === 'del' ? '' : (l.b ?? '')}</span>
            <span className="dl-sign">{l.t === 'add' ? '+' : l.t === 'del' ? '\u2212' : '\u00A0'}</span>
            <span className="dl-text">{l.text || '\u00A0'}</span>
          </div>
        ))}
      </div>
    );
  }

  function BashBody({ cmd, output, exit }) {
    return (
      <div className="bash">
        <div className="bash-cmd"><span className="prompt">$</span> {cmd}</div>
        {output != null && output !== '' && <pre className="bash-out">{output}</pre>}
      </div>
    );
  }

  function SearchBody({ result }) {
    return (
      <div className="search">
        {result.groups.map((g, i) => (
          <div key={i} className="search-group">
            <div className="search-file">{g.file}</div>
            {g.hits.map((h, j) => (
              <div key={j} className="search-row">
                <span className="search-ln">{h.line}</span>
                <span className="search-text">{h.text}</span>
              </div>
            ))}
          </div>
        ))}
      </div>
    );
  }

  function ListBody({ entries }) {
    return (
      <div className="ls">
        {entries.map((e, i) => (
          <div key={i} className={'ls-row ls-' + (e.type || 'file')}>
            <span className="ls-glyph">{e.type === 'dir' ? '\u25B8' : '\u00B7'}</span>
            <span className="ls-name">{e.name}{e.type === 'dir' ? '/' : ''}</span>
            {e.meta && <span className="ls-meta">{e.meta}</span>}
          </div>
        ))}
      </div>
    );
  }

  function ReadBody({ path, lines, preview }) {
    return (
      <div className="read">
        {preview && preview.map((p, i) => (
          <div key={i} className="read-row"><span className="read-ln">{p.n}</span><span className="read-text">{p.text}</span></div>
        ))}
        <div className="read-meta">Read {lines} lines from {path}</div>
      </div>
    );
  }

  function TodoBody({ items }) {
    const box = { done: '\u2713', active: '\u25B8', pending: '\u00A0' };
    return (
      <div className="todo">
        {items.map((it, i) => (
          <div key={i} className={'todo-item ti-' + it.state}>
            <span className="todo-box">{box[it.state]}</span>
            <span className="todo-text">{it.text}</span>
          </div>
        ))}
      </div>
    );
  }

  // --- Tool call summary (for collapsed one-liners) ------------------------
  function toolSummary(ev) {
    switch (ev.kind) {
      case 'edit': case 'write': {
        const add = (ev.diff || []).filter(l => l.t === 'add').length;
        const del = (ev.diff || []).filter(l => l.t === 'del').length;
        return `+${add} \u2212${del}`;
      }
      case 'bash': return 'exit ' + (ev.exit ?? 0);
      case 'search': return (ev.result?.count ?? 0) + ' matches';
      case 'list': return (ev.entries?.length ?? 0) + ' items';
      case 'read': return ev.lines + ' lines';
      case 'todo': { const d = ev.items.filter(i => i.state === 'done').length; return d + '/' + ev.items.length; }
      default: return '';
    }
  }

  function ToolBody({ ev }) {
    switch (ev.kind) {
      case 'edit': case 'write': return <DiffBody diff={ev.diff} />;
      case 'bash': return <BashBody cmd={ev.cmd} output={ev.output} exit={ev.exit} />;
      case 'search': return <SearchBody result={ev.result} />;
      case 'list': return <ListBody entries={ev.entries} />;
      case 'read': return <ReadBody path={ev.path} lines={ev.lines} preview={ev.preview} />;
      case 'todo': return <TodoBody items={ev.items} />;
      default: return null;
    }
  }

  // --- Tool call wrapper ----------------------------------------------------
  function ToolCall({ ev, running }) {
    const { toolStyle, colors } = useContext(window.ReplCtx);
    const meta = window.TOOLS[ev.kind] || { label: ev.kind, hue: 'tList' };
    const hue = colors[meta.hue] || colors.accent;
    const [open, setOpen] = useState(false);
    const isOpen = running ? true : (toolStyle === 'collapsed' ? open : true);
    const failed = ev.exit != null && ev.exit !== 0;
    const badge = running
      ? <Badge kind="run"><Spinner /> running</Badge>
      : ev.kind === 'bash'
        ? <Badge kind={failed ? 'err' : 'ok'}>{'exit ' + (ev.exit ?? 0)}</Badge>
        : null;

    return (
      <div className={'block tool tool-' + ev.kind} data-running={running ? '1' : '0'} style={{ '--th': hue }}>
        <div className="tool-head" onClick={() => toolStyle !== 'inline' && !running && setOpen(o => !o)} role={toolStyle !== 'inline' ? 'button' : undefined}>
          {toolStyle === 'collapsed' && <span className="chev tool-chev">{isOpen ? '\u25BE' : '\u25B8'}</span>}
          <span className="tool-dot" />
          <span className="tool-name">{meta.label}</span>
          <span className="tool-title">{ev.title}</span>
          {(toolStyle === 'collapsed' || running) && <span className="tool-summary">{running ? '' : toolSummary(ev)}</span>}
          <span className="tool-status">{badge}</span>
        </div>
        {isOpen && (
          <div className="tool-body">
            {running ? <StatusLine>{ev.runLabel || ('Running ' + meta.label + '\u2026')}</StatusLine> : <ToolBody ev={ev} />}
          </div>
        )}
      </div>
    );
  }

  // --- Dispatch any event ---------------------------------------------------
  function EventBlock({ ev, running, streaming }) {
    switch (ev.type) {
      case 'user': return <UserMsg text={ev.text} />;
      case 'assistant': return <AssistantMsg text={ev.text} streaming={streaming} />;
      case 'reasoning': return <Reasoning text={ev.text} ms={ev.ms} defaultOpen={ev.defaultOpen} />;
      case 'status': return <StatusLine>{ev.text}</StatusLine>;
      case 'alert': return <Alert level={ev.level} title={ev.title}>{ev.detail && <window.Markdown>{ev.detail}</window.Markdown>}</Alert>;
      case 'tool': return <ToolCall ev={ev} running={running} />;
      default: return null;
    }
  }

  Object.assign(window, { Spinner, Badge, RoleLabel, UserMsg, AssistantMsg, Reasoning, StatusLine, Alert, ToolCall, EventBlock });
})();
