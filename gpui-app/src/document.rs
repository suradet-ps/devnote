//! Document model: text buffer, caret/selection, undo history, line index.
//!
//! This is a plain `String` buffer (not a rope) - appropriate for the notepad
//! scope. The line-start index is rebuilt on every mutation; for the file
//! sizes this app targets (<= 10 MB soft limit) that stays well under a frame.

use std::ops::Range;
use std::path::PathBuf;

/// How many undo snapshots to keep.
const UNDO_LIMIT: usize = 500;

#[derive(Clone)]
pub struct Document {
    pub path: Option<PathBuf>,
    pub file_name: String,
    pub content: String,
    saved_content: String,
    pub encoding: String,
    /// False when detection had low confidence (surfaced as a toast).
    #[allow(dead_code)]
    pub encoding_confident: bool,
    pub line_ending: String,
    pub read_only: bool,

    /// Caret position in bytes into `content`.
    pub caret: usize,
    /// Selection anchor in bytes; equal to `caret` when there is no selection.
    pub anchor: usize,
    /// IME composition range, if any.
    pub marked_range: Option<Range<usize>>,
    /// Goal column while moving vertically (kept during consecutive up/down).
    pub goal_col: Option<usize>,

    cursor_line: usize,
    line_starts: Vec<usize>,

    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
}

/// One reversible edit: the replaced byte range, the text that was removed,
/// the text that was inserted, and the caret/anchor before the edit.
#[derive(Clone)]
struct Snapshot {
    range: Range<usize>,
    removed: String,
    inserted: String,
    caret_before: usize,
    anchor_before: usize,
}

impl Document {
    pub fn new_untitled(index: u32) -> Self {
        let content = String::new();
        let mut doc = Self {
            path: None,
            file_name: format!("untitled-{index}"),
            content,
            saved_content: String::new(),
            encoding: "UTF-8".to_string(),
            encoding_confident: true,
            line_ending: "LF".to_string(),
            read_only: false,
            caret: 0,
            anchor: 0,
            marked_range: None,
            goal_col: None,
            cursor_line: 0,
            line_starts: vec![0],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        doc.rebuild_lines();
        doc
    }

    pub fn from_payload(payload: crate::files::FilePayload) -> Self {
        let path = PathBuf::from(&payload.path);
        let mut doc = Self {
            path: Some(path),
            file_name: payload.file_name,
            content: payload.content,
            saved_content: String::new(),
            encoding: payload.encoding,
            encoding_confident: payload.encoding_confident,
            line_ending: payload.line_ending,
            read_only: payload.preview,
            caret: 0,
            anchor: 0,
            marked_range: None,
            goal_col: None,
            cursor_line: 0,
            line_starts: vec![0],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        doc.saved_content = doc.content.clone();
        doc.rebuild_lines();
        doc
    }

    // -- queries -----------------------------------------------------------

    pub fn is_dirty(&self) -> bool {
        self.content != self.saved_content
    }

    pub fn mark_saved(&mut self) {
        self.saved_content = self.content.clone();
    }

    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    pub fn has_selection(&self) -> bool {
        self.anchor != self.caret
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    /// 1-based (line, column) using character columns (Thai-friendly).
    pub fn cursor_line_col(&self) -> (usize, usize) {
        let line = self.cursor_line;
        let start = self.line_starts[line];
        let col = self.content[start..self.caret].chars().count();
        (line + 1, col + 1)
    }

    pub fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.content.len());
        let mut end_content = end;
        // Exclude the trailing newline sequence from the displayed range.
        if end_content > start {
            if self.content.as_bytes().get(end_content - 1) == Some(&b'\n') {
                end_content -= 1;
            }
            if end_content > start && self.content.as_bytes().get(end_content - 1) == Some(&b'\r') {
                end_content -= 1;
            }
        }
        start..end_content
    }

    /// Visible line text (without trailing line ending).
    pub fn line_text(&self, line: usize) -> &str {
        let range = self.line_range(line);
        &self.content[range]
    }

    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }

    pub fn char_count(&self) -> usize {
        self.content.chars().count()
    }

