//! Tool metadata: label + hue token per tool kind. Mirrors `TOOLS` in
//! `docs/repl/tokens.jsx` (lines 114–123).

use crate::event::ToolKindId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HueToken {
    TRead,
    TEdit,
    TBash,
    TSearch,
    TList,
    TTodo,
    TWeb,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolMeta {
    pub label: &'static str,
    pub hue: HueToken,
}

pub const fn tool_meta(kind: ToolKindId) -> ToolMeta {
    match kind {
        ToolKindId::Read => ToolMeta { label: "read", hue: HueToken::TRead },
        ToolKindId::Write => ToolMeta { label: "write", hue: HueToken::TEdit },
        ToolKindId::Edit => ToolMeta { label: "edit", hue: HueToken::TEdit },
        ToolKindId::Bash => ToolMeta { label: "bash", hue: HueToken::TBash },
        ToolKindId::Search => ToolMeta { label: "search", hue: HueToken::TSearch },
        ToolKindId::List => ToolMeta { label: "list", hue: HueToken::TList },
        ToolKindId::Todo => ToolMeta { label: "todo", hue: HueToken::TTodo },
        ToolKindId::Web => ToolMeta { label: "fetch", hue: HueToken::TWeb },
    }
}
