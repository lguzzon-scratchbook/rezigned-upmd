//! Owned scroll/selection state for the preview pane.
//!
//! This module is the single owner of viewport geometry and list position:
//! area, width, height, offset, selection, deferred jump target, and status
//! gutter preference. The interface keeps ratatui's `ListState` at the render
//! seam only (a throwaway adapter built per frame), so the rest of the
//! module works against plain indices. That locality removes `Cell`/`RefCell`
//! churn from `Preview` and gives the rebuild path one place to apply
//! jump/preserve/follow decisions. Pure helpers (`identity_of`, `resolve`,
//! `layout_extent_for_code`, `inline_pty_rows`) leverage slices only, keeping
//! depth low and staying testable without a full `Preview`.

use ratatui::layout::Rect;

use crate::apps::config::{
    BORDER_HEIGHT, CODE_NAVIGATION_CONTEXT_ROWS, INLINE_PTY_MIN_PERCENT, INLINE_PTY_MIN_ROWS,
    PREVIEW_CONTENT_TOP_OFFSET, PREVIEW_CONTENT_X_OFFSET, PREVIEW_FRAME_OVERHEAD, PTY_DEFAULT_COLS,
};
use crate::runner::CodeId;

use super::layout_lines::LayoutLine;
use crate::apps::tui::markdown::{LogicalLine, SourcePosition};

/// Identifies a layout line across layout rebuilds.
#[derive(Clone, Copy)]
pub(crate) enum LayoutLineIdentity {
    Code {
        id: CodeId,
        line_idx: usize,
        wrap_idx: usize,
    },
    Document {
        source_position: SourcePosition,
        wrap_idx: usize,
    },
}

/// Rebuild decision handed to [`Viewport::rebuild_layout`].
pub(crate) enum RebuildTarget {
    /// Explicit jump (deferred block target): snap offset to the index.
    Jump(usize),
    /// Passive rebuild: keep the row, shifted by `offset`.
    Preserve { idx: usize, offset: usize },
}

/// Owned viewport: geometry plus scroll/selection position.
///
/// Interior mutability keeps `Preview`'s `&self` render/mouse interface
/// intact; the module stays the single owner of geometry and list position.
pub(crate) struct Viewport {
    area: std::cell::Cell<Rect>,
    width: std::cell::Cell<usize>,
    height: std::cell::Cell<usize>,
    offset: std::cell::Cell<usize>,
    selected: std::cell::Cell<Option<usize>>,
    target_block: std::cell::Cell<Option<CodeId>>,
    prefer_status_gutter: std::cell::Cell<Option<CodeId>>,
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new()
    }
}

impl Viewport {
    pub(crate) fn new() -> Self {
        Self {
            area: std::cell::Cell::new(Rect::default()),
            width: std::cell::Cell::new(PTY_DEFAULT_COLS as usize),
            height: std::cell::Cell::new(0),
            offset: std::cell::Cell::new(0),
            selected: std::cell::Cell::new(None),
            target_block: std::cell::Cell::new(None),
            prefer_status_gutter: std::cell::Cell::new(None),
        }
    }

    pub(crate) fn area(&self) -> Rect {
        self.area.get()
    }

    pub(crate) fn width(&self) -> usize {
        self.width.get()
    }

    pub(crate) fn height(&self) -> usize {
        self.height.get()
    }

    pub(crate) fn offset(&self) -> usize {
        self.offset.get()
    }

    pub(crate) fn selected(&self) -> Option<usize> {
        self.selected.get()
    }

    pub(crate) fn set_width(&self, width: usize) {
        self.width.set(width);
    }

    /// Tracks the rendered area; derives width and content height from it.
    pub(crate) fn set_area(&self, area: Rect) {
        self.area.set(area);
        self.width.set(area.width as usize);
        self.height
            .set((area.height as usize).saturating_sub(BORDER_HEIGHT));
    }

    /// Content rect inside borders/padding; single source for mouse mapping
    /// and image placement.
    pub(crate) fn content_rect(&self) -> Rect {
        let area = self.area.get();
        Rect::new(
            area.x.saturating_add(PREVIEW_CONTENT_X_OFFSET),
            area.y.saturating_add(PREVIEW_CONTENT_TOP_OFFSET),
            area.width.saturating_sub(PREVIEW_FRAME_OVERHEAD as u16),
            (area.height as usize).saturating_sub(BORDER_HEIGHT) as u16,
        )
    }

