// tokens.jsx — vibe / mode / density / tool token system for the Agent REPL renderer
// Loaded first. Exposes VIBES, DENSITIES, TOOLS, buildVars, ReplCtx on window.

const ReplCtx = React.createContext({ vibe: 'slate', mode: 'dark', toolStyle: 'card', density: 'comfortable', colors: {} });

// Each vibe is a color *strategy* with a dark + light palette:
//   phosphor = minimal (status-only color), slate = restrained semantic,
//   spectrum = rich (a hue per tool), ember = warm & friendly.
const VIBES = {
  phosphor: {
    label: 'Phosphor',
    blurb: 'Classic CRT terminal. All-mono, minimal color \u2014 green/amber on black, dark ink on paper.',
    prose: 'mono',
    radius: 2,
    dark: {
      bg: 'oklch(0.155 0.012 150)', bgRaised: 'oklch(0.2 0.016 150)', bgInset: 'oklch(0.125 0.012 150)',
      border: 'oklch(0.3 0.024 150)', borderStrong: 'oklch(0.44 0.05 150)',
      text: 'oklch(0.87 0.06 150)', textDim: 'oklch(0.66 0.05 150)', textFaint: 'oklch(0.5 0.04 150)',
      accent: 'oklch(0.84 0.18 145)', accentSoft: 'oklch(0.32 0.07 145)',
      success: 'oklch(0.84 0.18 145)', danger: 'oklch(0.7 0.2 26)', warning: 'oklch(0.84 0.15 82)', info: 'oklch(0.8 0.11 195)',
      tRead: 'oklch(0.8 0.11 195)', tEdit: 'oklch(0.84 0.18 145)', tBash: 'oklch(0.84 0.15 82)',
      tSearch: 'oklch(0.78 0.1 175)', tList: 'oklch(0.7 0.06 150)', tTodo: 'oklch(0.84 0.18 145)', tWeb: 'oklch(0.8 0.11 195)',
    },
    light: {
      bg: 'oklch(0.96 0.012 110)', bgRaised: 'oklch(0.985 0.009 110)', bgInset: 'oklch(0.93 0.015 115)',
      border: 'oklch(0.86 0.022 120)', borderStrong: 'oklch(0.72 0.04 130)',
      text: 'oklch(0.32 0.05 150)', textDim: 'oklch(0.48 0.06 150)', textFaint: 'oklch(0.62 0.05 145)',
      accent: 'oklch(0.5 0.15 150)', accentSoft: 'oklch(0.92 0.05 150)',
      success: 'oklch(0.5 0.15 150)', danger: 'oklch(0.52 0.19 28)', warning: 'oklch(0.56 0.13 75)', info: 'oklch(0.5 0.1 200)',
      tRead: 'oklch(0.5 0.1 200)', tEdit: 'oklch(0.5 0.15 150)', tBash: 'oklch(0.55 0.13 78)',
      tSearch: 'oklch(0.5 0.09 175)', tList: 'oklch(0.5 0.06 150)', tTodo: 'oklch(0.5 0.15 150)', tWeb: 'oklch(0.5 0.1 200)',
    },
  },
  slate: {
    label: 'Slate',
    blurb: 'Modern dev tool. Cool neutrals, one indigo accent, restrained semantic color.',
    prose: 'sans',
    radius: 8,
    dark: {
      bg: 'oklch(0.172 0.006 255)', bgRaised: 'oklch(0.212 0.009 255)', bgInset: 'oklch(0.142 0.006 255)',
      border: 'oklch(0.285 0.012 255)', borderStrong: 'oklch(0.4 0.022 255)',
      text: 'oklch(0.9 0.005 255)', textDim: 'oklch(0.69 0.01 255)', textFaint: 'oklch(0.52 0.012 255)',
      accent: 'oklch(0.66 0.15 256)', accentSoft: 'oklch(0.3 0.07 256)',
      success: 'oklch(0.72 0.14 155)', danger: 'oklch(0.66 0.19 22)', warning: 'oklch(0.79 0.13 75)', info: 'oklch(0.7 0.12 235)',
      tRead: 'oklch(0.7 0.12 235)', tEdit: 'oklch(0.72 0.14 155)', tBash: 'oklch(0.79 0.13 75)',
      tSearch: 'oklch(0.68 0.14 300)', tList: 'oklch(0.7 0.1 210)', tTodo: 'oklch(0.72 0.13 340)', tWeb: 'oklch(0.72 0.12 195)',
    },
    light: {
      bg: 'oklch(0.975 0.004 255)', bgRaised: 'oklch(0.997 0.001 255)', bgInset: 'oklch(0.945 0.006 255)',
      border: 'oklch(0.9 0.008 255)', borderStrong: 'oklch(0.8 0.014 255)',
      text: 'oklch(0.28 0.016 262)', textDim: 'oklch(0.48 0.015 260)', textFaint: 'oklch(0.62 0.013 258)',
      accent: 'oklch(0.52 0.17 262)', accentSoft: 'oklch(0.94 0.03 262)',
      success: 'oklch(0.54 0.15 150)', danger: 'oklch(0.55 0.2 25)', warning: 'oklch(0.6 0.13 68)', info: 'oklch(0.54 0.13 240)',
      tRead: 'oklch(0.54 0.14 240)', tEdit: 'oklch(0.52 0.14 150)', tBash: 'oklch(0.55 0.13 68)',
      tSearch: 'oklch(0.52 0.16 300)', tList: 'oklch(0.5 0.1 215)', tTodo: 'oklch(0.55 0.15 345)', tWeb: 'oklch(0.54 0.12 197)',
    },
  },
  spectrum: {
    label: 'Spectrum',
    blurb: 'Rich & semantic. A distinct vivid hue per tool category, soft accent fills.',
    prose: 'sans',
    radius: 10,
    dark: {
      bg: 'oklch(0.178 0.014 285)', bgRaised: 'oklch(0.225 0.018 285)', bgInset: 'oklch(0.145 0.013 285)',
      border: 'oklch(0.305 0.022 285)', borderStrong: 'oklch(0.43 0.04 285)',
      text: 'oklch(0.92 0.006 285)', textDim: 'oklch(0.71 0.014 285)', textFaint: 'oklch(0.55 0.022 285)',
      accent: 'oklch(0.7 0.17 300)', accentSoft: 'oklch(0.33 0.08 300)',
      success: 'oklch(0.74 0.16 150)', danger: 'oklch(0.66 0.2 25)', warning: 'oklch(0.81 0.15 85)', info: 'oklch(0.74 0.14 235)',
      tRead: 'oklch(0.72 0.16 240)', tEdit: 'oklch(0.74 0.16 150)', tBash: 'oklch(0.81 0.15 85)',
      tSearch: 'oklch(0.7 0.18 305)', tList: 'oklch(0.74 0.14 200)', tTodo: 'oklch(0.72 0.17 345)', tWeb: 'oklch(0.76 0.15 190)',
    },
    light: {
      bg: 'oklch(0.98 0.006 290)', bgRaised: 'oklch(0.998 0.002 290)', bgInset: 'oklch(0.955 0.01 290)',
      border: 'oklch(0.9 0.014 290)', borderStrong: 'oklch(0.8 0.026 290)',
      text: 'oklch(0.29 0.025 290)', textDim: 'oklch(0.5 0.022 290)', textFaint: 'oklch(0.64 0.018 290)',
      accent: 'oklch(0.55 0.21 300)', accentSoft: 'oklch(0.95 0.04 300)',
      success: 'oklch(0.55 0.17 150)', danger: 'oklch(0.56 0.21 25)', warning: 'oklch(0.6 0.15 75)', info: 'oklch(0.55 0.16 245)',
      tRead: 'oklch(0.55 0.18 255)', tEdit: 'oklch(0.54 0.16 150)', tBash: 'oklch(0.57 0.15 72)',
      tSearch: 'oklch(0.54 0.2 305)', tList: 'oklch(0.53 0.15 212)', tTodo: 'oklch(0.55 0.19 350)', tWeb: 'oklch(0.55 0.16 200)',
    },
  },
  ember: {
    label: 'Ember',
    blurb: 'Warm & friendly. Amber accent, rounded and approachable \u2014 brown-black or cream paper.',
    prose: 'sans',
    radius: 13,
    dark: {
      bg: 'oklch(0.165 0.014 50)', bgRaised: 'oklch(0.212 0.017 52)', bgInset: 'oklch(0.135 0.013 48)',
      border: 'oklch(0.3 0.024 55)', borderStrong: 'oklch(0.42 0.04 55)',
      text: 'oklch(0.91 0.014 65)', textDim: 'oklch(0.7 0.022 60)', textFaint: 'oklch(0.55 0.026 55)',
      accent: 'oklch(0.74 0.15 55)', accentSoft: 'oklch(0.34 0.07 55)',
      success: 'oklch(0.74 0.13 140)', danger: 'oklch(0.65 0.19 28)', warning: 'oklch(0.81 0.14 70)', info: 'oklch(0.72 0.1 220)',
      tRead: 'oklch(0.72 0.1 220)', tEdit: 'oklch(0.74 0.13 140)', tBash: 'oklch(0.78 0.14 60)',
      tSearch: 'oklch(0.7 0.13 330)', tList: 'oklch(0.74 0.11 95)', tTodo: 'oklch(0.74 0.13 20)', tWeb: 'oklch(0.72 0.11 200)',
    },
    light: {
      bg: 'oklch(0.97 0.014 75)', bgRaised: 'oklch(0.992 0.009 75)', bgInset: 'oklch(0.945 0.018 70)',
      border: 'oklch(0.89 0.024 65)', borderStrong: 'oklch(0.78 0.04 60)',
      text: 'oklch(0.3 0.028 50)', textDim: 'oklch(0.48 0.032 52)', textFaint: 'oklch(0.62 0.03 55)',
      accent: 'oklch(0.56 0.16 52)', accentSoft: 'oklch(0.93 0.04 62)',
      success: 'oklch(0.52 0.13 140)', danger: 'oklch(0.54 0.19 30)', warning: 'oklch(0.58 0.13 62)', info: 'oklch(0.52 0.1 220)',
      tRead: 'oklch(0.52 0.1 220)', tEdit: 'oklch(0.52 0.13 140)', tBash: 'oklch(0.56 0.14 60)',
      tSearch: 'oklch(0.52 0.13 332)', tList: 'oklch(0.54 0.11 92)', tTodo: 'oklch(0.54 0.15 28)', tWeb: 'oklch(0.52 0.11 202)',
    },
  },
};

