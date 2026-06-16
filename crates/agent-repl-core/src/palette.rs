//! Color palettes per (vibe, mode). Source of truth: `docs/repl/tokens.jsx`
//! `VIBES` table. Values are stored as raw oklch triples and converted to
//! sRGB at theme-build time; the result is what the renderer hands to ratatui.

use palette::{FromColor, Oklch, Srgb};

use crate::theme::{Mode, Vibe};
use crate::tools::HueToken;

/// 8-bit sRGB triple, ready for terminal rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const BLACK: Self = Self(0, 0, 0);

    pub fn from_oklch(l: f32, c: f32, h: f32) -> Self {
        let oklch = Oklch::new(l, c, h);
        let srgb = Srgb::from_color(oklch);
        let r = (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self(r, g, b)
    }
}

/// Mix two sRGB colors in oklab space at ratio `t` (0.0 → a, 1.0 → b).
/// Mirrors the design's `color-mix(in oklch, …)` calls used for soft fills.
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let sa = Srgb::new(a.0 as f32 / 255.0, a.1 as f32 / 255.0, a.2 as f32 / 255.0);
    let sb = Srgb::new(b.0 as f32 / 255.0, b.1 as f32 / 255.0, b.2 as f32 / 255.0);
    let la: palette::Oklab = palette::Oklab::from_color(sa);
    let lb: palette::Oklab = palette::Oklab::from_color(sb);
    let lerp = palette::Oklab::new(
        la.l + (lb.l - la.l) * t,
        la.a + (lb.a - la.a) * t,
        la.b + (lb.b - la.b) * t,
    );
    let srgb = Srgb::from_color(lerp);
    Rgb(
        (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

/// One full resolved palette (21 tokens). Layout mirrors `tokens.jsx` 1:1.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Rgb,
    pub bg_raised: Rgb,
    pub bg_inset: Rgb,
    pub border: Rgb,
    pub border_strong: Rgb,
    pub text: Rgb,
    pub text_dim: Rgb,
    pub text_faint: Rgb,
    pub accent: Rgb,
    pub accent_soft: Rgb,
    pub success: Rgb,
    pub danger: Rgb,
    pub warning: Rgb,
    pub info: Rgb,
    pub t_read: Rgb,
    pub t_edit: Rgb,
    pub t_bash: Rgb,
    pub t_search: Rgb,
    pub t_list: Rgb,
    pub t_todo: Rgb,
    pub t_web: Rgb,
}

impl Palette {
    pub fn hue(&self, token: HueToken) -> Rgb {
        match token {
            HueToken::TRead => self.t_read,
            HueToken::TEdit => self.t_edit,
            HueToken::TBash => self.t_bash,
            HueToken::TSearch => self.t_search,
            HueToken::TList => self.t_list,
            HueToken::TTodo => self.t_todo,
            HueToken::TWeb => self.t_web,
        }
    }
}

/// Raw oklch triples for a palette. Built up as a const table so the
/// resolved [`Palette`] is just a (cheap) one-time conversion.
struct LchTable {
    bg: (f32, f32, f32),
    bg_raised: (f32, f32, f32),
    bg_inset: (f32, f32, f32),
    border: (f32, f32, f32),
    border_strong: (f32, f32, f32),
    text: (f32, f32, f32),
    text_dim: (f32, f32, f32),
    text_faint: (f32, f32, f32),
    accent: (f32, f32, f32),
    accent_soft: (f32, f32, f32),
    success: (f32, f32, f32),
    danger: (f32, f32, f32),
    warning: (f32, f32, f32),
    info: (f32, f32, f32),
    t_read: (f32, f32, f32),
    t_edit: (f32, f32, f32),
    t_bash: (f32, f32, f32),
    t_search: (f32, f32, f32),
    t_list: (f32, f32, f32),
    t_todo: (f32, f32, f32),
    t_web: (f32, f32, f32),
}

impl LchTable {
    fn resolve(&self) -> Palette {
        let f = |(l, c, h): (f32, f32, f32)| Rgb::from_oklch(l, c, h);
        Palette {
            bg: f(self.bg),
            bg_raised: f(self.bg_raised),
            bg_inset: f(self.bg_inset),
            border: f(self.border),
            border_strong: f(self.border_strong),
            text: f(self.text),
            text_dim: f(self.text_dim),
            text_faint: f(self.text_faint),
            accent: f(self.accent),
            accent_soft: f(self.accent_soft),
            success: f(self.success),
            danger: f(self.danger),
            warning: f(self.warning),
            info: f(self.info),
            t_read: f(self.t_read),
            t_edit: f(self.t_edit),
            t_bash: f(self.t_bash),
            t_search: f(self.t_search),
            t_list: f(self.t_list),
            t_todo: f(self.t_todo),
            t_web: f(self.t_web),
        }
    }
}

pub fn palette_for(vibe: Vibe, mode: Mode) -> Palette {
    let table = match (vibe, mode) {
        (Vibe::Phosphor, Mode::Dark) => &PHOSPHOR_DARK,
        (Vibe::Phosphor, Mode::Light) => &PHOSPHOR_LIGHT,
        (Vibe::Slate, Mode::Dark) => &SLATE_DARK,
        (Vibe::Slate, Mode::Light) => &SLATE_LIGHT,
        (Vibe::Spectrum, Mode::Dark) => &SPECTRUM_DARK,
        (Vibe::Spectrum, Mode::Light) => &SPECTRUM_LIGHT,
        (Vibe::Ember, Mode::Dark) => &EMBER_DARK,
        (Vibe::Ember, Mode::Light) => &EMBER_LIGHT,
    };
    table.resolve()
}

// ---------------------------------------------------------------------------
// Vibe tables. Values are a direct port of `docs/repl/tokens.jsx` (VIBES).
// Numeric form: (lightness 0–1, chroma, hue in degrees 0–360).
// ---------------------------------------------------------------------------

const PHOSPHOR_DARK: LchTable = LchTable {
    bg: (0.155, 0.012, 150.0),
    bg_raised: (0.2, 0.016, 150.0),
    bg_inset: (0.125, 0.012, 150.0),
    border: (0.3, 0.024, 150.0),
    border_strong: (0.44, 0.05, 150.0),
    text: (0.87, 0.06, 150.0),
    text_dim: (0.66, 0.05, 150.0),
    text_faint: (0.5, 0.04, 150.0),
    accent: (0.84, 0.18, 145.0),
    accent_soft: (0.32, 0.07, 145.0),
    success: (0.84, 0.18, 145.0),
    danger: (0.7, 0.2, 26.0),
    warning: (0.84, 0.15, 82.0),
    info: (0.8, 0.11, 195.0),
    t_read: (0.8, 0.11, 195.0),
    t_edit: (0.84, 0.18, 145.0),
    t_bash: (0.84, 0.15, 82.0),
    t_search: (0.78, 0.1, 175.0),
    t_list: (0.7, 0.06, 150.0),
    t_todo: (0.84, 0.18, 145.0),
    t_web: (0.8, 0.11, 195.0),
};

const PHOSPHOR_LIGHT: LchTable = LchTable {
    bg: (0.96, 0.012, 110.0),
    bg_raised: (0.985, 0.009, 110.0),
    bg_inset: (0.93, 0.015, 115.0),
    border: (0.86, 0.022, 120.0),
    border_strong: (0.72, 0.04, 130.0),
    text: (0.32, 0.05, 150.0),
    text_dim: (0.48, 0.06, 150.0),
    text_faint: (0.62, 0.05, 145.0),
    accent: (0.5, 0.15, 150.0),
    accent_soft: (0.92, 0.05, 150.0),
    success: (0.5, 0.15, 150.0),
    danger: (0.52, 0.19, 28.0),
    warning: (0.56, 0.13, 75.0),
    info: (0.5, 0.1, 200.0),
    t_read: (0.5, 0.1, 200.0),
    t_edit: (0.5, 0.15, 150.0),
    t_bash: (0.55, 0.13, 78.0),
    t_search: (0.5, 0.09, 175.0),
    t_list: (0.5, 0.06, 150.0),
    t_todo: (0.5, 0.15, 150.0),
    t_web: (0.5, 0.1, 200.0),
};

const SLATE_DARK: LchTable = LchTable {
    bg: (0.172, 0.006, 255.0),
    bg_raised: (0.212, 0.009, 255.0),
    bg_inset: (0.142, 0.006, 255.0),
    border: (0.285, 0.012, 255.0),
    border_strong: (0.4, 0.022, 255.0),
    text: (0.9, 0.005, 255.0),
    text_dim: (0.69, 0.01, 255.0),
    text_faint: (0.52, 0.012, 255.0),
    accent: (0.66, 0.15, 256.0),
    accent_soft: (0.3, 0.07, 256.0),
    success: (0.72, 0.14, 155.0),
    danger: (0.66, 0.19, 22.0),
    warning: (0.79, 0.13, 75.0),
    info: (0.7, 0.12, 235.0),
    t_read: (0.7, 0.12, 235.0),
    t_edit: (0.72, 0.14, 155.0),
    t_bash: (0.79, 0.13, 75.0),
    t_search: (0.68, 0.14, 300.0),
    t_list: (0.7, 0.1, 210.0),
    t_todo: (0.72, 0.13, 340.0),
    t_web: (0.72, 0.12, 195.0),
};

const SLATE_LIGHT: LchTable = LchTable {
    bg: (0.975, 0.004, 255.0),
    bg_raised: (0.997, 0.001, 255.0),
    bg_inset: (0.945, 0.006, 255.0),
    border: (0.9, 0.008, 255.0),
    border_strong: (0.8, 0.014, 255.0),
    text: (0.28, 0.016, 262.0),
    text_dim: (0.48, 0.015, 260.0),
    text_faint: (0.62, 0.013, 258.0),
    accent: (0.52, 0.17, 262.0),
    accent_soft: (0.94, 0.03, 262.0),
    success: (0.54, 0.15, 150.0),
    danger: (0.55, 0.2, 25.0),
    warning: (0.6, 0.13, 68.0),
    info: (0.54, 0.13, 240.0),
    t_read: (0.54, 0.14, 240.0),
    t_edit: (0.52, 0.14, 150.0),
    t_bash: (0.55, 0.13, 68.0),
    t_search: (0.52, 0.16, 300.0),
    t_list: (0.5, 0.1, 215.0),
    t_todo: (0.55, 0.15, 345.0),
    t_web: (0.54, 0.12, 197.0),
};

const SPECTRUM_DARK: LchTable = LchTable {
    bg: (0.178, 0.014, 285.0),
    bg_raised: (0.225, 0.018, 285.0),
    bg_inset: (0.145, 0.013, 285.0),
    border: (0.305, 0.022, 285.0),
    border_strong: (0.43, 0.04, 285.0),
    text: (0.92, 0.006, 285.0),
    text_dim: (0.71, 0.014, 285.0),
    text_faint: (0.55, 0.022, 285.0),
    accent: (0.7, 0.17, 300.0),
    accent_soft: (0.33, 0.08, 300.0),
    success: (0.74, 0.16, 150.0),
    danger: (0.66, 0.2, 25.0),
    warning: (0.81, 0.15, 85.0),
    info: (0.74, 0.14, 235.0),
    t_read: (0.72, 0.16, 240.0),
    t_edit: (0.74, 0.16, 150.0),
    t_bash: (0.81, 0.15, 85.0),
    t_search: (0.7, 0.18, 305.0),
    t_list: (0.74, 0.14, 200.0),
    t_todo: (0.72, 0.17, 345.0),
    t_web: (0.76, 0.15, 190.0),
};

const SPECTRUM_LIGHT: LchTable = LchTable {
    bg: (0.98, 0.006, 290.0),
    bg_raised: (0.998, 0.002, 290.0),
    bg_inset: (0.955, 0.01, 290.0),
    border: (0.9, 0.014, 290.0),
    border_strong: (0.8, 0.026, 290.0),
    text: (0.29, 0.025, 290.0),
    text_dim: (0.5, 0.022, 290.0),
    text_faint: (0.64, 0.018, 290.0),
    accent: (0.55, 0.21, 300.0),
    accent_soft: (0.95, 0.04, 300.0),
    success: (0.55, 0.17, 150.0),
    danger: (0.56, 0.21, 25.0),
    warning: (0.6, 0.15, 75.0),
    info: (0.55, 0.16, 245.0),
    t_read: (0.55, 0.18, 255.0),
    t_edit: (0.54, 0.16, 150.0),
    t_bash: (0.57, 0.15, 72.0),
    t_search: (0.54, 0.2, 305.0),
    t_list: (0.53, 0.15, 212.0),
    t_todo: (0.55, 0.19, 350.0),
    t_web: (0.55, 0.16, 200.0),
};

const EMBER_DARK: LchTable = LchTable {
    bg: (0.165, 0.014, 50.0),
    bg_raised: (0.212, 0.017, 52.0),
    bg_inset: (0.135, 0.013, 48.0),
    border: (0.3, 0.024, 55.0),
    border_strong: (0.42, 0.04, 55.0),
    text: (0.91, 0.014, 65.0),
    text_dim: (0.7, 0.022, 60.0),
    text_faint: (0.55, 0.026, 55.0),
    accent: (0.74, 0.15, 55.0),
    accent_soft: (0.34, 0.07, 55.0),
    success: (0.74, 0.13, 140.0),
    danger: (0.65, 0.19, 28.0),
    warning: (0.81, 0.14, 70.0),
    info: (0.72, 0.1, 220.0),
    t_read: (0.72, 0.1, 220.0),
    t_edit: (0.74, 0.13, 140.0),
    t_bash: (0.78, 0.14, 60.0),
    t_search: (0.7, 0.13, 330.0),
    t_list: (0.74, 0.11, 95.0),
    t_todo: (0.74, 0.13, 20.0),
    t_web: (0.72, 0.11, 200.0),
};

const EMBER_LIGHT: LchTable = LchTable {
    bg: (0.97, 0.014, 75.0),
    bg_raised: (0.992, 0.009, 75.0),
    bg_inset: (0.945, 0.018, 70.0),
    border: (0.89, 0.024, 65.0),
    border_strong: (0.78, 0.04, 60.0),
    text: (0.3, 0.028, 50.0),
    text_dim: (0.48, 0.032, 52.0),
    text_faint: (0.62, 0.03, 55.0),
    accent: (0.56, 0.16, 52.0),
    accent_soft: (0.93, 0.04, 62.0),
    success: (0.52, 0.13, 140.0),
    danger: (0.54, 0.19, 30.0),
    warning: (0.58, 0.13, 62.0),
    info: (0.52, 0.1, 220.0),
    t_read: (0.52, 0.1, 220.0),
    t_edit: (0.52, 0.13, 140.0),
    t_bash: (0.56, 0.14, 60.0),
    t_search: (0.52, 0.13, 332.0),
    t_list: (0.54, 0.11, 92.0),
    t_todo: (0.54, 0.15, 28.0),
    t_web: (0.52, 0.11, 202.0),
};
