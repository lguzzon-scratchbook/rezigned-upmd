use std::cell::RefCell;

use crate::apps::tui::markdown::LogicalLine;

use super::layout_lines::LayoutLine;

/// Search term and lower-cased text cache keyed by logical-line index.
pub struct PreviewSearch {
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
            term_lower: None,
            logical_texts: RefCell::new(vec![]),
        }
    }

    pub fn set_term(&mut self, term: &str) {
        self.term_lower = if term.is_empty() {
            None
        } else {
            Some(term.to_lowercase())
        };
    }

    /// Rebuilds lower-cased searchable texts. Skips all allocation when no
    /// term is active (clears stale cache instead). Called on content change;
    /// per-keystroke filtering reuses the cache via [`Self::matches`].
    pub fn rebuild_texts(&self, logical_lines: &[LogicalLine]) {
        if self.term_lower.is_none() {
            if !self.logical_texts.borrow().is_empty() {
                self.logical_texts.borrow_mut().clear();
            }
            return;
        }
        *self.logical_texts.borrow_mut() = logical_lines
            .iter()
            .map(|ll| ll.text_content().to_lowercase())
            .collect();
    }

    /// Rebuilds cache if query activated while cache was cleared as inactive.
    /// No-op when lengths match: content rebuilds already refresh the cache.
    pub fn ensure_texts(&self, logical_lines: &[LogicalLine]) {
        if self.term_lower.is_none() {
            return;
        }
        if self.logical_texts.borrow().len() != logical_lines.len() {
            self.rebuild_texts(logical_lines);
        }
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

    pub fn term_lower(&self) -> Option<&str> {
        self.term_lower.as_deref()
    }
}
