//! `DevNoteApp` - the root view. Owns the documents, drives file operations,
//! renders the chrome (tab bar, welcome screen, status bar, find panel,
//! dialogs, toasts) and implements `EntityInputHandler` so IME text (Thai,
//! CJK, ...) flows into the active document.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, FocusHandle, Focusable,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, UTF16Selection, Window,
    WindowAppearance, div, point, prelude::*, px, size,
};

use crate::actions::*;
use crate::document::Document;
use crate::editor;
use crate::files::{self, FilePayload};
use crate::find::{self, SearchOptions};
use crate::i18n::I18n;
use crate::settings::{Settings, ThemeChoice};
use crate::text_input::TextInput;
use crate::theme::Theme;

#[derive(Default)]
pub struct FindState {
    pub open: bool,
    pub replace_open: bool,
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub matches: Vec<Range<usize>>,
    pub current: Option<usize>,
    pub invalid_regex: bool,
    query_seen: String,
    replacement_seen: String,
    flags_seen: (bool, bool),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DialogKind {
    CloseWindow,
    CloseTab,
}

struct Toast {
    id: u64,
    text: String,
    error: bool,
}

pub struct DevNoteApp {
    pub docs: Vec<Document>,
    pub active: usize,
    untitled_counter: u32,

    pub theme: Theme,
    theme_choice: ThemeChoice,
    dark: bool,
    pub font_size: f32,
    pub show_line_numbers: bool,
    pub i18n: I18n,

    pub editor_focus_handle: FocusHandle,
    pub editor_scroll: gpui::ScrollHandle,
    pub editor_metrics: Option<editor::EditorMetrics>,

    pub find: FindState,
    find_input: Entity<TextInput>,
    replace_input: Entity<TextInput>,

    dialog: Option<DialogKind>,
    toasts: Vec<Toast>,
    next_toast_id: u64,

    recent: Vec<String>,
    settings: Settings,
    selecting: bool,
    last_title: String,
}

impl DevNoteApp {
    pub fn new(settings: Settings, cx: &mut Context<Self>) -> Self {
        let theme = Theme::light();
        let locale = settings.locale();
        let font = editor::ui_font();
        let find_input =
            cx.new(|cx| TextInput::new("", font.clone(), 13.0, theme.ink, theme.primary, cx));
        let replace_input =
            cx.new(|cx| TextInput::new("", font.clone(), 13.0, theme.ink, theme.primary, cx));
        let recent: Vec<String> = files::read_json("recent_files.json").unwrap_or_default();

        let mut app = Self {
            docs: Vec::new(),
            active: 0,
            untitled_counter: 0,
            theme,
            theme_choice: settings.theme,
            dark: false,
            font_size: settings.font_size,
            show_line_numbers: settings.show_line_numbers,
            i18n: I18n::new(locale),
            editor_focus_handle: cx.focus_handle(),
            editor_scroll: gpui::ScrollHandle::new(),
            editor_metrics: None,
            find: FindState::default(),
            find_input,
            replace_input,
            dialog: None,
            toasts: Vec::new(),
            next_toast_id: 1,
            recent,
            settings,
            selecting: false,
            last_title: String::new(),
        };
        app.new_tab_inner();
        app
    }

    // -- documents ---------------------------------------------------------

    pub fn active_doc(&self) -> Option<&Document> {
        self.docs.get(self.active)
    }

    pub fn active_doc_mut(&mut self) -> Option<&mut Document> {
        self.docs.get_mut(self.active)
    }

    fn new_tab_inner(&mut self) {
        self.untitled_counter += 1;
        self.docs
            .push(Document::new_untitled(self.untitled_counter));
        self.active = self.docs.len() - 1;
    }

    fn add_payload(&mut self, payload: FilePayload) -> bool {
        // Focus the tab when the file is already open.
        if let Some(index) = self.docs.iter().position(|d| {
            d.path
                .as_ref()
                .map(|p| p.to_string_lossy() == payload.path.as_str())
                .unwrap_or(false)
        }) {
            self.active = index;
            return false;
        }
        self.docs.push(Document::from_payload(payload));
        self.active = self.docs.len() - 1;
        true
    }

    // -- theme / title / toasts -------------------------------------------

    fn sync_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dark = match self.theme_choice {
            ThemeChoice::Light => false,
            ThemeChoice::Dark => true,
            ThemeChoice::System => matches!(
                window.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
        };
        if dark != self.dark {
            self.dark = dark;
            self.theme = Theme::for_dark(dark);
            let theme = self.theme;
            for input in [&self.find_input, &self.replace_input] {
                input.update(cx, |input, cx| {
                    input.set_colors(theme.ink, theme.primary, cx);
                });
            }
        }
    }

    fn sync_title(&mut self, window: &mut Window) {
        let title = match self.active_doc() {
            Some(doc) => {
                let dirty = if doc.is_dirty() { "* " } else { "" };
                format!("{dirty}{} - DevNote", doc.file_name)
            }
            None => "DevNote".to_string(),
        };
        if title != self.last_title {
            self.last_title = title.clone();
            window.set_window_title(&title);
        }
    }