const DENSITIES = {
  comfortable: { label: 'Comfortable', fs: '14px', fsSm: '12.5px', fsXs: '11px', lh: '1.62', gap: '20px', padY: '11px', padX: '14px', blockPadY: '13px', headGap: '9px' },
  compact:     { label: 'Compact',     fs: '12.5px', fsSm: '11.5px', fsXs: '10px', lh: '1.45', gap: '9px',  padY: '6px',  padX: '10px', blockPadY: '7px',  headGap: '7px' },
};

// Tool metadata: label shown + which color token drives its hue.
const TOOLS = {
  read:   { label: 'read',   hue: 'tRead' },
  write:  { label: 'write',  hue: 'tEdit' },
  edit:   { label: 'edit',   hue: 'tEdit' },
  bash:   { label: 'bash',   hue: 'tBash' },
  search: { label: 'search', hue: 'tSearch' },
  list:   { label: 'list',   hue: 'tList' },
  todo:   { label: 'todo',   hue: 'tTodo' },
  web:    { label: 'fetch',  hue: 'tWeb' },
};

const MONO = "'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, Consolas, monospace";
const SANS = "'IBM Plex Sans', ui-sans-serif, system-ui, -apple-system, sans-serif";

function camelToKebab(s) { return s.replace(/[A-Z]/g, m => '-' + m.toLowerCase()); }

