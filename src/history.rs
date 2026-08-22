#[derive(Debug, Default)]
pub struct History {
    entries: Vec<String>,
    position: Option<usize>,
    draft: String,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, command: &str) {
        if command.is_empty() {
            return;
        }

        if self.entries.last().is_none_or(|last| last != command) {
            self.entries.push(command.to_owned());
        }
        self.reset();
    }

    pub fn previous(&mut self, current: &str) -> String {
        if self.entries.is_empty() {
            return current.to_owned();
        }

        let next_position = match self.position {
            Some(position) => position.saturating_sub(1),
            None => {
                self.draft = current.to_owned();
                self.entries.len() - 1
            }
        };

        self.position = Some(next_position);
        self.entries[next_position].clone()
    }

    pub fn next(&mut self, current: &str) -> String {
        let Some(position) = self.position else {
            return current.to_owned();
        };

        if position + 1 >= self.entries.len() {
            let draft = self.draft.clone();
            self.reset();
            return draft;
        }

        let next_position = position + 1;
        self.position = Some(next_position);
        self.entries[next_position].clone()
    }

    pub fn reset(&mut self) {
        self.position = None;
        self.draft.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn navigates_back_and_forward_and_restores_draft() {
        let mut history = History::new();
        history.add("first");
        history.add("second");

        assert_eq!(history.previous("draft"), "second");
        assert_eq!(history.previous("second"), "first");
        assert_eq!(history.previous("first"), "first");
        assert_eq!(history.next("first"), "second");
        assert_eq!(history.next("second"), "draft");
    }

    #[test]
    fn skips_consecutive_duplicate_commands() {
        let mut history = History::new();
        history.add("same");
        history.add("same");

        assert_eq!(history.previous(""), "same");
        assert_eq!(history.previous("same"), "same");
    }
}