    fn push_toast(&mut self, text: String, error: bool, cx: &mut Context<Self>) {
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        self.toasts.push(Toast { id, text, error });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(4))
                .await;
            this.update(cx, |view, cx| {
                view.toasts.retain(|t| t.id != id);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    // -- find / replace ----------------------------------------------------

    fn search_options(&self) -> SearchOptions {
        SearchOptions {
            case_sensitive: self.find.case_sensitive,
            use_regex: self.find.use_regex,
        }
    }

    fn recompute_matches(&mut self, cx: &App) {
        let query = self.find_input.read(cx).text().to_string();
        self.find.invalid_regex = self.find.use_regex
            && !query.is_empty()
            && regex::RegexBuilder::new(&query).build().is_err();
        let options = self.search_options();
        let (matches, caret) = match self.active_doc() {
            Some(doc) => (find::find_all(&doc.content, &query, options), doc.caret),
            None => (Vec::new(), 0),
        };
        self.find.matches = matches;
        self.find.current = self
            .find
            .matches
            .iter()
            .position(|m| m.start >= caret)
            .or_else(|| (!self.find.matches.is_empty()).then_some(0));
    }

    /// Poll the find/replace inputs each frame; recompute when they changed.
    fn poll_find_inputs(&mut self, cx: &mut Context<Self>) {
        let query = self.find_input.read(cx).text().to_string();
        let replacement = self.replace_input.read(cx).text().to_string();
        let flags = (self.find.case_sensitive, self.find.use_regex);
        if query != self.find.query_seen
            || replacement != self.find.replacement_seen
            || flags != self.find.flags_seen
        {
            self.find.query_seen = query;
            self.find.replacement_seen = replacement;
            self.find.flags_seen = flags;
            if self.find.open {
                self.recompute_matches(cx);
            }
        }
    }

    fn open_find(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.find.open = true;
        self.find.replace_open = self.find.replace_open || replace;
        // Seed the query with the current selection (one-line selections only).
        if self.find.query_seen.is_empty()
            && let Some(doc) = self.active_doc()
        {
            let selection = doc.selection();
            let selected = &doc.content[selection];
            if !selected.is_empty() && !selected.contains('\n') {
                let selected = selected.to_string();
                self.find_input
                    .update(cx, |input, cx| input.set_text(&selected, cx));
            }
        }
        self.recompute_matches(cx);
        if replace || self.find.replace_open {
            self.replace_input
                .update(cx, |input, _cx| input.focus(window));
        } else {
            self.find_input.update(cx, |input, _cx| input.focus(window));
        }
        cx.notify();
    }

    fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find.open = false;
        self.find.replace_open = false;
        self.find.matches.clear();
        self.find.current = None;
        window.focus(&self.editor_focus_handle);
        cx.notify();
    }

    fn goto_match(&mut self, next: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.matches.is_empty() {
            return;
        }
        let count = self.find.matches.len();
        let current = match self.find.current {
            Some(index) if next => (index + 1) % count,
            Some(index) => (index + count - 1) % count,
            None if next => 0,
            None => count - 1,
        };
        self.find.current = Some(current);
        let range = self.find.matches[current].clone();
        if let Some(doc) = self.active_doc_mut() {
            doc.move_to(range.start, false);
            doc.select_to(range.end);
        }
        self.after_edit(window, cx);
    }

    fn replace_current(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self.find.current else {
            return;
        };
        let Some(range) = self.find.matches.get(current).cloned() else {
            return;
        };
        let replacement = self.replace_input.read(cx).text().to_string();
        let query = self.find_input.read(cx).text().to_string();
        let options = self.search_options();
        if let Some(doc) = self.active_doc_mut() {
            let expanded =
                find::expand_replacement(&doc.content, &query, &replacement, &range, options);
            doc.replace_range(range, &expanded);
        }
        // Recompute first, then select the match the recompute points at;
        // calling `goto_match` here would advance past it a second time.
        self.recompute_matches(cx);
        let selected = if let Some(current) = self.find.current
            && let Some(range) = self.find.matches.get(current).cloned()
        {
            if let Some(doc) = self.active_doc_mut() {
                doc.move_to(range.start, false);
                doc.select_to(range.end);
            }
            Some(current)
        } else {
            None
        };
        self.after_edit(window, cx);
        // `after_edit` re-derives `current` from the caret (which sits at the
        // end of the freshly selected match); keep the selected match current
        // so the highlight and the selection stay in sync.
        if let Some(current) = selected
            && current < self.find.matches.len()
        {
            self.find.current = Some(current);
        }
    }

    fn replace_all_matches(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.find_input.read(cx).text().to_string();
        let replacement = self.replace_input.read(cx).text().to_string();
        let options = self.search_options();
        let Some(doc) = self.active_doc_mut() else {
            return;
        };
        let (content, count) = find::replace_all(&doc.content, &query, &replacement, options);
        if count > 0 {
            let len = content.len();
            doc.replace_range(0..doc.content.len(), &content);
            doc.move_to(len.min(doc.content.len()), false);
        }
        self.recompute_matches(cx);
        self.after_edit(window, cx);
    }

    // -- file operations ---------------------------------------------------

    fn open_file_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let filters: Vec<String> = [
            "txt", "md", "log", "csv", "json", "toml", "yaml", "yml", "xml", "rs", "ts", "tsx",
            "js", "jsx", "py", "html", "css", "sql", "sh", "go", "rb", "c", "cpp", "h", "java",
            "php",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let picked = rfd::FileDialog::new()
            .set_title(self.i18n.t("open"))
            .add_filter("Text files", &filters)
            .add_filter("All files", &["*".to_string()])
            .pick_file();
        if let Some(path) = picked {
            self.open_path(&path.to_string_lossy(), window, cx);
        }
    }

    fn open_path(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        match files::read_file(path) {
            Ok(payload) => {
                let confident = payload.encoding_confident;
                let encoding = payload.encoding.clone();
                if self.add_payload(payload) {
                    self.remember_recent(path);
                }
                if !confident {
                    let message = format!("{} ({})", self.i18n.t("error_low_confidence"), encoding);
                    self.push_toast(message, true, cx);
                }
            }
            Err(error) => {
                let message = format!("{}: {}", self.i18n.t("error_open"), error);
                self.push_toast(message, true, cx);
            }
        }
        self.after_edit(window, cx);
    }

    fn save_doc(
        &mut self,
        index: usize,
        force_as: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(doc) = self.docs.get_mut(index) else {
            return true;
        };
        if doc.read_only {
            let message = self.i18n.t("file_read_only").to_string();
            self.push_toast(message, true, cx);
            return false;
        }

        let mut path = doc.path.as_ref().map(|p| p.to_string_lossy().to_string());
        if force_as || path.is_none() {
            let suggested = doc.file_name.clone();
            let picked = rfd::FileDialog::new()
                .set_title(self.i18n.t("save_as"))
                .set_file_name(&suggested)
                .add_filter("Text files", &["txt".to_string()])
                .add_filter("All files", &["*".to_string()])
                .save_file();
            let Some(picked) = picked else {
                return false;
            };
            path = Some(files::ensure_extension(&picked.to_string_lossy(), "txt"));
        }
        let Some(path) = path else {
            return false;
        };

        let doc = &mut self.docs[index];
        let result = files::save_file(&path, &doc.content, &doc.line_ending, &doc.encoding);
        match result {
            Ok(()) => {
                doc.path = Some(path.clone().into());
                doc.file_name = std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| doc.file_name.clone());
                doc.mark_saved();
                self.remember_recent(&path);
                self.sync_title(window);
                true
            }
            Err(error) => {
                let message = format!("{}: {}", self.i18n.t("error_save"), error);
                self.push_toast(message, true, cx);
                false
            }
        }
    }

