use crate::{
    interpreter::InterpreterResult,
    presentation::model_traits::{ListCell, ListCellKind, ListModel},
};

#[derive(Debug, Clone)]
pub struct OutputEntry {
    pub text: String,
    pub kind: ListCellKind,
}

impl OutputEntry {
    pub fn line(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: ListCellKind::Default,
        }
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: ListCellKind::Changeable,
        }
    }
}

pub fn apply_result_to_output(
    entries: &mut Vec<OutputEntry>,
    current_line: &mut String,
    result: &InterpreterResult,
) {
    match result {
        InterpreterResult::PrintChar(c) => {
            current_line.push(*c);
        }
        InterpreterResult::PrintLn(text) => {
            current_line.push_str(text);
            entries.push(OutputEntry::line(std::mem::take(current_line)));
        }
        InterpreterResult::InputRequest(_) => {}
        InterpreterResult::Display { .. } => {}
        InterpreterResult::EndProgram => {
            if !current_line.is_empty() {
                entries.push(OutputEntry::line(std::mem::take(current_line)));
            }
            entries.push(OutputEntry::system("[program finished]"));
        }
    }
}

/// Lazy output model — borrows the entries slice and only clones visible items
/// when [`ListModel::get_item`] is called. Building is O(1).
pub struct OutputListModel<'a> {
    entries: &'a [OutputEntry],
    current_line: &'a str,
}

impl<'a> OutputListModel<'a> {
    pub fn from_output(entries: &'a [OutputEntry], current_line: &'a str) -> Self {
        Self {
            entries,
            current_line,
        }
    }
}

impl ListModel for OutputListModel<'_> {
    fn item_count(&self) -> usize {
        self.entries.len() + usize::from(!self.current_line.is_empty())
    }

    fn get_item(&self, index: usize) -> Option<ListCell> {
        if index < self.entries.len() {
            let entry = &self.entries[index];
            let mut cell = ListCell::new(entry.text.clone(), entry.kind);
            if matches!(entry.kind, ListCellKind::Changeable) {
                cell = cell.with_bold(true);
            }
            Some(cell)
        } else if index == self.entries.len() && !self.current_line.is_empty() {
            Some(ListCell::new(
                self.current_line.to_string(),
                ListCellKind::Active,
            ))
        } else {
            None
        }
    }
}