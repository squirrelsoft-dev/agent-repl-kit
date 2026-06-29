//! `agent-repl-core` — token + event model that drives the agent-repl
//! renderer. Pure data, no I/O. The future Ink/TS package mirrors this API.
//!
//! See `docs/DESIGN_AND_USAGE.md` for the design spec.

#![warn(missing_debug_implementations)]

pub mod approval;
pub mod event;
pub mod palette;
pub mod question;
pub mod theme;
pub mod tools;

pub use approval::{ApprovalChoice, ApprovalPrompt};
pub use question::{
    FormAnswers, Question, QuestionAnswer, QuestionForm, QuestionKind,
};
pub use event::{
    AlertLevel, DiffKind, DiffLine, EntryType, Event, ListEntry, ReadLine, SearchGroup, SearchHit,
    SearchResult, ToolCall, ToolKind, ToolKindId, TodoItem, TodoState,
};
pub use palette::{mix, palette_for, Palette, Rgb};
pub use theme::{
    density_spec, vibe_info, Density, DensitySpec, Mode, ProseFont, Theme, ToolStyle, Vibe,
    VibeInfo,
};
pub use tools::{tool_meta, HueToken, ToolMeta};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_builder_round_trips_all_combos() {
        let vibes = [Vibe::Phosphor, Vibe::Slate, Vibe::Spectrum, Vibe::Ember];
        let modes = [Mode::Dark, Mode::Light];
        let styles = [ToolStyle::Inline, ToolStyle::Card, ToolStyle::Collapsed];
        let densities = [Density::Comfortable, Density::Compact];
        let mut n = 0;
        for &v in &vibes {
            for &m in &modes {
                for &s in &styles {
                    for &d in &densities {
                        let t = Theme::new(v).with_mode(m).with_tool_style(s).with_density(d);
                        assert_eq!(t.vibe, v);
                        assert_eq!(t.mode, m);
                        assert_eq!(t.tool_style, s);
                        assert_eq!(t.density, d);
                        n += 1;
                    }
                }
            }
        }
        assert_eq!(n, 4 * 2 * 3 * 2);
    }

    #[test]
    fn palette_resolves_for_every_vibe_mode_combo() {
        // Smoke-test: every (vibe, mode) palette resolves without panicking
        // and produces a non-zero `text` color.
        for &v in &[Vibe::Phosphor, Vibe::Slate, Vibe::Spectrum, Vibe::Ember] {
            for &m in &[Mode::Dark, Mode::Light] {
                let p = palette_for(v, m);
                // text must be visible on bg (not the same exact color).
                assert_ne!(p.text, p.bg, "{:?}/{:?} text == bg", v, m);
            }
        }
    }

    #[test]
    fn tool_meta_covers_all_kinds() {
        // Exhaustive match: if a new ToolKind is added without updating
        // tool_meta, this won't compile.
        for id in [
            ToolKindId::Read,
            ToolKindId::Write,
            ToolKindId::Edit,
            ToolKindId::Bash,
            ToolKindId::Search,
            ToolKindId::List,
            ToolKindId::Todo,
            ToolKindId::Web,
            ToolKindId::Info,
        ] {
            let m = tool_meta(id);
            assert!(!m.label.is_empty());
        }
    }

    #[test]
    fn cycle_vibe_visits_all_four() {
        let mut t = Theme::phosphor();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..4 {
            seen.insert(t.vibe);
            t = t.cycle_vibe();
        }
        assert_eq!(seen.len(), 4);
        assert_eq!(t.vibe, Vibe::Phosphor); // back to start
    }

    #[test]
    fn mix_endpoints_are_identity() {
        let a = Rgb(20, 30, 40);
        let b = Rgb(220, 180, 140);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }
}