    fn save_active(&mut self, force_as: bool, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.active;
        self.save_doc(index, force_as, window, cx);
    }

    fn remember_recent(&mut self, path: &str) {
        let path = files::validate_path(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string());
        self.recent.retain(|p| p != &path);
        self.recent.insert(0, path);
        self.recent.truncate(10);
        let _ = files::write_json("recent_files.json", &self.recent);
    }

    // -- close flow --------------------------------------------------------

    fn any_dirty(&self) -> bool {
        self.docs.iter().any(Document::is_dirty)
    }

    /// Called from `on_window_should_close`; returns whether the window may
    /// close immediately.
    pub fn request_close(&mut self, cx: &mut Context<Self>) -> bool {
        if self.any_dirty() {
            self.dialog = Some(DialogKind::CloseWindow);
            cx.notify();
            false
        } else {
            true
        }
    }

    fn close_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.docs.is_empty() {
            return;
        }
        let dirty = self.active_doc().map(Document::is_dirty).unwrap_or(false);
        if dirty {
            self.dialog = Some(DialogKind::CloseTab);
            cx.notify();
            return;
        }
        self.remove_active_tab(window, cx);
    }

    fn remove_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active < self.docs.len() {
            self.docs.remove(self.active);
            if self.active >= self.docs.len() && self.active > 0 {
                self.active -= 1;
            }
        }
        self.editor_scroll.set_offset(point(px(0.0), px(0.0)));
        self.after_edit(window, cx);
    }

    fn dialog_save_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.dialog else {
            return;
        };
        let targets: Vec<usize> = match kind {
            DialogKind::CloseWindow => self
                .docs
                .iter()
                .enumerate()
                .filter(|(_, d)| d.is_dirty())
                .map(|(i, _)| i)
                .collect(),
            DialogKind::CloseTab => match self.active_doc() {
                Some(doc) if doc.is_dirty() => vec![self.active],
                _ => Vec::new(),
            },
        };
        for index in targets {
            if !self.save_doc(index, false, window, cx) {
                // Save cancelled or failed - keep the dialog open.
                return;
            }
        }
        self.dialog = None;
        match kind {
            DialogKind::CloseWindow => window.remove_window(),
            DialogKind::CloseTab => self.remove_active_tab(window, cx),
        }
    }

    fn dialog_dont_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.dialog else {
            return;
        };
        self.dialog = None;
        match kind {
            DialogKind::CloseWindow => window.remove_window(),
            DialogKind::CloseTab => self.remove_active_tab(window, cx),
        }
    }

    fn dialog_cancel(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        cx.notify();
    }

    // -- editor plumbing ---------------------------------------------------

    pub fn after_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.open {
            self.recompute_matches(cx);
        }
        self.scroll_caret_into_view();
        self.sync_title(window);
        cx.notify();
    }

    fn scroll_caret_into_view(&mut self) {
        let Some(doc) = self.active_doc() else {
            return;
        };
        let line_height = editor::line_height_for(self.font_size);
        let line = doc.cursor_line();
        let caret_top = line as f32 * line_height;
        let caret_bottom = caret_top + line_height;

        let viewport_height = {
            let bounds = self.editor_scroll.bounds();
            let height: f32 = bounds.size.height.into();
            if height > 1.0 {
                height
            } else {
                // Before the first layout, fall back to a conservative guess.
                480.0
            }
        };
        let offset = self.editor_scroll.offset();
        let offset_y: f32 = offset.y.into();
        let mut new_offset = offset_y;
        if caret_top + offset_y < 0.0 {
            new_offset = -caret_top + 8.0;
        } else if caret_bottom + offset_y > viewport_height {
            new_offset = viewport_height - caret_bottom - 8.0;
        }
        if (new_offset - offset_y).abs() > 0.5 {
            self.editor_scroll
                .set_offset(point(offset.x, px(new_offset)));
        }
    }

    fn offset_for_mouse(&self, window: &mut Window, position: Pixels2Point) -> Option<usize> {
        let metrics = self.editor_metrics?;
        let doc = self.active_doc()?;
        Some(metrics.document_offset(doc, self.font_size, window.text_system(), position))
    }

    pub(crate) fn editor_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.editor_focus_handle);
        if let Some(offset) = self.offset_for_mouse(window, event.position) {
            let selecting = event.modifiers.shift;
            if let Some(doc) = self.active_doc_mut() {
                if selecting {
                    doc.select_to(offset);
                } else {
                    doc.move_to(offset, false);
                }
            }
            self.selecting = true;
            self.after_edit(window, cx);
        }
    }

    pub(crate) fn editor_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.selecting {
            return;
        }
        if let Some(offset) = self.offset_for_mouse(window, event.position) {
            if let Some(doc) = self.active_doc_mut() {
                doc.select_to(offset);
            }
            self.after_edit(window, cx);
        }
    }

    pub(crate) fn editor_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.selecting = false;
    }

    // -- editor actions ----------------------------------------------------

    fn with_doc_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: impl FnOnce(&mut Document),
    ) {
        if let Some(doc) = self.active_doc_mut() {
            action(doc);
        }
        self.after_edit(window, cx);
    }

    fn on_backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, Document::backspace);
    }

    fn on_delete_forward(
        &mut self,
        _: &DeleteForward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_doc_action(window, cx, Document::delete_forward);
    }

    fn on_move_left(&mut self, _: &MoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_left(false));
    }

    fn on_move_right(&mut self, _: &MoveRight, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_right(false));
    }

    fn on_move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_up(false));
    }

    fn on_move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_down(false));
    }

    fn on_select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_left(true));
    }

    fn on_select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_right(true));
    }

    fn on_select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_up(true));
    }

    fn on_select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_down(true));
    }

    fn on_move_home(&mut self, _: &MoveHome, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_home(false));
    }

    fn on_move_end(&mut self, _: &MoveEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_end(false));
    }

    fn on_move_doc_start(&mut self, _: &MoveDocStart, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_doc_start(false));
    }

    fn on_move_doc_end(&mut self, _: &MoveDocEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_doc_end(false));
    }

    fn on_page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_page(-30, false));
    }

    fn on_page_down(&mut self, _: &PageDown, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| d.move_page(30, false));
    }

    fn on_select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, Document::select_all);
    }

    fn on_insert_newline(
        &mut self,
        _: &InsertNewline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_doc_action(window, cx, |d| {
            let range = d.selection();
            d.replace_range(range, "\n");
        });
    }

    fn on_insert_tab(&mut self, _: &InsertTab, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, |d| {
            let range = d.selection();
            d.replace_range(range, "    ");
        });
    }

    fn on_copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(doc) = self.active_doc()
            && doc.has_selection()
        {
            let text = doc.content[doc.selection()].to_string();
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.active_doc() else {
            return;
        };
        if !doc.has_selection() {
            return;
        }
        let text = doc.content[doc.selection()].to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.with_doc_action(window, cx, |d| {
            let range = d.selection();
            d.replace_range(range, "");
        });
    }

    fn on_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let normalized = files::normalize_line_endings(&text, "\n");
            self.with_doc_action(window, cx, |d| {
                let range = d.selection();
                d.replace_range(range, &normalized);
            });
        }
    }

    fn on_undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, Document::undo);
    }

    fn on_redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc_action(window, cx, Document::redo);
    }

    // -- global actions ----------------------------------------------------

    fn on_new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.new_tab_inner();
        self.after_edit(window, cx);
    }

    fn on_open_file(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        self.open_file_dialog(window, cx);
    }

    fn on_save_file(&mut self, _: &SaveFile, window: &mut Window, cx: &mut Context<Self>) {
        self.save_active(false, window, cx);
    }

    fn on_save_file_as(&mut self, _: &SaveFileAs, window: &mut Window, cx: &mut Context<Self>) {
        self.save_active(true, window, cx);
    }

    fn on_close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        self.close_tab(window, cx);
    }

    fn on_next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        if !self.docs.is_empty() {
            self.active = (self.active + 1) % self.docs.len();
        }
        self.editor_scroll.set_offset(point(px(0.0), px(0.0)));
        self.after_edit(window, cx);
    }

    fn on_prev_tab(&mut self, _: &PrevTab, window: &mut Window, cx: &mut Context<Self>) {
        if !self.docs.is_empty() {
            self.active = (self.active + self.docs.len() - 1) % self.docs.len();
        }
        self.editor_scroll.set_offset(point(px(0.0), px(0.0)));
        self.after_edit(window, cx);
    }

    fn on_quit(&mut self, _: &QuitApp, window: &mut Window, cx: &mut Context<Self>) {
        // Route quits through the same guarded close flow as the window's
        // close button so dirty documents get their confirmation dialog.
        if self.request_close(cx) {
            window.remove_window();
        }
    }

    fn on_toggle_find(&mut self, _: &ToggleFind, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.open && !self.find.replace_open {
            self.close_find(window, cx);
        } else {
            self.open_find(false, window, cx);
        }
    }

    fn on_toggle_replace(
        &mut self,
        _: &ToggleReplace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.find.open && self.find.replace_open {
            self.close_find(window, cx);
        } else {
            self.open_find(true, window, cx);
        }
    }

    fn on_find_next(&mut self, _: &FindNext, window: &mut Window, cx: &mut Context<Self>) {
        self.goto_match(true, window, cx);
    }

    fn on_find_prev(&mut self, _: &FindPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.goto_match(false, window, cx);
    }

    fn on_replace_current(
        &mut self,
        _: &ReplaceCurrent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_current(window, cx);
    }

    fn on_replace_all_matches(
        &mut self,
        _: &ReplaceAllMatches,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_all_matches(window, cx);
    }

    fn on_submit_input(&mut self, _: &SubmitInput, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.open {
            let replace_focused = self.replace_input.read(cx).is_focused(window);
            if replace_focused {
                self.replace_current(window, cx);
            } else {
                self.goto_match(true, window, cx);
            }
        }
    }

    fn on_close_overlay(&mut self, _: &CloseOverlay, window: &mut Window, cx: &mut Context<Self>) {
        if self.dialog.is_some() {
            self.dialog_cancel(cx);
        } else if self.find.open {
            self.close_find(window, cx);
        }
    }

    fn on_increase_font(
        &mut self,
        _: &IncreaseFontSize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.font_size = (self.font_size + 1.0).min(32.0);
        self.persist_font_size(cx);
        self.after_edit(window, cx);
    }

    fn on_decrease_font(
        &mut self,
        _: &DecreaseFontSize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.font_size = (self.font_size - 1.0).max(9.0);
        self.persist_font_size(cx);
        self.after_edit(window, cx);
    }

    fn on_reset_font(&mut self, _: &ResetFontSize, window: &mut Window, cx: &mut Context<Self>) {
        self.font_size = 14.0;
        self.persist_font_size(cx);
        self.after_edit(window, cx);
    }

    fn persist_font_size(&mut self, cx: &mut Context<Self>) {
        self.settings.font_size = self.font_size;
        self.persist_settings(cx);
    }

    fn persist_settings(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.settings.save() {
            let message = format!("{}: {}", self.i18n.t("error_save"), error);
            self.push_toast(message, true, cx);
        }
    }

    fn on_toggle_theme(&mut self, _: &ToggleTheme, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_choice = match self.theme_choice {
            ThemeChoice::System => ThemeChoice::Light,
            ThemeChoice::Light => ThemeChoice::Dark,
            ThemeChoice::Dark => ThemeChoice::System,
        };
        self.settings.theme = self.theme_choice;
        self.persist_settings(cx);
        self.sync_theme(window, cx);
        cx.notify();
    }

    // -- tabs --------------------------------------------------------------

    fn activate_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.docs.len() {
            self.active = index;
            self.editor_scroll.set_offset(point(px(0.0), px(0.0)));
            self.after_edit(window, cx);
        }
    }

    fn close_tab_at(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.docs.len() {
            self.active = index;
            self.close_tab(window, cx);
        }
    }
}