    /// Zero-based row within the content area, or `None` on the border.
    pub(crate) fn rel_row(&self, mouse: &crossterm::event::MouseEvent) -> Option<usize> {
        let area = self.area.get();
        let content_y = area.y.saturating_add(PREVIEW_CONTENT_TOP_OFFSET);
        let content_bottom = area
            .y
            .saturating_add(area.height.saturating_sub(BORDER_HEIGHT as u16));
        if mouse.row < content_y || mouse.row >= content_bottom {
            return None;
        }
        Some(mouse.row.saturating_sub(content_y) as usize)
    }

    pub(crate) fn select(&self, idx: Option<usize>) {
        self.selected.set(idx);
    }

    pub(crate) fn set_offset(&self, offset: usize) {
        self.offset.set(offset);
    }

    pub(crate) fn take_target(&self) -> Option<CodeId> {
        self.target_block.take()
    }

    #[allow(dead_code)]
    pub(crate) fn clear_target(&self) {
        self.target_block.set(None);
    }

    pub(crate) fn prefer_status_gutter_for(&self, id: CodeId) {
        self.prefer_status_gutter.set(Some(id));
    }

    /// Applies a rebuilt layout: jump snaps to the index, preserve keeps the
    /// shifted row, then both clamp. Returns true when the layout emptied.
    pub(crate) fn rebuild_layout(&self, new_len: usize, target: Option<RebuildTarget>) -> bool {
        match target {
            Some(RebuildTarget::Jump(idx)) => {
                self.selected.set(Some(idx));
                self.offset.set(idx);
            }
            Some(RebuildTarget::Preserve { idx, offset }) => {
                self.selected.set(Some(idx));
                self.offset.set(offset);
            }
            None => {}
        }
        self.clamp(new_len)
    }

    /// Keeps offset/selection valid for `len` rows. Returns true on empty.
    pub(crate) fn clamp(&self, len: usize) -> bool {
        if len == 0 {
            self.selected.set(None);
            self.offset.set(0);
            return true;
        }
        let max_idx = len - 1;
        if let Some(selected) = self.selected.get() {
            self.selected.set(Some(selected.min(max_idx)));
        }
        self.offset.set(self.offset.get().min(max_idx));
        false
    }

    /// Scrolls by `delta` rows; selection follows the offset.
    pub(crate) fn scroll_by(&self, delta: isize, len: usize) {
        if len == 0 {
            self.selected.set(None);
            self.offset.set(0);
            return;
        }
        let max_idx = len - 1;
        let next = self
            .offset
            .get()
            .saturating_add_signed(delta)
            .clamp(0, max_idx);
        self.selected.set(Some(next));
        self.offset.set(next);
    }

    /// Pages by `pages` viewports from the selected row; offset follows.
    pub(crate) fn page_by(&self, pages: isize, len: usize) {
        if len == 0 {
            self.selected.set(None);
            self.offset.set(0);
            return;
        }
        let max_idx = len - 1;
        let current = self
            .selected
            .get()
            .unwrap_or_else(|| self.offset.get())
            .min(max_idx);
        let next = current
            .saturating_add_signed(pages.saturating_mul(self.height.get() as isize))
            .clamp(0, max_idx);
        self.selected.set(Some(next));
        self.offset.set(next);
    }

    /// Snaps selection and offset to a code block start.
    pub(crate) fn goto_code_start(&self, idx: usize) {
        self.selected.set(Some(idx));
        self.offset.set(idx);
    }

    /// Records a code navigation: defers when the block has no layout row
    /// yet, selects in place when already visible, else defers and reveals.
    pub(crate) fn note_code_visible_or_defer(&self, id: CodeId, idx: Option<usize>) {
        let Some(idx) = idx else {
            self.target_block.set(Some(id));
            return;
        };
        if self.has_context(idx) {
            self.target_block.set(None);
            self.selected.set(Some(idx));
        } else {
            self.target_block.set(Some(id));
            self.goto_code_start(idx);
        }
    }

    /// Selects an already-visible block without moving the viewport.
    pub(crate) fn select_in_place(&self, idx: Option<usize>) {
        self.target_block.set(None);
        if let Some(idx) = idx {
            self.selected.set(Some(idx));
        }
    }

    /// Resolves the transient status-gutter preference against the active
    /// block; consumed on navigate-away.
    pub(crate) fn gutter_for(&self, active: Option<CodeId>) -> Option<CodeId> {
        match self.prefer_status_gutter.get() {
            Some(id) if active == Some(id) => Some(id),
            Some(_) => {
                self.prefer_status_gutter.set(None);
                None
            }
            None => None,
        }
    }

    /// Follows a grown code block's bottom edge so tail output stays visible.
    pub(crate) fn grown_bottom_offset(
        &self,
        offset: usize,
        end: usize,
        rows: usize,
        previous_rows: usize,
    ) -> usize {
        let height = self.height.get();
        if height == 0 {
            return offset;
        }
        if rows <= previous_rows || end < offset.saturating_add(height) {
            return offset;
        }
        end.saturating_add(1).saturating_sub(height)
    }

