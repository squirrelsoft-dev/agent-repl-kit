// input.jsx — the REPL composer (input). Token-driven, tool-style-aware.
// Exposes InputFrame, InputStates, LiveComposer on window.

(function () {
  const { useState, useRef, useEffect } = React;
  const Spinner = () => React.createElement(window.Spinner);

  const SLASH = [
    { cmd: '/clear', desc: 'Clear the conversation' },
    { cmd: '/compact', desc: 'Summarize & free up context' },
    { cmd: '/model', desc: 'Switch the active model' },
    { cmd: '/diff', desc: 'Review pending changes' },
    { cmd: '/undo', desc: 'Revert the last edit' },
    { cmd: '/run', desc: 'Run a shell command' },
  ];

  function Kbd({ children }) { return <span className="kbd">{children}</span>; }

  function SlashMenu({ query = '', sel = 0, placement = 'top' }) {
    const q = query.replace(/^\//, '').toLowerCase();
    const items = SLASH.filter(s => s.cmd.slice(1).startsWith(q));
    const list = items.length ? items : SLASH;
    return (
      <div className={'slash slash-' + placement}>
        <div className="slash-head">commands</div>
        {list.map((s, i) => (
          <div key={s.cmd} className={'slash-row' + (i === sel ? ' sel' : '')}>
            <span className="slash-cmd">{s.cmd}</span>
            <span className="slash-desc">{s.desc}</span>
          </div>
        ))}
      </div>
    );
  }

  function ContextChips({ chips }) {
    return (
      <div className="ipt-chips">
        {chips.map((c, i) => (
          <span key={i} className={'chip chip-' + (c.kind || 'file')}>
            <span className="chip-icon">{c.kind === 'img' ? '\u25A3' : '@'}</span>
            <span className="chip-name">{c.name}</span>
            <span className="chip-x">{'\u00D7'}</span>
          </span>
        ))}
        <button className="chip chip-add">+ add context</button>
      </div>
    );
  }

  function FooterHints({ working }) {
    if (working) {
      return (
        <div className="ipt-footer working">
          <span className="foot-left"><Spinner /> <span>{'Working\u2026'}</span></span>
          <span className="foot-right"><Kbd>esc</Kbd> to interrupt</span>
        </div>
      );
    }
    return (
      <div className="ipt-footer">
        <span className="foot-left">
          <span className="ctx-pill">{'\u25C7 sonnet-4'}</span>
          <span className="ctx-dim">~/web-app</span>
          <span className="ctx-dim">{'\u2387 main'}</span>
        </span>
        <span className="foot-right">
          <Kbd>{'\u23CE'}</Kbd> send <Kbd>{'\u21E7\u23CE'}</Kbd> newline <Kbd>/</Kbd> cmds <Kbd>@</Kbd> files
        </span>
      </div>
    );
  }

  // Presentational shell. `fieldNode` is the editable/placeholder slot.
  function InputFrame({ chips, slash, error, working, fieldNode, footer = true, onSend, onStop }) {
    return (
      <div className={'ipt' + (working ? ' working' : '') + (error ? ' has-error' : '')}>
        {slash && <SlashMenu query={slash.query} sel={slash.sel} placement={slash.placement} />}
        {chips && chips.length > 0 && <ContextChips chips={chips} />}
        {error && <div className="ipt-alert"><span className="ipt-alert-sym">{'\u26A0'}</span> {error}</div>}
        <div className="ipt-main">
          <span className="ipt-sigil">{working ? <Spinner /> : '\u276F'}</span>
          <div className="ipt-fieldwrap">{fieldNode}</div>
          <div className="ipt-actions">
            {working
              ? <button className="stop-btn" onClick={onStop}><span className="stop-sq" /> stop</button>
              : <button className="send-btn" onClick={onSend}>send <span className="send-arrow">{'\u23CE'}</span></button>}
          </div>
        </div>
        {footer && <FooterHints working={working} />}
      </div>
    );
  }

  // Static field for the gallery.
  function StaticField({ value, placeholder }) {
    if (!value) return <div className="ipt-field empty">{placeholder}</div>;
    return <div className="ipt-field"><span className="ipt-text">{value}</span><span className="caret" /></div>;
  }

  // Gallery of every composer state.
  function InputStates() {
    const rows = [
      { label: 'Idle / ready', node: <InputFrame fieldNode={<StaticField placeholder={'Ask the agent to do something\u2026'} />} /> },
      { label: 'Typing', node: <InputFrame fieldNode={<StaticField value="Add a dark-mode toggle to the Settings page" />} /> },
      { label: 'Multiline + code', node: <InputFrame fieldNode={<StaticField value={"Make theme loading resilient:\n\n  const t = localStorage.getItem('theme')\n\nFall back to 'light' when it's null."} />} /> },
      { label: 'Slash commands', node: <InputFrame slash={{ query: '', sel: 1, placement: 'below' }} fieldNode={<StaticField value="/" />} /> },
      { label: 'With context (@files + attachment)', node: <InputFrame chips={[{ kind: 'file', name: 'src/pages/Settings.tsx' }, { kind: 'file', name: 'ThemeProvider.tsx' }, { kind: 'img', name: 'mockup.png' }]} fieldNode={<StaticField value="Match the toggle styling shown in the mockup" />} /> },
      { label: 'Agent working (interrupt)', node: <InputFrame working fieldNode={<div className="ipt-field empty">esc to interrupt, or keep typing to queue\u2026</div>} /> },
      { label: 'Context-limit warning', node: <InputFrame error={'Approaching context limit \u2014 run /compact to free space'} fieldNode={<StaticField value="\u2026and update the tests too" />} /> },
    ];
    return (
      <div className="canvas">
        <div className="canvas-inner gallery">
          <p className="gallery-intro">The composer across states. It inherits the current vibe, mode, and density; its frame follows the tool-style — flush under a hairline for <b>Inline</b>, a bordered box for <b>Card</b> / <b>Collapsed</b>.</p>
          {rows.map(r => (
            <div className="gallery-item" key={r.label}>
              <div className="gallery-label">{r.label}</div>
              <div className="gallery-block">{r.node}</div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  // Interactive composer for the live demo dock.
  function LiveComposer({ working, onSubmit }) {
    const [val, setVal] = useState('');
    const ta = useRef(null);
    const slashOpen = val.trim().startsWith('/');

    useEffect(() => {
      const el = ta.current; if (!el) return;
      el.style.height = 'auto';
      el.style.height = Math.min(el.scrollHeight, 150) + 'px';
    }, [val]);

    function submit() { const t = val.trim(); if (!t || working) return; onSubmit(t); setVal(''); }
    function onKey(e) { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); submit(); } }

    const field = (
      <textarea ref={ta} className="ipt-field live" rows={1} disabled={working}
        placeholder={working ? 'agent is working\u2026 (esc to interrupt)' : 'Ask the agent to do something\u2026'}
        value={val} onChange={e => setVal(e.target.value)} onKeyDown={onKey} />
    );

    return (
      <div className="composer">
        <div className="composer-inner">
          <InputFrame working={working} onSend={submit}
            slash={slashOpen ? { query: val.trim(), sel: 0, placement: 'top' } : null}
            fieldNode={field} />
        </div>
      </div>
    );
  }

  Object.assign(window, { InputFrame, InputStates, LiveComposer });
})();