/// Alias kept for readability of mouse positions.
type Pixels2Point = gpui::Point<Pixels>;

/// Callback shape used by the find-panel buttons.
type DevNoteAction = Box<dyn Fn(&mut DevNoteApp, &mut Window, &mut Context<DevNoteApp>)>;

impl EntityInputHandler for DevNoteApp {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let doc = self.active_doc()?;
        let range = doc.range_from_utf16(&range_utf16);
        actual_range.replace(doc.range_to_utf16(&range));
        Some(doc.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let doc = self.active_doc()?;
        Some(UTF16Selection {
            range: doc.range_to_utf16(&doc.selection()),
            reversed: doc.anchor > doc.caret,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let doc = self.active_doc()?;
        doc.marked_range
            .as_ref()
            .map(|range| doc.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(doc) = self.active_doc_mut() {
            doc.marked_range = None;
        }
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self.active_doc_mut() else {
            return;
        };
        if doc.read_only {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|utf16| doc.range_from_utf16(utf16))
            .or(doc.marked_range.clone())
            .unwrap_or_else(|| doc.selection());
        doc.replace_range(range, new_text);
        self.after_edit(window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self.active_doc_mut() else {
            return;
        };
        if doc.read_only {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|utf16| doc.range_from_utf16(utf16))
            .or(doc.marked_range.clone())
            .unwrap_or_else(|| doc.selection());
        let start = range.start;
        doc.replace_range(range, new_text);
        if !new_text.is_empty() {
            doc.marked_range = Some(start..start + new_text.len());
        }
        if let Some(selected_utf16) = new_selected_range_utf16 {
            let selected = doc.range_from_utf16(&selected_utf16);
            doc.caret = (start + selected.end).min(doc.content.len());
            doc.anchor = (start + selected.start).min(doc.content.len());
        }
        self.after_edit(window, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let metrics = self.editor_metrics?;
        let line = self.active_doc()?.cursor_line();
        let line_height = px(metrics.line_height);
        let top = metrics.content_bounds.top() + px(line as f32 * metrics.line_height);
        Some(Bounds::new(
            point(bounds.left(), top),
            size(bounds.size.width, line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.offset_for_mouse(window, point)?;
        Some(self.active_doc()?.offset_to_utf16(offset))
    }
}

impl Focusable for DevNoteApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.editor_focus_handle.clone()
    }
}

// ---------------------------------------------------------------------------
// rendering
// ---------------------------------------------------------------------------

impl Render for DevNoteApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_theme(window, cx);
        self.poll_find_inputs(cx);
        self.sync_title(window);

        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(self.theme.canvas)
            .text_color(self.theme.ink)
            .font_family(editor::ui_font().family);

        root = root
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_open_file))
            .on_action(cx.listener(Self::on_save_file))
            .on_action(cx.listener(Self::on_save_file_as))
            .on_action(cx.listener(Self::on_close_tab))
            .on_action(cx.listener(Self::on_next_tab))
            .on_action(cx.listener(Self::on_prev_tab))
            .on_action(cx.listener(Self::on_quit))
            .on_action(cx.listener(Self::on_toggle_find))
            .on_action(cx.listener(Self::on_toggle_replace))
            .on_action(cx.listener(Self::on_find_next))
            .on_action(cx.listener(Self::on_find_prev))
            .on_action(cx.listener(Self::on_replace_current))
            .on_action(cx.listener(Self::on_replace_all_matches))
            .on_action(cx.listener(Self::on_submit_input))
            .on_action(cx.listener(Self::on_close_overlay))
            .on_action(cx.listener(Self::on_increase_font))
            .on_action(cx.listener(Self::on_decrease_font))
            .on_action(cx.listener(Self::on_reset_font))
            .on_action(cx.listener(Self::on_toggle_theme))
            .on_action(cx.listener(Self::on_backspace))
            .on_action(cx.listener(Self::on_delete_forward))
            .on_action(cx.listener(Self::on_move_left))
            .on_action(cx.listener(Self::on_move_right))
            .on_action(cx.listener(Self::on_move_up))
            .on_action(cx.listener(Self::on_move_down))
            .on_action(cx.listener(Self::on_select_left))
            .on_action(cx.listener(Self::on_select_right))
            .on_action(cx.listener(Self::on_select_up))
            .on_action(cx.listener(Self::on_select_down))
            .on_action(cx.listener(Self::on_move_home))
            .on_action(cx.listener(Self::on_move_end))
            .on_action(cx.listener(Self::on_move_doc_start))
            .on_action(cx.listener(Self::on_move_doc_end))
            .on_action(cx.listener(Self::on_page_up))
            .on_action(cx.listener(Self::on_page_down))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_insert_newline))
            .on_action(cx.listener(Self::on_insert_tab))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo));

        root = root.child(self.render_tab_bar(cx));

        let mut main_area = div().relative().flex().flex_col().flex_1().min_h_0();
        if self.docs.is_empty() {
            main_area = main_area.child(self.render_welcome(window, cx));
        } else {
            main_area = main_area.child(editor::render_editor(self, cx));
        }
        if self.find.open {
            main_area = main_area.child(self.render_find_panel(window, cx));
        }
        if self.dialog.is_some() {
            main_area = main_area.child(self.render_dialog(window, cx));
        }
        if !self.toasts.is_empty() {
            main_area = main_area.child(self.render_toasts(cx));
        }
        root = root.child(main_area);

        if !self.docs.is_empty() {
            root = root.child(self.render_status_bar(cx));
        }
        root
    }
}