    /// True when `idx` sits fully in view with navigation context rows below.
    pub(crate) fn has_context(&self, idx: usize) -> bool {
        let height = self.height.get();
        let offset = self.offset.get();
        let required_rows = CODE_NAVIGATION_CONTEXT_ROWS.min(height);
        height > 0
            && idx >= offset
            && idx.saturating_add(required_rows) <= offset.saturating_add(height)
    }
}

/// Captures the selected row's stable identity before a rebuild.
pub(crate) fn identity_of(
    layout: &[LayoutLine],
    logical: &[LogicalLine],
    selected: Option<usize>,
) -> Option<LayoutLineIdentity> {
    let line = layout.get(selected?)?;
    match line.code_id(logical) {
        Some(id) => {
            let first_logical_idx = layout
                .iter()
                .find(|line| line.logical(logical).code_id == Some(id))?
                .logical_idx;
            Some(LayoutLineIdentity::Code {
                id,
                line_idx: line.logical_idx.saturating_sub(first_logical_idx),
                wrap_idx: line.wrap_idx,
            })
        }
        None => Some(LayoutLineIdentity::Document {
            source_position: logical[line.logical_idx].source_position,
            wrap_idx: line.wrap_idx,
        }),
    }
}

/// Resolves a captured identity against the rebuilt layout.
pub(crate) fn resolve(
    layout: &[LayoutLine],
    logical: &[LogicalLine],
    identity: LayoutLineIdentity,
) -> Option<usize> {
    match identity {
        LayoutLineIdentity::Code {
            id,
            line_idx,
            wrap_idx,
        } => {
            let first_logical_idx = layout
                .iter()
                .find(|line| line.logical(logical).code_id == Some(id))?
                .logical_idx;
            let logical_idx = first_logical_idx + line_idx;
            layout
                .iter()
                .position(|line| line.logical_idx == logical_idx && line.wrap_idx == wrap_idx)
                .or_else(|| {
                    layout
                        .iter()
                        .position(|line| line.logical_idx == logical_idx)
                })
        }
        LayoutLineIdentity::Document {
            source_position,
            wrap_idx,
        } => layout
            .iter()
            .enumerate()
            .min_by_key(|(_, line)| {
                let candidate = logical[line.logical_idx].source_position;
                let different_kind = !candidate.same_kind(source_position);
                let source_distance = candidate.offset().abs_diff(source_position.offset());
                let different_wrap = line.wrap_idx != wrap_idx;
                (different_kind, source_distance, different_wrap)
            })
            .map(|(index, _)| index),
    }
}

/// First/last layout extent and row count for a code block.
pub(crate) fn layout_extent_for_code(
    layout: &[LayoutLine],
    logical: &[LogicalLine],
    id: CodeId,
) -> Option<(usize, usize)> {
    let mut indices = layout
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| (line.code_id(logical) == Some(id)).then_some(idx));
    let first = indices.next()?;
    let end = indices.next_back().unwrap_or(first);
    Some((end, end - first + 1))
}

