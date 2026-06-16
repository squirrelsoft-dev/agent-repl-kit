// app.jsx — toolbar controls, gallery view, and live streaming demo.

(function () {
  const { useState, useRef, useEffect, useMemo, useLayoutEffect } = React;

  function Seg({ label, value, options, onChange }) {
    return (
      <div className="seg-wrap">
        <span className="seg-label">{label}</span>
        <div className="seg" role="tablist">
          {options.map(o => (
            <button key={o.v} className={'seg-btn' + (o.v === value ? ' on' : '')} onClick={() => onChange(o.v)}>{o.label}</button>
          ))}
        </div>
      </div>
    );
  }

  // Pre-flatten the transcript into discrete playback frames.
  function buildFrames(transcript) {
    const frames = [];
    let cur = [];
    const durFor = ev => ev.type === 'reasoning' ? 850 : ev.type === 'assistant' ? 950 : ev.type === 'user' ? 550 : ev.type === 'alert' ? 900 : 600;
    transcript.forEach(ev => {
      if (ev.type === 'tool' && ev.wait) {
        cur = [...cur, { ev, running: true }];
        frames.push({ items: cur, dur: ev.wait });
        cur = [...cur.slice(0, -1), { ev, running: false }];
        frames.push({ items: cur, dur: 320 });
      } else {
        cur = [...cur, { ev, running: false }];
        frames.push({ items: cur, dur: durFor(ev) });
      }
    });
    return frames;
  }

  function Stream({ items, follow }) {
    return (
      <div className="stream">
        {items.map((it, i) => (
          <window.EventBlock key={i} ev={it.ev} running={it.running} streaming={false} />
        ))}
      </div>
    );
  }

  function LiveDemo() {
    const frames = useMemo(() => buildFrames(window.TRANSCRIPT), []);
    const [ptr, setPtr] = useState(0);
    const [playing, setPlaying] = useState(true);
    const [speed, setSpeed] = useState(1.5);
    const [extra, setExtra] = useState([]);
    const [userWorking, setUserWorking] = useState(false);
    const timer = useRef(null);
    const replyTimer = useRef(null);
    const scroller = useRef(null);
    const atBottom = useRef(true);

    useEffect(() => {
      if (!playing) return;
      if (ptr >= frames.length - 1) { setPlaying(false); return; }
      timer.current = setTimeout(() => setPtr(p => Math.min(p + 1, frames.length - 1)), frames[ptr].dur / speed);
      return () => clearTimeout(timer.current);
    }, [ptr, playing, speed, frames]);

    useLayoutEffect(() => {
      const el = scroller.current; if (!el) return;
      if (atBottom.current) el.scrollTop = el.scrollHeight;
    }, [ptr, extra, userWorking]);

    function onScroll() {
      const el = scroller.current; if (!el) return;
      atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
    }

    function handleSubmit(text) {
      atBottom.current = true;
      setExtra(e => [...e, { ev: { type: 'user', text }, running: false }]);
      setUserWorking(true);
      clearTimeout(replyTimer.current);
      replyTimer.current = setTimeout(() => {
        setExtra(e => [...e, { ev: { type: 'assistant', text: "On it — I'll scope that out and make the change, then run the checks." }, running: false }]);
        setUserWorking(false);
      }, 1600);
    }

    const done = ptr >= frames.length - 1;
    const items = (frames[ptr] ? frames[ptr].items : []).concat(extra);
    const working = !done || userWorking;

    return (
      <div className="demo">
        <div className="transport">
          <button className="tbtn primary" onClick={() => { if (done) { setPtr(0); setPlaying(true); atBottom.current = true; } else setPlaying(p => !p); }}>
            {done ? '\u21BB replay' : playing ? '\u2759\u2759 pause' : '\u25B6 play'}
          </button>
          <button className="tbtn" onClick={() => { setPtr(0); setPlaying(false); atBottom.current = true; }}>{'\u23EE'} reset</button>
          <input className="scrub" type="range" min="0" max={frames.length - 1} value={ptr}
            onChange={e => { setPtr(+e.target.value); setPlaying(false); }} />
          <span className="frame-count">{ptr + 1}/{frames.length}</span>
          <div className="speeds">
            {[0.5, 1, 1.5, 3].map(s => (
              <button key={s} className={'tbtn sm' + (s === speed ? ' on' : '')} onClick={() => setSpeed(s)}>{s + '\u00D7'}</button>
            ))}
          </div>
        </div>
        <div className="canvas" ref={scroller} onScroll={onScroll}>
          <div className="canvas-inner">
            <Stream items={items} />
            {working && <div className="cursor-line"><window.Spinner /> <span>{'agent is working\u2026'}</span></div>}
          </div>
        </div>
        <window.LiveComposer working={working} onSubmit={handleSubmit} />
      </div>
    );
  }

  function Gallery() {
    return (
      <div className="canvas">
        <div className="canvas-inner gallery">
          <p className="gallery-intro">Every output type the renderer supports, in the current vibe / tool-style / density. Switch the controls above to recompose them all.</p>
          {window.GALLERY.map(g => (
            <div className="gallery-item" key={g.id}>
              <div className="gallery-label">{g.label}</div>
              <div className="gallery-block"><window.EventBlock ev={g.ev} running={false} /></div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  function App() {
    const [vibe, setVibe] = useState('slate');
    const [mode, setMode] = useState('dark');
    const [toolStyle, setToolStyle] = useState('card');
    const [density, setDensity] = useState('comfortable');
    const [tab, setTab] = useState('demo');

    const vars = window.buildVars(vibe, density, mode);
    const colors = window.paletteFor(vibe, mode);
    const ctx = { vibe, mode, toolStyle, density, colors };

    useEffect(() => { document.body.style.background = colors.bg; }, [colors.bg]);

    return (
      <window.ReplCtx.Provider value={ctx}>
        <div className="app" style={vars} data-toolstyle={toolStyle} data-vibe={vibe} data-mode={mode}>
          <header className="toolbar">
            <div className="tb-row tb-top">
              <div className="brand"><span className="brand-dot" /> agent<span className="brand-sep">/</span>repl</div>
              <div className="tb-top-right">
                <button className="mode-toggle" onClick={() => setMode(m => m === 'dark' ? 'light' : 'dark')} title="Toggle light / dark">
                  <span className="mode-glyph">{mode === 'dark' ? '\u263C' : '\u263E'}</span>
                  {mode === 'dark' ? 'Light' : 'Dark'}
                </button>
                <div className="tabs">
                  <button className={'tab' + (tab === 'demo' ? ' on' : '')} onClick={() => setTab('demo')}>Live demo</button>
                  <button className={'tab' + (tab === 'input' ? ' on' : '')} onClick={() => setTab('input')}>Input</button>
                  <button className={'tab' + (tab === 'gallery' ? ' on' : '')} onClick={() => setTab('gallery')}>Gallery</button>
                </div>
              </div>
            </div>
            <div className="tb-row tb-config">
              <Seg label="vibe" value={vibe} onChange={setVibe} options={Object.keys(window.VIBES).map(k => ({ v: k, label: window.VIBES[k].label }))} />
              <Seg label="tool style" value={toolStyle} onChange={setToolStyle} options={[{ v: 'inline', label: 'Inline' }, { v: 'card', label: 'Card' }, { v: 'collapsed', label: 'Collapsed' }]} />
              <Seg label="density" value={density} onChange={setDensity} options={[{ v: 'comfortable', label: 'Comfortable' }, { v: 'compact', label: 'Compact' }]} />
            </div>
          </header>
          {tab === 'demo' ? <LiveDemo /> : tab === 'input' ? <window.InputStates /> : <Gallery />}
          <div className="vibe-note">{window.VIBES[vibe].blurb}</div>
        </div>
      </window.ReplCtx.Provider>
    );
  }

  ReactDOM.createRoot(document.getElementById('root')).render(<App />);
})();