impl DevNoteApp {
    fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut bar = div()
            .flex()
            .flex_row()
            .items_center()
            .h(px(36.0))
            .w_full()
            .bg(theme.surface_dark)
            .text_size(px(13.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .child(div().size(px(8.0)).rounded_full().bg(theme.primary))
                    .child(
                        div()
                            .text_color(theme.on_dark)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("DevNote"),
                    ),
            );

        let mut tabs = div().flex().flex_row().items_center().h_full();
        for (index, doc) in self.docs.iter().enumerate() {
            let is_active = index == self.active;
            let dirty = doc.is_dirty();
            let name = doc.file_name.clone();
            let tab_bg = if is_active {
                theme.canvas
            } else {
                theme.surface_dark_soft
            };
            let text_color = if is_active {
                theme.ink
            } else {
                theme.on_dark_soft
            };

            let mut tab = div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .h_full()
                .px_3()
                .max_w(px(180.0))
                .overflow_hidden()
                .bg(tab_bg)
                .text_color(text_color)
                .cursor_pointer()
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseUpEvent, window, cx| {
                        this.activate_tab(index, window, cx);
                    }),
                )
                .child(div().whitespace_nowrap().child(name));

            if dirty {
                tab = tab.child(div().size(px(6.0)).rounded_full().bg(theme.primary));
            }

