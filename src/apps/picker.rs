use keymap::KeyMap;

/// Actions shared by the TUI and CLI file pickers.
///
/// Both frontends use the same key bindings so users get consistent behavior
/// whether they open the picker from the TUI (`o` key) or from the CLI
/// (`upmd --cli <directory>` with multiple Markdown files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, KeyMap)]
pub enum PickerAction {
    /// Delete the last character from the filter query.
    #[key("backspace", help = "delete")]
    Delete,
    /// Open the currently selected file.
    #[key("enter", help = "open")]
    Select,
    /// Move selection to the next match.
    #[key("down", "ctrl-n", help = "next")]
    Next,
    /// Move selection to the previous match.
    #[key("up", "ctrl-p", help = "prev")]
    Prev,
    /// Cancel the picker without selecting a file.
    #[key("esc", "ctrl-c", "ctrl-d", help = "cancel")]
    Quit,
    /// Append a typed character to the filter query.
    #[key("@any")]
    Input(char),
}

/// Shared state for file pickers: the file list, filter query, filtered
/// matches, and current selection index.
///
/// Both the TUI [`FilePicker`](crate::apps::tui::file_picker::FilePicker) and
/// CLI [`CliPicker`](crate::apps::cli::picker::CliPicker) embed this struct and
/// delegate navigation logic to [`PickerState::handle_navigation`], ensuring
/// consistent filter and selection behavior across frontends.
pub struct PickerState {
    pub files: Vec<crate::markdown_files::MarkdownFile>,
    pub matches: Vec<usize>,
    pub query: String,
    pub selected: usize,
    /// Lower-cased `files[i].display`, rebuilt only when the file list changes.
    /// `build_matches` filters against this so keystrokes do no allocation.
    lowered: Vec<String>,
}

impl PickerState {
    pub fn new(files: Vec<crate::markdown_files::MarkdownFile>) -> Self {
        let mut state = Self {
            lowered: files.iter().map(|f| f.display.to_lowercase()).collect(),
            files,
            matches: Vec::new(),
            query: String::new(),
            selected: 0,
        };
        state.rebuild();
        state
    }

    /// Recomputes `matches` from `query` and clamps `selected`.
    pub fn rebuild(&mut self) {
        self.matches = self.build_matches();
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }

    /// Returns indices into `files` whose display path contains `query`
    /// (case-insensitive).  An empty query matches everything.
    pub fn build_matches(&self) -> Vec<usize> {
        if self.query.is_empty() {
            return (0..self.files.len()).collect();
        }
        // ponytail: single lowercase per keystroke; per-file lowercase cached.
        // Upgrade when measured: incremental prefix filtering.
        let query = self.query.to_lowercase();
        if self.lowered.len() != self.files.len() {
            // `files` is pub and may have been mutated directly; fall back
            // to uncached filtering rather than returning wrong results.
            return self
                .files
                .iter()
                .enumerate()
                .filter(|(_, file)| file.display.to_lowercase().contains(&query))
                .map(|(idx, _)| idx)
                .collect();
        }
        self.lowered
            .iter()
            .enumerate()
            .filter(|(_, lower)| lower.contains(&query))
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Returns the index into `files` of the currently selected match, or
    /// `None` if there are no matches.
    pub fn selected_file_idx(&self) -> Option<usize> {
        self.matches.get(self.selected).copied()
    }

    /// Handles navigation actions (`Input`, `Delete`, `Next`, `Prev`).
    ///
    /// Returns `true` if the action was a navigation action (and state may have
    /// changed), `false` for `Select` and `Quit` which the caller must handle.
    pub fn handle_navigation(&mut self, action: &PickerAction) -> bool {
        match action {
            PickerAction::Input(ch) => {
                if !ch.is_control() {
                    self.query.push(*ch);
                    self.rebuild();
                }
                true
            }
            PickerAction::Delete => {
                self.query.pop();
                self.rebuild();
                true
            }
            PickerAction::Next => {
                if !self.matches.is_empty() {
                    self.selected = (self.selected + 1).min(self.matches.len().saturating_sub(1));
                }
                true
            }
            PickerAction::Prev => {
                self.selected = self.selected.saturating_sub(1);
                true
            }
            _ => false,
        }
    }
}
