//! Helpers to convert core `Rgb` to ratatui `Color` / `Style`.

use agent_repl_core::Rgb;
use ratatui::style::{Color, Modifier, Style};

pub fn color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

pub fn fg(rgb: Rgb) -> Style {
    Style::default().fg(color(rgb))
}

pub fn fg_bold(rgb: Rgb) -> Style {
    Style::default().fg(color(rgb)).add_modifier(Modifier::BOLD)
}

pub fn fg_dim(rgb: Rgb) -> Style {
    Style::default().fg(color(rgb)).add_modifier(Modifier::DIM)
}

pub fn fg_bg(fg: Rgb, bg: Rgb) -> Style {
    Style::default().fg(color(fg)).bg(color(bg))
}
