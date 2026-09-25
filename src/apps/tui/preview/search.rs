use std::cell::RefCell;

use crate::apps::tui::markdown::LogicalLine;

use super::layout_lines::LayoutLine;

/// Search term and lower-cased text cache keyed by logical-line index.
pub struct PreviewSearch {
    term: Option<String>,
    term_lower: Option<String>,
    logical_texts: RefCell<Vec<String>>,
}

impl Default for PreviewSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewSearch {
    pub fn new() -> Self {
        Self {
            term: None,
            term_lower: None,
            logical_texts: RefCell::new(vec![]),
        }
    }

    pub fn set_term(&mut self, term: &str) {
        if term.is_empty() {
            self.term = None;
            self.term_lower = None;
        } else {
            self.term = Some(term.to_string());
            self.term_lower = Some(term.to_lowercase());
        }
    }

    pub fn rebuild_texts(&self, logical_lines: &[LogicalLine]) {
        *self.logical_texts.borrow_mut() = logical_lines
            .iter()
            .map(|ll| ll.text_content().to_lowercase())
            .collect();
    }

    pub fn matches(&self, layout_lines: &[LayoutLine]) -> Vec<usize> {
        let Some(term_lower) = self.term_lower.as_deref() else {
            return vec![];
        };
        let texts = self.logical_texts.borrow();
        layout_lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                texts
                    .get(l.logical_idx)
                    .is_some_and(|t| t.contains(term_lower))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn term(&self) -> Option<&str> {
        self.term.as_deref()
    }

    pub fn term_lower(&self) -> Option<&str> {
        self.term_lower.as_deref()
    }
}
