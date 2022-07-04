use log::debug;

struct Change(usize, String);

#[derive(Debug)]
pub struct Range {
    start: usize,
    end: usize
}

impl From<rslint_parser::TextRange> for Range {
    fn from(text_range: rslint_parser::TextRange) -> Self {
        Range { start: text_range.start().into(), end: text_range.end().into() }
    }
}

pub trait TextEdit {
    fn load(text: impl ToString) -> TextEditor;
    fn insert_after(&mut self, range: Range, text: impl ToString);
    fn insert_before(&mut self, range: Range, text: impl ToString);
    fn apply(&mut self) -> String;
}

pub struct TextEditor {
    changes: Vec<Change>,
    source: String,
}

impl TextEdit for TextEditor {
    fn load(text: impl ToString) -> TextEditor {
        TextEditor {
            changes: vec![],
            source: text.to_string(),
        }
    }

    fn insert_after(&mut self, range: Range, text: impl ToString) {
        debug!("FIXER insert_after: {:?}", range);
        self.changes
            .push(Change(range.end, text.to_string()));
    }

    fn insert_before(&mut self, range: Range, text: impl ToString) {
        debug!("FIXER insert_before: {:?}", range);
        self.changes
            .push(Change(range.start, text.to_string()));
    }

    fn apply(&mut self) -> String {
        let new_source_length = {
            let total_insertion_length: usize =
                self.changes.iter().map(|change| change.1.len()).sum();

            self.source.len() + total_insertion_length
        };

        self.changes.sort_by(|a, b| a.0.cmp(&b.0));
        let mut buf = String::with_capacity(new_source_length);
        let mut pointer = 0usize;

        for change in &self.changes {
            let current_pointer = change.0;
            if current_pointer > pointer {
                buf.push_str(&self.source[pointer..current_pointer]);
            }

            buf.push_str(change.1.as_str());
            pointer = current_pointer
        }
        buf.push_str(&self.source[pointer..self.source.len()]);

        buf
    }
}