/// Computes how many PTY rows fit below a block's source lines in the
/// viewport, returning `(rows, new_offset)`. Scrolls the viewport if fewer
/// than 40% of the rows (min 8) remain below the source.
pub(crate) fn inline_pty_rows(
    viewport: usize,
    source_end: usize,
    source_rows: usize,
    offset: usize,
) -> (usize, usize) {
    if viewport == 0 {
        return (1, offset);
    }

    let target = ((viewport * INLINE_PTY_MIN_PERCENT).div_ceil(100))
        .max(INLINE_PTY_MIN_ROWS)
        .min(viewport)
        .min(viewport.saturating_sub(source_rows).max(1));

    let available = if source_end < offset {
        viewport
    } else {
        viewport
            .saturating_sub(source_end.saturating_sub(offset).saturating_add(1))
            .max(1)
    };

    if available >= target {
        (available, offset)
    } else {
        let new_offset = source_end
            .saturating_add(1)
            .saturating_add(target)
            .saturating_sub(viewport);
        (target, new_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport_with_area() -> Viewport {
        let viewport = Viewport::new();
        viewport.set_area(Rect::new(0, 0, 80, 20));
        viewport
    }

    #[test]
    fn content_rect_uses_single_source_offsets() {
        let viewport = viewport_with_area();
        assert_eq!(
            viewport.content_rect(),
            Rect::new(
                PREVIEW_CONTENT_X_OFFSET,
                PREVIEW_CONTENT_TOP_OFFSET,
                80 - PREVIEW_FRAME_OVERHEAD as u16,
                20 - BORDER_HEIGHT as u16,
            )
        );
    }

    #[test]
    fn rel_row_rejects_border_rows() {
        let viewport = viewport_with_area();
        let inside = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Moved,
            column: 5,
            row: PREVIEW_CONTENT_TOP_OFFSET,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        assert_eq!(viewport.rel_row(&inside), Some(0));
        let border = crossterm::event::MouseEvent { row: 0, ..inside };
        assert_eq!(viewport.rel_row(&border), None);
        let bottom = crossterm::event::MouseEvent {
            row: 20 - BORDER_HEIGHT as u16,
            ..inside
        };
        assert_eq!(viewport.rel_row(&bottom), None);
    }

    #[test]
    fn clamp_keeps_offset_and_selection_in_range() {
        let viewport = viewport_with_area();
        viewport.select(Some(9));
        viewport.set_offset(9);
        assert!(!viewport.clamp(5));
        assert_eq!(viewport.selected(), Some(4));
        assert_eq!(viewport.offset(), 4);
        assert!(viewport.clamp(0));
        assert_eq!(viewport.selected(), None);
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn scroll_by_follows_offset_with_selection() {
        let viewport = viewport_with_area();
        viewport.select(Some(0));
        viewport.set_offset(0);
        viewport.scroll_by(1, 10);
        assert_eq!((viewport.selected(), viewport.offset()), (Some(1), 1));
        viewport.scroll_by(-5, 10);
        assert_eq!((viewport.selected(), viewport.offset()), (Some(0), 0));
        viewport.scroll_by(99, 10);
        assert_eq!((viewport.selected(), viewport.offset()), (Some(9), 9));
    }

    #[test]
    fn page_by_moves_one_viewport_from_selection() {
        let viewport = viewport_with_area();
        viewport.select(Some(2));
        viewport.set_offset(2);
        viewport.page_by(1, 100);
        // ponytail: height 18 fixed here; multi-page chaining covered by scroll path.
        assert_eq!((viewport.selected(), viewport.offset()), (Some(20), 20));
        viewport.page_by(-1, 100);
        assert_eq!((viewport.selected(), viewport.offset()), (Some(2), 2));
    }

    #[test]
    fn rebuild_layout_jump_snaps_offset() {
        let viewport = viewport_with_area();
        assert!(!viewport.rebuild_layout(10, Some(RebuildTarget::Jump(7))));
        assert_eq!((viewport.selected(), viewport.offset()), (Some(7), 7));
        assert!(!viewport.rebuild_layout(10, Some(RebuildTarget::Preserve { idx: 4, offset: 3 })));
        assert_eq!((viewport.selected(), viewport.offset()), (Some(4), 3));
    }

    #[test]
    fn note_visible_or_defer_covers_missing_and_hidden() {
        let viewport = viewport_with_area();
        viewport.set_offset(0);
        viewport.note_code_visible_or_defer(2, None);
        assert_eq!(viewport.take_target(), Some(2));
        viewport.note_code_visible_or_defer(1, Some(1));
        assert_eq!(viewport.take_target(), None);
        assert_eq!(viewport.selected(), Some(1));
        viewport.note_code_visible_or_defer(1, Some(50));
        assert_eq!(viewport.take_target(), Some(1));
        assert_eq!((viewport.selected(), viewport.offset()), (Some(50), 50));
    }

    #[test]
    fn gutter_for_consumes_on_navigate_away() {
        let viewport = viewport_with_area();
        viewport.prefer_status_gutter_for(1);
        assert_eq!(viewport.gutter_for(Some(1)), Some(1));
        assert_eq!(viewport.gutter_for(Some(2)), None);
        assert_eq!(viewport.gutter_for(Some(2)), None);
    }

    #[test]
    fn grown_bottom_offset_follows_only_past_viewport() {
        let viewport = viewport_with_area();
        assert_eq!(viewport.grown_bottom_offset(0, 30, 10, 5), 30 + 1 - 18);
        assert_eq!(viewport.grown_bottom_offset(0, 10, 10, 5), 0);
        assert_eq!(viewport.grown_bottom_offset(0, 30, 5, 5), 0);
    }

    #[test]
    fn inline_pty_uses_all_rows_below_visible_source() {
        assert_eq!(inline_pty_rows(40, 4, 5, 0), (35, 0));
    }

    #[test]
    fn inline_pty_scrolls_minimally_to_reserve_proportional_height() {
        assert_eq!(inline_pty_rows(40, 38, 5, 0), (16, 15));
    }

    #[test]
    fn inline_pty_caps_target_when_source_nearly_fills_viewport() {
        assert_eq!(inline_pty_rows(40, 35, 36, 0), (4, 0));
    }
}
