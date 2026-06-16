// markdown.jsx — tiny, dependency-free markdown -> React for assistant prose.
// Supports: ### headings, **bold**, *italic*, `code`, [link](url),
// - / * bullet lists, 1. ordered lists, ```fenced code```, blockquotes, paragraphs.

(function () {
  let _k = 0;
  const key = () => 'md' + (_k++);

  // Inline: **bold**, *italic*, `code`, [text](url)
  function inline(text) {
    const out = [];
    const re = /(\*\*([^*]+)\*\*)|(`([^`]+)`)|(\*([^*]+)\*)|(\[([^\]]+)\]\(([^)]+)\))/g;
    let last = 0, m;
    while ((m = re.exec(text))) {
      if (m.index > last) out.push(text.slice(last, m.index));
      if (m[1]) out.push(<strong key={key()}>{m[2]}</strong>);
      else if (m[3]) out.push(<code key={key()} className="md-code">{m[4]}</code>);
      else if (m[5]) out.push(<em key={key()}>{m[6]}</em>);
      else if (m[7]) out.push(<a key={key()} className="md-link" href={m[9]} onClick={e => e.preventDefault()}>{m[8]}</a>);
      last = re.lastIndex;
    }
    if (last < text.length) out.push(text.slice(last));
    return out;
  }

  function render(src) {
    const lines = (src || '').replace(/\t/g, '  ').split('\n');
    const blocks = [];
    let i = 0;
    while (i < lines.length) {
      let line = lines[i];
      // fenced code
      if (/^```/.test(line)) {
        const lang = line.slice(3).trim();
        const buf = [];
        i++;
        while (i < lines.length && !/^```/.test(lines[i])) { buf.push(lines[i]); i++; }
        i++;
        blocks.push(<pre key={key()} className="md-pre" data-lang={lang}><code>{buf.join('\n')}</code></pre>);
        continue;
      }
      // heading
      const h = line.match(/^(#{1,4})\s+(.*)$/);
      if (h) { const L = h[1].length; blocks.push(React.createElement('div', { key: key(), className: 'md-h md-h' + L }, inline(h[2]))); i++; continue; }
      // blockquote
      if (/^>\s?/.test(line)) {
        const buf = [];
        while (i < lines.length && /^>\s?/.test(lines[i])) { buf.push(lines[i].replace(/^>\s?/, '')); i++; }
        blocks.push(<blockquote key={key()} className="md-quote">{inline(buf.join(' '))}</blockquote>);
        continue;
      }
      // unordered list
      if (/^\s*[-*]\s+/.test(line)) {
        const items = [];
        while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) { items.push(lines[i].replace(/^\s*[-*]\s+/, '')); i++; }
        blocks.push(<ul key={key()} className="md-ul">{items.map(t => <li key={key()}>{inline(t)}</li>)}</ul>);
        continue;
      }
      // ordered list
      if (/^\s*\d+\.\s+/.test(line)) {
        const items = [];
        while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) { items.push(lines[i].replace(/^\s*\d+\.\s+/, '')); i++; }
        blocks.push(<ol key={key()} className="md-ol">{items.map(t => <li key={key()}>{inline(t)}</li>)}</ol>);
        continue;
      }
      // blank
      if (/^\s*$/.test(line)) { i++; continue; }
      // paragraph (gather until blank / block start)
      const buf = [line]; i++;
      while (i < lines.length && !/^\s*$/.test(lines[i]) && !/^(```|#{1,4}\s|>\s?|\s*[-*]\s|\s*\d+\.\s)/.test(lines[i])) { buf.push(lines[i]); i++; }
      blocks.push(<p key={key()} className="md-p">{inline(buf.join(' '))}</p>);
    }
    return blocks;
  }

  function Markdown({ children }) {
    return <div className="prose">{render(children)}</div>;
  }

  Object.assign(window, { Markdown });
})();
