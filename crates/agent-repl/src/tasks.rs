//! Sticky task-list panel — a checklist of the agent's current plan, floating
//! directly above the working line while tasks are in flight. The driving task
//! sets it via [`ReplHandle::set_tasks`](crate::handle::ReplHandle::set_tasks)
//! and clears it when the work is done; the panel claims zero rows when empty.
//!
//! Reuses [`todo_body`](crate::blocks::bodies::todo_body) for the ✓/▸/○ rows, so
//! the sticky panel and the in-stream `todo_write` block stay visually identical.

use agent_repl_core::{Theme, TodoItem, TodoState};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::blocks::bodies::todo_body;
use crate::style::{color, fg};

/// Max task rows shown at once; a longer list scrolls to keep the active task
/// visible (the panel never grows past this so it can't swallow the transcript).
const MAX_ROWS: usize = 10;

/// Rows the task panel occupies: one per visible task + top/bottom border. Zero
/// when there are no tasks (the region collapses).
pub fn required_height(tasks: &[TodoItem]) -> u16 {
    if tasks.is_empty() {
        return 0;
    }
    tasks.len().min(MAX_ROWS) as u16 + 2
}

/// Render the task panel into `area` (a bordered checklist titled with the
/// done/total progress). No-op when there are no tasks.
pub fn render(tasks: &[TodoItem], theme: &Theme, frame: &mut Frame, area: Rect) {
    if area.height == 0 || area.width == 0 || tasks.is_empty() {
        return;
    }
    let p = &theme.palette;
    let done = tasks.iter().filter(|t| matches!(t.state, TodoState::Done)).count();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(fg(p.t_todo))
        .title(Span::styled(
            format!(" tasks · {done}/{} ", tasks.len()),
            fg(p.t_todo).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(color(p.bg_raised)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    // Window the rows so the active task stays visible when the list is long.
    let rows = todo_body(tasks, p);
    let visible: Vec<_> = if rows.len() <= MAX_ROWS {
        rows
    } else {
        let active = tasks
            .iter()
            .position(|t| matches!(t.state, TodoState::Active))
            .unwrap_or(0);
        let start = active.min(rows.len() - MAX_ROWS);
        rows[start..start + MAX_ROWS].to_vec()
    };
    frame.render_widget(Paragraph::new(Text::from(visible)), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(text: &str) -> TodoItem {
        TodoItem { state: TodoState::Pending, text: text.into() }
    }

    #[test]
    fn empty_list_collapses_to_zero_rows() {
        assert_eq!(required_height(&[]), 0);
    }

    #[test]
    fn height_is_tasks_plus_borders_capped() {
        assert_eq!(required_height(&[item("a"), item("b")]), 4); // 2 tasks + 2 borders
        let many: Vec<_> = (0..40).map(|i| item(&format!("t{i}"))).collect();
        assert_eq!(required_height(&many), MAX_ROWS as u16 + 2); // capped
    }
}
