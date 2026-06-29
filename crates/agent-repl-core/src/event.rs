//! Event model. Mirrors the JSX event shapes in `docs/DESIGN_AND_USAGE.md` §3.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "lowercase"))]
pub enum Event {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Reasoning {
        text: String,
        #[cfg_attr(feature = "serde", serde(default))]
        ms: Option<u32>,
        #[cfg_attr(feature = "serde", serde(default))]
        default_open: bool,
    },
    Status {
        text: String,
    },
    Alert {
        level: AlertLevel,
        title: String,
        #[cfg_attr(feature = "serde", serde(default))]
        detail: Option<String>,
    },
    Tool(ToolCall),
}

impl Event {
    pub fn user(text: impl Into<String>) -> Self {
        Self::User { text: text.into() }
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::Assistant { text: text.into() }
    }
    pub fn status(text: impl Into<String>) -> Self {
        Self::Status { text: text.into() }
    }
    pub fn reasoning(text: impl Into<String>, ms: Option<u32>) -> Self {
        Self::Reasoning { text: text.into(), ms, default_open: false }
    }
    pub fn error(title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Alert {
            level: AlertLevel::Error,
            title: title.into(),
            detail: Some(detail.into()),
        }
    }
    pub fn warning(title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Alert {
            level: AlertLevel::Warning,
            title: title.into(),
            detail: Some(detail.into()),
        }
    }
    pub fn tool(call: ToolCall) -> Self {
        Self::Tool(call)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum AlertLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ToolCall {
    pub title: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub running: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub run_label: Option<String>,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub kind: ToolKind,
}

impl ToolCall {
    pub fn new(title: impl Into<String>, kind: ToolKind) -> Self {
        Self { title: title.into(), running: false, run_label: None, kind }
    }
    pub fn running(mut self) -> Self {
        self.running = true;
        self
    }
    pub fn with_run_label(mut self, label: impl Into<String>) -> Self {
        self.run_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "lowercase"))]
pub enum ToolKind {
    Search {
        result: SearchResult,
    },
    Read {
        path: String,
        lines: usize,
        #[cfg_attr(feature = "serde", serde(default))]
        preview: Vec<ReadLine>,
    },
    List {
        entries: Vec<ListEntry>,
    },
    Edit {
        diff: Vec<DiffLine>,
    },
    Write {
        diff: Vec<DiffLine>,
    },
    Bash {
        cmd: String,
        #[cfg_attr(feature = "serde", serde(default))]
        output: String,
        #[cfg_attr(feature = "serde", serde(default))]
        exit: Option<i32>,
    },
    Todo {
        items: Vec<TodoItem>,
    },
    Web {
        url: String,
        #[cfg_attr(feature = "serde", serde(default))]
        summary: Option<String>,
    },
    /// A generic tool with no dedicated component. `detail` is a one-line
    /// summary of the call (e.g. compact args); `output` is the tool's result
    /// text. The fallback for any tool the host doesn't map to a richer kind,
    /// so an unknown tool renders as a neutral info block instead of being
    /// misframed as a shell command.
    Info {
        #[cfg_attr(feature = "serde", serde(default))]
        detail: String,
        #[cfg_attr(feature = "serde", serde(default))]
        output: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKindId {
    Search,
    Read,
    List,
    Edit,
    Write,
    Bash,
    Todo,
    Web,
    Info,
}

impl ToolKind {
    pub fn id(&self) -> ToolKindId {
        match self {
            Self::Search { .. } => ToolKindId::Search,
            Self::Read { .. } => ToolKindId::Read,
            Self::List { .. } => ToolKindId::List,
            Self::Edit { .. } => ToolKindId::Edit,
            Self::Write { .. } => ToolKindId::Write,
            Self::Bash { .. } => ToolKindId::Bash,
            Self::Todo { .. } => ToolKindId::Todo,
            Self::Web { .. } => ToolKindId::Web,
            Self::Info { .. } => ToolKindId::Info,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SearchResult {
    pub count: usize,
    pub groups: Vec<SearchGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SearchGroup {
    pub file: String,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SearchHit {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReadLine {
    /// 1-based line number (`n` in the JSX shape).
    pub n: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ListEntry {
    pub name: String,
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub entry_type: EntryType,
    #[cfg_attr(feature = "serde", serde(default))]
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum EntryType {
    Dir,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiffLine {
    #[cfg_attr(feature = "serde", serde(rename = "t"))]
    pub kind: DiffKind,
    /// Old (pre-edit) line number — present for `Del` and `Ctx`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub a: Option<usize>,
    /// New (post-edit) line number — present for `Add` and `Ctx`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub b: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum DiffKind {
    Add,
    Del,
    Ctx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TodoItem {
    pub state: TodoState,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum TodoState {
    Done,
    Active,
    Pending,
}