            tab = tab.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(16.0))
                    .rounded_sm()
                    .hover(|style| style.bg(theme.surface_cream_strong.opacity(0.4)))
                    .cursor_pointer()
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseUpEvent, window, cx| {
                            cx.stop_propagation();
                            this.close_tab_at(index, window, cx);
                        }),
                    )
                    .child(div().text_size(px(11.0)).child("x")),
            );

            tabs = tabs.child(tab);
        }

        bar = bar
            .child(div().flex_1().min_w_0().overflow_hidden().child(tabs))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(24.0))
                    .mx_2()
                    .rounded_sm()
                    .text_color(theme.on_dark_soft)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.surface_dark_soft))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseUpEvent, window, cx| {
                            this.on_new_file(&NewFile, window, cx);
                        }),
                    )
                    .child("+"),
            );
        bar
    }

    fn render_welcome(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let recent = self.recent.clone();
        let recent_rows = recent.iter().take(5).cloned().collect::<Vec<_>>();

        let mut card = div()
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .max_w(px(520.0))
            .child(
                div()
                    .text_size(px(30.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.ink)
                    .child(self.i18n.t("welcome_title")),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(theme.muted)
                    .text_center()
                    .child(self.i18n.t("welcome_subtitle")),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(theme.primary)
                            .text_color(theme.on_primary)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.primary_active))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseUpEvent, window, cx| {
                                    this.on_new_file(&NewFile, window, cx);
                                }),
                            )
                            .child(self.i18n.t("new_file")),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(theme.canvas)
                            .border_1()
                            .border_color(theme.hairline)
                            .text_color(theme.ink)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.surface_soft))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseUpEvent, window, cx| {
                                    this.on_open_file(&OpenFile, window, cx);
                                }),
                            )
                            .child(self.i18n.t("open_file")),
                    ),
            );

        if recent_rows.is_empty() {
            card = card.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.muted_soft)
                    .child(self.i18n.t("no_recent")),
            );
        } else {
            let mut list = div().flex().flex_col().w_full().mt_4().gap_px().child(
                div()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_soft)
                    .mb_1()
                    .child(self.i18n.t("recent_files")),
            );
            for path in recent_rows {
                let label = std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                let full = path.clone();
                list = list.child(
                    div()
                        .flex()
                        .flex_col()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.surface_soft))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseUpEvent, window, cx| {
                                this.open_path(&full, window, cx);
                            }),
                        )
                        .child(div().text_color(theme.ink).child(label))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.muted_soft)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .child(path),
                        ),
                );
            }
            card = card.child(list);
        }

        card = card.child(
            div()
                .text_size(px(11.0))
                .text_color(theme.muted_soft)
                .mt_4()
                .child(self.i18n.t("shortcuts_hint")),
        );

        div()
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .bg(theme.canvas)
            .child(card)
    }

    fn render_status_bar(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(doc) = self.active_doc() else {
            return div();
        };
        let (line, col) = doc.cursor_line_col();
        let language = files::language_for(doc.path.as_deref());
        let encoding = doc.encoding.clone();
        let line_ending = doc.line_ending.clone();
        let words = doc.word_count();
        let chars = doc.char_count();
        let read_only = doc.read_only;
        let match_info = if self.find.open && !self.find.query_seen.is_empty() {
            let current = self.find.current.map(|i| i + 1).unwrap_or(0);
            Some(format!("{current}/{}", self.find.matches.len()))
        } else {
            None
        };

        let cell = |text: String| div().child(text);

        let mut left = div().flex().flex_row().items_center().gap_3();
        left = left.child(cell(language.to_string()));
        if read_only {
            left = left.child(
                div()
                    .text_color(theme.error)
                    .child(self.i18n.t("file_read_only")),
            );
        }
        if let Some(info) = match_info {
            left = left.child(cell(info));
        }

        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(26.0))
            .w_full()
            .px_3()
            .bg(theme.surface_soft)
            .border_t_1()
            .border_color(theme.hairline)
            .text_size(px(11.0))
            .text_color(theme.muted)
            .child(left)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(cell(encoding))
                    .child(cell(line_ending))
                    .child(cell(format!("Ln {line}, Col {col}")))
                    .child(cell(format!("{words} {}", self.i18n.t("words"))))
                    .child(cell(format!("{chars} {}", self.i18n.t("chars")))),
            )
    }

    fn render_find_panel(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let replace_open = self.find.replace_open;
        let case_sensitive = self.find.case_sensitive;
        let use_regex = self.find.use_regex;
        let invalid_regex = self.find.invalid_regex;
        let status_text = if self.find.query_seen.is_empty() {
            String::new()
        } else if invalid_regex {
            "invalid pattern".to_string()
        } else if self.find.matches.is_empty() {
            self.i18n.t("no_results").to_string()
        } else {
            let current = self.find.current.map(|i| i + 1).unwrap_or(0);
            format!("{current}/{}", self.find.matches.len())
        };

        let chip = |label: &'static str, active: bool| {
            let mut chip = div()
                .px_2()
                .py_1()
                .rounded_sm()
                .text_size(px(11.0))
                .cursor_pointer();
            if active {
                chip = chip.bg(theme.primary).text_color(theme.on_primary);
            } else {
                chip = chip
                    .bg(theme.surface_cream_strong)
                    .text_color(theme.muted)
                    .hover(|style| style.bg(theme.surface_soft));
            }
            chip.child(label)
        };

        let case_chip = chip("Aa", case_sensitive).on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _w, cx| {
                this.find.case_sensitive = !this.find.case_sensitive;
                cx.notify();
            }),
        );
        let regex_chip = chip(".*", use_regex).on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _w, cx| {
                this.find.use_regex = !this.find.use_regex;
                cx.notify();
            }),
        );

        let mut column = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().child(self.find_input.clone()))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(if invalid_regex {
                                theme.error
                            } else {
                                theme.muted
                            })
                            .min_w(px(48.0))
                            .child(status_text),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(case_chip)
                    .child(regex_chip)
                    .child(div().flex_1())
                    .child(self.panel_button(
                        self.i18n.t("prev"),
                        Box::new(|this, window, cx| this.goto_match(false, window, cx)),
                        cx,
                    ))
                    .child(self.panel_button(
                        self.i18n.t("next"),
                        Box::new(|this, window, cx| this.goto_match(true, window, cx)),
                        cx,
                    ))
                    .child(self.panel_button(
                        self.i18n.t("close"),
                        Box::new(|this, window, cx| this.close_find(window, cx)),
                        cx,
                    )),
            );

        if replace_open {
            column = column.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .pt_2()
                    .border_t_1()
                    .border_color(theme.hairline)
                    .child(div().flex_1().child(self.replace_input.clone()))
                    .child(self.panel_button(
                        self.i18n.t("replace"),
                        Box::new(|this, window, cx| this.replace_current(window, cx)),
                        cx,
                    ))
                    .child(self.panel_button(
                        self.i18n.t("replace_all"),
                        Box::new(|this, window, cx| this.replace_all_matches(window, cx)),
                        cx,
                    )),
            );
        }

        div()
            .absolute()
            .top(px(10.0))
            .right(px(18.0))
            .w(px(400.0))
            .p_3()
            .bg(theme.surface_card)
            .border_1()
            .border_color(theme.hairline)
            .rounded_lg()
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_e, _w, _cx| {})
            .child(column)
    }

    fn panel_button(
        &self,
        label: &'static str,
        action: DevNoteAction,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        div()
            .px_2()
            .py_1()
            .rounded_sm()
            .text_size(px(11.0))
            .bg(theme.surface_cream_strong)
            .text_color(theme.ink)
            .cursor_pointer()
            .hover(|style| style.bg(theme.surface_soft))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this: &mut DevNoteApp, _: &MouseUpEvent, window, cx| {
                    action(this, window, cx);
                }),
            )
            .child(label)
    }

    fn render_dialog(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .on_mouse_down(MouseButton::Left, |_e, _w, _cx| {})
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(400.0))
                    .p_5()
                    .bg(theme.canvas)
                    .border_1()
                    .border_color(theme.hairline)
                    .rounded_lg()
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(15.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(self.i18n.t("unsaved_title")),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.muted)
                            .child(self.i18n.t("unsaved_body")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .mt_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .text_color(theme.ink)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.surface_soft))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _: &MouseUpEvent, _w, cx| {
                                            this.dialog_cancel(cx);
                                        }),
                                    )
                                    .child(self.i18n.t("cancel")),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .text_color(theme.ink)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.surface_soft))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _: &MouseUpEvent, window, cx| {
                                            this.dialog_dont_save(window, cx);
                                        }),
                                    )
                                    .child(self.i18n.t("dont_save")),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(theme.primary)
                                    .text_color(theme.on_primary)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.primary_active))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _: &MouseUpEvent, window, cx| {
                                            this.dialog_save_all(window, cx);
                                        }),
                                    )
                                    .child(self.i18n.t("save_all")),
                            ),
                    ),
            )
    }

    fn render_toasts(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut stack = div()
            .absolute()
            .bottom(px(36.0))
            .right(px(16.0))
            .flex()
            .flex_col()
            .gap_2();
        for toast in &self.toasts {
            let accent = if toast.error {
                theme.error
            } else {
                theme.success
            };
            stack = stack.child(
                div()
                    .max_w(px(360.0))
                    .px_3()
                    .py_2()
                    .bg(theme.surface_dark)
                    .text_color(theme.on_dark)
                    .rounded_md()
                    .border_l_4()
                    .border_color(accent)
                    .text_size(px(12.0))
                    .child(toast.text.clone()),
            );
        }
        stack
    }
}
