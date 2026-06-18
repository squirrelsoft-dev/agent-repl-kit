//! Agent-driven question form (Looper bolt-on — PR upstream candidate).
//!
//! Lets the agent ask the user one or more questions and collect structured
//! answers. Two question shapes are supported:
//!
//! * [`QuestionKind::Single`] — multiple choice, pick exactly one. With a
//!   freeform entry the user can instead type their own ("Other") answer.
//! * [`QuestionKind::Multi`] — multi-select, toggle any number. With a freeform
//!   entry an extra toggleable row lets the user add a custom message.
//!
//! A [`QuestionForm`] holds several questions; the renderer shows each as a
//! tab. When there is more than one question, a trailing **Submit** tab is
//! added (see the renderer). This module is pure data — no I/O, no rendering.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Whether a question takes one answer or many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum QuestionKind {
    /// Pick exactly one option (multiple choice).
    Single,
    /// Toggle any number of options (multi-select).
    Multi,
}

/// One question. Rendered as a single tab in the question box.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Question {
    /// The question itself, e.g. `Which database should we use?`.
    pub title: String,
    /// Optional secondary line of guidance.
    pub detail: Option<String>,
    /// Single-answer or multi-answer.
    pub kind: QuestionKind,
    /// The fixed choices, in display order.
    pub options: Vec<String>,
    /// When `Some(label)`, an extra freeform row is offered (e.g. `"Other"` for
    /// a [`QuestionKind::Single`] question, or `"Something else"` for a
    /// [`QuestionKind::Multi`] one) where the user types their own answer.
    pub freeform: Option<String>,
}

impl Question {
    /// A multiple-choice question (pick one), no freeform entry.
    pub fn single(title: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            title: title.into(),
            detail: None,
            kind: QuestionKind::Single,
            options,
            freeform: None,
        }
    }

    /// A multi-select question (pick any), no freeform entry.
    pub fn multi(title: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            title: title.into(),
            detail: None,
            kind: QuestionKind::Multi,
            options,
            freeform: None,
        }
    }

    /// Add a secondary guidance line.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Offer a freeform entry labeled `label` (e.g. `"Other"`). On a `Single`
    /// question it lets the user type an answer instead of picking; on a `Multi`
    /// question it is an extra toggleable row carrying a custom message.
    pub fn with_freeform(mut self, label: impl Into<String>) -> Self {
        self.freeform = Some(label.into());
        self
    }

    /// Convenience: [`Self::with_freeform`] with the label `"Other"`.
    pub fn with_other(self) -> Self {
        self.with_freeform("Other")
    }
}

/// A set of questions to ask in one pass. Each is shown as a tab.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QuestionForm {
    /// Optional headline shown in the box border, e.g. `Before I start…`.
    pub intro: Option<String>,
    /// The questions, one per tab.
    pub questions: Vec<Question>,
}

impl QuestionForm {
    /// A form from a list of questions.
    pub fn new(questions: Vec<Question>) -> Self {
        Self { intro: None, questions }
    }

    /// A single-question form.
    pub fn one(question: Question) -> Self {
        Self { intro: None, questions: vec![question] }
    }

    /// Set the headline shown on the box.
    pub fn with_intro(mut self, intro: impl Into<String>) -> Self {
        self.intro = Some(intro.into());
        self
    }
}

/// The answer to one [`Question`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum QuestionAnswer {
    /// Answer to a [`QuestionKind::Single`] question. Exactly one of `option`
    /// (an index into [`Question::options`]) or `other` (freeform text) is set
    /// once answered; both are `None` if the user skipped it.
    Single {
        option: Option<usize>,
        other: Option<String>,
    },
    /// Answer to a [`QuestionKind::Multi`] question. `options` are the toggled
    /// indices (sorted), `custom` the freeform message if one was added.
    Multi {
        options: Vec<usize>,
        custom: Option<String>,
    },
}

impl QuestionAnswer {
    /// True if the user provided no answer for this question.
    pub fn is_empty(&self) -> bool {
        match self {
            QuestionAnswer::Single { option, other } => option.is_none() && other.is_none(),
            QuestionAnswer::Multi { options, custom } => options.is_empty() && custom.is_none(),
        }
    }

    /// Render this answer back to human-readable text using the question's
    /// option labels. `None` when unanswered.
    pub fn describe(&self, question: &Question) -> Option<String> {
        match self {
            QuestionAnswer::Single { option, other } => {
                if let Some(i) = option {
                    question.options.get(*i).cloned()
                } else {
                    other.clone()
                }
            }
            QuestionAnswer::Multi { options, custom } => {
                let mut parts: Vec<String> = options
                    .iter()
                    .filter_map(|i| question.options.get(*i).cloned())
                    .collect();
                if let Some(c) = custom {
                    parts.push(c.clone());
                }
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join(", "))
                }
            }
        }
    }
}

/// All answers for a [`QuestionForm`], one per question, same order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FormAnswers {
    pub answers: Vec<QuestionAnswer>,
}

impl FormAnswers {
    pub fn new(answers: Vec<QuestionAnswer>) -> Self {
        Self { answers }
    }
}
