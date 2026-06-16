//! `Theme` = (vibe, mode, tool_style, density) + the resolved palette and
//! shape/density specs derived from those choices. Mirrors `buildVars`
//! and `paletteFor` in `docs/repl/tokens.jsx`.

use crate::palette::{palette_for, Palette};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vibe {
    Phosphor,
    Slate,
    Spectrum,
    Ember,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolStyle {
    Inline,
    Card,
    Collapsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Density {
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProseFont {
    Mono,
    Sans,
}

#[derive(Debug, Clone, Copy)]
pub struct VibeInfo {
    pub vibe: Vibe,
    pub label: &'static str,
    pub blurb: &'static str,
    pub prose: ProseFont,
    /// Border radius in CSS px (informational — terminals don't round).
    pub radius: u8,
}

pub const fn vibe_info(vibe: Vibe) -> VibeInfo {
    match vibe {
        Vibe::Phosphor => VibeInfo {
            vibe,
            label: "Phosphor",
            blurb: "Classic CRT terminal. All-mono, minimal color.",
            prose: ProseFont::Mono,
            radius: 2,
        },
        Vibe::Slate => VibeInfo {
            vibe,
            label: "Slate",
            blurb: "Modern dev tool. Cool neutrals, one indigo accent.",
            prose: ProseFont::Sans,
            radius: 8,
        },
        Vibe::Spectrum => VibeInfo {
            vibe,
            label: "Spectrum",
            blurb: "Rich & semantic. A distinct hue per tool category.",
            prose: ProseFont::Sans,
            radius: 10,
        },
        Vibe::Ember => VibeInfo {
            vibe,
            label: "Ember",
            blurb: "Warm & friendly. Amber accent, approachable.",
            prose: ProseFont::Sans,
            radius: 13,
        },
    }
}

/// Spacing + type-scale spec for a density. Numeric units are TUI cells
/// where applicable; the terminal renderer interprets them as line/column
/// gaps rather than pixels.
#[derive(Debug, Clone, Copy)]
pub struct DensitySpec {
    pub density: Density,
    pub label: &'static str,
    /// Blank lines between top-level blocks.
    pub gap: u16,
    /// Inner vertical padding (lines) inside a card body.
    pub pad_y: u16,
    /// Inner horizontal padding (columns) inside a card body.
    pub pad_x: u16,
    /// Vertical padding on the tool header row.
    pub block_pad_y: u16,
    /// Gap between the tool dot/name/title parts in the header.
    pub head_gap: u16,
}

pub const fn density_spec(density: Density) -> DensitySpec {
    match density {
        Density::Comfortable => DensitySpec {
            density,
            label: "Comfortable",
            gap: 1,
            pad_y: 1,
            pad_x: 2,
            block_pad_y: 0,
            head_gap: 1,
        },
        Density::Compact => DensitySpec {
            density,
            label: "Compact",
            gap: 0,
            pad_y: 0,
            pad_x: 1,
            block_pad_y: 0,
            head_gap: 1,
        },
    }
}

/// The composed theme. Build with `Theme::new(Vibe::Slate).dark().card().compact()`.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub vibe: Vibe,
    pub mode: Mode,
    pub tool_style: ToolStyle,
    pub density: Density,
    pub palette: Palette,
    pub info: VibeInfo,
    pub spacing: DensitySpec,
}

impl Theme {
    pub fn new(vibe: Vibe) -> Self {
        let mode = Mode::Dark;
        let tool_style = ToolStyle::Card;
        let density = Density::Comfortable;
        Self {
            vibe,
            mode,
            tool_style,
            density,
            palette: palette_for(vibe, mode),
            info: vibe_info(vibe),
            spacing: density_spec(density),
        }
    }

    // -- vibe convenience constructors -----------------------------------
    pub fn phosphor() -> Self { Self::new(Vibe::Phosphor) }
    pub fn slate()    -> Self { Self::new(Vibe::Slate) }
    pub fn spectrum() -> Self { Self::new(Vibe::Spectrum) }
    pub fn ember()    -> Self { Self::new(Vibe::Ember) }

    // -- mode -------------------------------------------------------------
    pub fn dark(mut self) -> Self {
        self.mode = Mode::Dark;
        self.palette = palette_for(self.vibe, self.mode);
        self
    }
    pub fn light(mut self) -> Self {
        self.mode = Mode::Light;
        self.palette = palette_for(self.vibe, self.mode);
        self
    }
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self.palette = palette_for(self.vibe, self.mode);
        self
    }

    // -- tool style -------------------------------------------------------
    pub fn inline(mut self) -> Self { self.tool_style = ToolStyle::Inline; self }
    pub fn card(mut self) -> Self { self.tool_style = ToolStyle::Card; self }
    pub fn collapsed(mut self) -> Self { self.tool_style = ToolStyle::Collapsed; self }
    pub fn with_tool_style(mut self, style: ToolStyle) -> Self {
        self.tool_style = style;
        self
    }

    // -- density ----------------------------------------------------------
    pub fn comfortable(mut self) -> Self {
        self.density = Density::Comfortable;
        self.spacing = density_spec(self.density);
        self
    }
    pub fn compact(mut self) -> Self {
        self.density = Density::Compact;
        self.spacing = density_spec(self.density);
        self
    }
    pub fn with_density(mut self, density: Density) -> Self {
        self.density = density;
        self.spacing = density_spec(self.density);
        self
    }

    // -- cycling helpers (used by the demo toolbar) -----------------------
    pub fn cycle_vibe(self) -> Self {
        let next = match self.vibe {
            Vibe::Phosphor => Vibe::Slate,
            Vibe::Slate => Vibe::Spectrum,
            Vibe::Spectrum => Vibe::Ember,
            Vibe::Ember => Vibe::Phosphor,
        };
        Self::new(next)
            .with_mode(self.mode)
            .with_tool_style(self.tool_style)
            .with_density(self.density)
    }
    pub fn toggle_mode(self) -> Self {
        let next = match self.mode { Mode::Dark => Mode::Light, Mode::Light => Mode::Dark };
        self.with_mode(next)
    }
    pub fn cycle_tool_style(self) -> Self {
        let next = match self.tool_style {
            ToolStyle::Inline => ToolStyle::Card,
            ToolStyle::Card => ToolStyle::Collapsed,
            ToolStyle::Collapsed => ToolStyle::Inline,
        };
        self.with_tool_style(next)
    }
    pub fn toggle_density(self) -> Self {
        let next = match self.density {
            Density::Comfortable => Density::Compact,
            Density::Compact => Density::Comfortable,
        };
        self.with_density(next)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::slate()
    }
}
