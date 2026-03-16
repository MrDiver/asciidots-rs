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
            kind: ListCellKind::OperatorDelimiter,
        }
    }
}

pub fn apply_result_to_output(
    entries: &mut Vec<OutputEntry>,
    current_line: &mut String,
    result: &InterpreterResult,
) {
    match result {
        InterpreterResult::Print(text) => {
            current_line.push_str(text);
        }
        InterpreterResult::PrintLn(text) => {
            current_line.push_str(text);
            entries.push(OutputEntry::line(std::mem::take(current_line)));
        }
        InterpreterResult::InputRequest(id) => {
            if !current_line.is_empty() {
                entries.push(OutputEntry::line(std::mem::take(current_line)));
            }
            entries.push(OutputEntry::system(format!("[input request: {id}]")));
        }
        InterpreterResult::EndProgram => {
            if !current_line.is_empty() {
                entries.push(OutputEntry::line(std::mem::take(current_line)));
            }
            entries.push(OutputEntry::system("[program finished]"));
        }
    }
}

pub struct OutputListModel {
    items: Vec<ListCell>,
}

impl OutputListModel {
    pub fn from_output(entries: &[OutputEntry], current_line: &str) -> Self {
        let mut items = entries
            .iter()
            .map(|entry| {
                let mut cell = ListCell::new(entry.text.clone(), entry.kind);
                if matches!(entry.kind, ListCellKind::OperatorDelimiter) {
                    cell = cell.with_bold(true);
                }
                cell
            })
            .collect::<Vec<_>>();

        if !current_line.is_empty() {
            items.push(ListCell::new(
                current_line.to_string(),
                ListCellKind::Active,
            ));
        }

        Self { items }
    }
}

impl ListModel for OutputListModel {
    fn item_count(&self) -> usize {
        self.items.len()
    }

    fn get_item(&self, index: usize) -> Option<ListCell> {
        self.items.get(index).cloned()
    }
}