function paletteFor(vibeKey, mode) {
  const v = VIBES[vibeKey] || VIBES.slate;
  return (mode === 'light' ? v.light : v.dark);
}

// Build the CSS-variable style object applied to the root container.
function buildVars(vibeKey, densKey, mode) {
  const v = VIBES[vibeKey] || VIBES.slate;
  const d = DENSITIES[densKey] || DENSITIES.comfortable;
  const colors = paletteFor(vibeKey, mode);
  const out = {};
  for (const [k, val] of Object.entries(colors)) out['--' + camelToKebab(k)] = val;
  out['--radius'] = v.radius + 'px';
  out['--font-mono'] = MONO;
  out['--font-sans'] = SANS;
  out['--font-prose'] = v.prose === 'mono' ? MONO : SANS;
  out['--fs'] = d.fs; out['--fs-sm'] = d.fsSm; out['--fs-xs'] = d.fsXs;
  out['--lh'] = d.lh; out['--gap'] = d.gap;
  out['--pad-y'] = d.padY; out['--pad-x'] = d.padX; out['--block-pad-y'] = d.blockPadY; out['--head-gap'] = d.headGap;
  return out;
}

Object.assign(window, { ReplCtx, VIBES, DENSITIES, TOOLS, TOOLS_KEYS: Object.keys(TOOLS), buildVars, paletteFor, MONO, SANS });
