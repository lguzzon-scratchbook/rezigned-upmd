use std::cell::RefCell;

use crate::apps::tui::markdown::LogicalLine;

use super::layout_lines::LayoutLine;

/// Search term plus lower-cased text cache keyed by logical-line index.
///
/// Module owns the term-to-text seam: content rebuilds push fresh texts in via
/// [`Self::rebuild_texts`], per-keystroke filtering reads them via
/// [`Self::matches`]. Callers keep locality by rebuilding explicitly at the two
/// mutation points (content rebuild, term change); no lazy length-keyed refresh
/// inside this interface, so equal-length content swaps cannot go silently stale.
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

    /// Sets the term and rebuilds the cache from current lines in one step, so
    /// term changes never observe a previous generation of content.
    pub fn set_term(&mut self, term: &str, logical_lines: &[LogicalLine]) {
        self.term_lower = if term.is_empty() {
            None
        } else {
            Some(term.to_lowercase())
        };
        self.rebuild_texts(logical_lines);
    }

    /// Unconditionally rebuilds lower-cased searchable texts. Clears stale cache
    /// when no term is active instead of leaving a prior generation behind.
    /// Call explicitly from content rebuilds; per-keystroke filtering reuses the
    /// cache via [`Self::matches`] without extra allocation.
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

    /// Filters layout rows by logical-line match. Falls back to live
    /// `text_content()` when the cache is stale-by-construction (length
    /// mismatch after a missed rebuild); the fresh path leverages the cache.
    pub fn matches(
        &self,
        layout_lines: &[LayoutLine],
        logical_lines: &[LogicalLine],
    ) -> Vec<usize> {
        let Some(term_lower) = self.term_lower.as_deref() else {
            return vec![];
        };
        let texts = self.logical_texts.borrow();
        if texts.len() != logical_lines.len() {
            drop(texts);
            return self.matches_live(layout_lines, logical_lines, term_lower);
        }
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

    /// Adapter for stale-cache reads: derives the match from current lines so
    /// results stay identical with or without a fresh cache generation.
    fn matches_live(
        &self,
        layout_lines: &[LayoutLine],
        logical_lines: &[LogicalLine],
        term_lower: &str,
    ) -> Vec<usize> {
        layout_lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                logical_lines
                    .get(l.logical_idx)
                    .is_some_and(|ll| ll.text_content().to_lowercase().contains(term_lower))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn term_lower(&self) -> Option<&str> {
        self.term_lower.as_deref()
    }
}
