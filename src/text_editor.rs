use rslint_parser::TextRange;

struct Change(usize, String);

pub trait TextEdit {
    fn load(text: impl ToString) -> TextEditor;
    fn insert_after(&mut self, range: TextRange, text: impl ToString);
    fn insert_before(&mut self, range: TextRange, text: impl ToString);
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

    fn insert_after(&mut self, range: TextRange, text: impl ToString) {
        self.changes
            .push(Change(range.end().into(), text.to_string()));
    }

    fn insert_before(&mut self, range: TextRange, text: impl ToString) {
        self.changes
            .push(Change(range.start().into(), text.to_string()));
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