    fn line_of(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next.saturating_sub(1),
        }
    }

    fn rebuild_lines(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        for (i, byte) in self.content.bytes().enumerate() {
            if byte == b'\n' {
                self.line_starts.push(i + 1);
            }
        }
        self.cursor_line = self.line_of(self.caret.min(self.content.len()));
    }

    // -- mutation ----------------------------------------------------------

    /// Record an edit before it is applied. Stores a delta (range + removed
    /// text) instead of cloning the whole buffer, so each keystroke costs
    /// O(edit size) rather than O(document size).
    fn snapshot(&mut self, range: &Range<usize>, inserted: &str) {
        if self.read_only {
            return;
        }
        self.undo_stack.push(Snapshot {
            range: range.clone(),
            removed: self.content[range.clone()].to_string(),
            inserted: inserted.to_string(),
            caret_before: self.caret,
            anchor_before: self.anchor,
        });
        if self.undo_stack.len() > UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if self.read_only {
            return;
        }
        if let Some(entry) = self.undo_stack.pop() {
            let start = entry.range.start;
            let end = (start + entry.inserted.len()).min(self.content.len());
            self.content.replace_range(start..end, &entry.removed);
            self.caret = entry.caret_before.min(self.content.len());
            self.anchor = entry.anchor_before.min(self.content.len());
            self.marked_range = None;
            self.rebuild_lines();
            self.redo_stack.push(entry);
        }
    }

    pub fn redo(&mut self) {
        if self.read_only {
            return;
        }
        if let Some(entry) = self.redo_stack.pop() {
            let start = entry.range.start;
            let end = (start + entry.removed.len()).min(self.content.len());
            self.content.replace_range(start..end, &entry.inserted);
            // `replace_range` leaves the caret after the inserted text.
            self.caret = (start + entry.inserted.len()).min(self.content.len());
            self.anchor = self.caret;
            self.marked_range = None;
            self.rebuild_lines();
            self.undo_stack.push(entry);
        }
    }

    /// Replace `range` with `text`, leaving the caret after the insertion.
    pub fn replace_range(&mut self, range: Range<usize>, text: &str) {
        if self.read_only {
            return;
        }
        let range = clamp_range(range, self.content.len());
        self.snapshot(&range, text);
        self.content.replace_range(range.clone(), text);
        self.caret = range.start + text.len();
        self.anchor = self.caret;
        self.marked_range = None;
        self.goal_col = None;
        self.rebuild_lines();
    }

    /// Delete the current selection, or the grapheme before the caret.
    pub fn backspace(&mut self) {
        if self.has_selection() {
            let selection = self.selection();
            self.replace_range(selection, "");
            return;
        }
        let prev = self.prev_boundary(self.caret);
        if prev != self.caret {
            self.replace_range(prev..self.caret, "");
        }
    }

    /// Delete the grapheme after the caret (or the selection).
    pub fn delete_forward(&mut self) {
        if self.has_selection() {
            let selection = self.selection();
            self.replace_range(selection, "");
            return;
        }
        let next = self.next_boundary(self.caret);
        if next != self.caret {
            self.replace_range(self.caret..next, "");
        }
    }

    // -- caret movement ----------------------------------------------------

    pub fn move_to(&mut self, offset: usize, selecting: bool) {
        self.caret = offset.min(self.content.len());
        if !selecting {
            self.anchor = self.caret;
        }
        self.marked_range = None;
        self.goal_col = None;
        self.cursor_line = self.line_of(self.caret);
    }

    pub fn select_to(&mut self, offset: usize) {
        self.caret = offset.min(self.content.len());
        self.marked_range = None;
        self.goal_col = None;
        self.cursor_line = self.line_of(self.caret);
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.content.len();
        self.cursor_line = self.line_of(self.caret);
    }

    pub fn prev_boundary(&self, offset: usize) -> usize {
        use unicode_segmentation::UnicodeSegmentation;
        self.content[..offset]
            .grapheme_indices(true)
            .next_back()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    pub fn next_boundary(&self, offset: usize) -> usize {
        use unicode_segmentation::UnicodeSegmentation;
        self.content[offset..]
            .grapheme_indices(true)
            .nth(1)
            .map(|(idx, _)| offset + idx)
            .unwrap_or(self.content.len())
    }

    pub fn move_left(&mut self, selecting: bool) {
        if self.has_selection() && !selecting {
            self.move_to(self.selection().start, false);
            return;
        }
        let target = self.prev_boundary(self.caret);
        self.move_to(target, selecting);
    }

    pub fn move_right(&mut self, selecting: bool) {
        if self.has_selection() && !selecting {
            self.move_to(self.selection().end, false);
            return;
        }
        let target = self.next_boundary(self.caret);
        self.move_to(target, selecting);
    }

    pub fn move_home(&mut self, selecting: bool) {
        let line = self.cursor_line;
        let start = self.line_starts[line];
        // First press goes to first non-whitespace (notepad-style), second to 0.
        let first_non_ws = self.content[start..self.caret]
            .find(|c: char| !c.is_whitespace())
            .map(|i| start + i)
            .unwrap_or(start);
        let target = if self.caret != start && self.caret > first_non_ws {
            first_non_ws
        } else {
            start
        };
        self.move_to(target, selecting);
    }

    pub fn move_end(&mut self, selecting: bool) {
        let line = self.cursor_line;
        let range = self.line_range(line);
        self.move_to(range.end, selecting);
    }

    pub fn move_doc_start(&mut self, selecting: bool) {
        self.move_to(0, selecting);
    }

    pub fn move_doc_end(&mut self, selecting: bool) {
        let end = self.content.len();
        self.move_to(end, selecting);
    }

    pub fn move_up(&mut self, selecting: bool) {
        let line = self.cursor_line;
        let col = self.goal_col.unwrap_or_else(|| self.char_col());
        if line == 0 {
            return;
        }
        let target = self.offset_for_line_col(line - 1, col);
        self.caret = target;
        if !selecting {
            self.anchor = self.caret;
        }
        self.goal_col = Some(col);
        self.cursor_line = self.line_of(self.caret);
        self.marked_range = None;
    }

    pub fn move_down(&mut self, selecting: bool) {
        let line = self.cursor_line;
        let col = self.goal_col.unwrap_or_else(|| self.char_col());
        if line + 1 >= self.line_count() {
            return;
        }
        let target = self.offset_for_line_col(line + 1, col);
        self.caret = target;
        if !selecting {
            self.anchor = self.caret;
        }
        self.goal_col = Some(col);
        self.cursor_line = self.line_of(self.caret);
        self.marked_range = None;
    }

    /// Scroll-page movement: move `lines` visual lines up/down.
    pub fn move_page(&mut self, lines: i64, selecting: bool) {
        let line = self.cursor_line;
        let col = self.char_col();
        let target_line = (line as i64 + lines).clamp(0, self.line_count() as i64 - 1) as usize;
        let target = self.offset_for_line_col(target_line, col);
        self.caret = target;
        if !selecting {
            self.anchor = self.caret;
        }
        self.cursor_line = self.line_of(self.caret);
        self.goal_col = None;
    }

    fn char_col(&self) -> usize {
        let start = self.line_starts[self.cursor_line];
        self.content[start..self.caret].chars().count()
    }

    fn offset_for_line_col(&self, line: usize, col: usize) -> usize {
        let range = self.line_range(line);
        let line_end = range.end;
        let mut offset = range.start;
        let mut remaining = col;
        for ch in self.content[range].chars() {
            if remaining == 0 {
                break;
            }
            offset += ch.len_utf8();
            remaining -= 1;
        }
        if remaining > 0 && line + 1 < self.line_count() {
            // Column past end of line: clamp to end of line (not past newline).
            return line_end;
        }
        offset
    }

    // -- UTF-16 conversion (for IME / accessibility APIs) ------------------

    pub fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    pub fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    pub fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

fn clamp_range(range: Range<usize>, len: usize) -> Range<usize> {
    range.start.min(len)..range.end.min(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(content: &str) -> Document {
        let mut doc = Document::new_untitled(1);
        doc.replace_range(0..0, content);
        doc.move_to(0, false);
        doc
    }

    #[test]
    fn line_starts_and_ranges() {
        let d = doc("ab\ncd\r\nef");
        assert_eq!(d.line_count(), 3);
        assert_eq!(d.line_text(0), "ab");
        assert_eq!(d.line_text(1), "cd");
        assert_eq!(d.line_text(2), "ef");
    }

    #[test]
    fn dirty_tracking() {
        let mut d = doc("hello");
        assert!(d.is_dirty());
        d.mark_saved();
        assert!(!d.is_dirty());
        d.replace_range(0..0, "x");
        assert!(d.is_dirty());
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut d = doc("hello");
        d.mark_saved();
        d.replace_range(5..5, " world");
        assert_eq!(d.content, "hello world");
        d.undo();
        assert_eq!(d.content, "hello");
        d.redo();
        assert_eq!(d.content, "hello world");
    }

    #[test]
    fn undo_redo_across_multiple_edits() {
        let mut d = doc("hello");
        d.mark_saved();
        d.replace_range(5..5, " world"); // "hello world"
        d.replace_range(0..5, "goodbye"); // "goodbye world"
        d.undo();
        assert_eq!(d.content, "hello world");
        d.undo();
        assert_eq!(d.content, "hello");
        d.redo();
        assert_eq!(d.content, "hello world");
        d.redo();
        assert_eq!(d.content, "goodbye world");
        // A fresh edit clears the redo stack.
        d.undo();
        d.replace_range(0..0, "x");
        d.redo();
        assert_eq!(d.content, "xhello world");
    }

    #[test]
    fn undo_restores_caret_before_edit() {
        let mut d = doc("hello");
        d.move_to(2, false);
        d.replace_range(2..2, "XY");
        assert_eq!(d.content, "heXYllo");
        assert_eq!(d.caret, 4);
        d.undo();
        assert_eq!(d.content, "hello");
        assert_eq!(d.caret, 2);
    }

    #[test]
    fn vertical_movement_keeps_goal_column() {
        let mut d = doc("long line here\nab\nlonger second line");
        d.move_to(10, false);
        d.move_down(false);
        // Column clamped on the short line, goal column remembered.
        assert_eq!(d.cursor_line(), 1);
        d.move_down(false);
        assert_eq!(d.cursor_line(), 2);
        let (_, col) = d.cursor_line_col();
        assert_eq!(col, 11);
    }

    #[test]
    fn utf16_round_trip_with_thai_text() {
        let d = doc("เนเธ—เธข abc");
        let utf16 = d.range_to_utf16(&(0..d.content.len()));
        let back = d.range_from_utf16(&utf16);
        assert_eq!(back, 0..d.content.len());
        assert_eq!(utf16.end, "เนเธ—เธข abc".encode_utf16().count());
    }

    #[test]
    fn home_goes_to_first_non_whitespace_then_start() {
        let mut d = doc("    indented");
        d.move_to(8, false);
        d.move_home(false);
        assert_eq!(d.caret, 4);
        d.move_home(false);
        assert_eq!(d.caret, 0);
    }
}
