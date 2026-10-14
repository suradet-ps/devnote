//! Editor surface: a custom GPUI element that paints line numbers, text,
//! selection, search matches and the caret for the active document.
//!
//! Only lines intersecting the window viewport are shaped and painted, so the
//! cost per frame is O(visible lines) regardless of document size.

use gpui::{
    App, Bounds, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, SharedString, Style, TextRun,
    UnderlineStyle, Window, fill, point, px, relative, size,
};

use crate::app::DevNoteApp;

pub const CARET_WIDTH: f32 = 2.0;
pub const TEXT_PAD: f32 = 10.0;

pub fn line_height_for(font_size: f32) -> f32 {
    (font_size * 1.7).round()
}

pub fn gutter_width(show_numbers: bool, line_count: usize) -> f32 {
    if !show_numbers {
        return 0.0;
    }
    let digits = line_count.max(1).to_string().len();
    (digits as f32 * 8.5 + 24.0).max(44.0)
}

/// Metrics captured during paint so the app can map mouse positions and scroll
/// the caret into view without re-deriving layout.
#[derive(Clone, Copy)]
pub struct EditorMetrics {
    /// Bounds of the content area in window coordinates (shifted by scrolling).
    pub content_bounds: Bounds<Pixels>,
    pub line_height: f32,
    pub gutter_width: f32,
    pub line_count: usize,
}

impl EditorMetrics {
    /// Byte offset into the document for a window-space point.
    pub fn document_offset(
        &self,
        doc: &crate::document::Document,
        font_size: f32,
        text_system: &gpui::WindowTextSystem,
        position: gpui::Point<Pixels>,
    ) -> usize {
        if self.line_count == 0 {
            return 0;
        }
        let y = position.y - self.content_bounds.top();
        let raw_line = (y / px(self.line_height)).floor();
        let line = (raw_line.max(0.0) as usize).min(self.line_count - 1);
        let range = doc.line_range(line);

        let text_left = self.content_bounds.left() + px(self.gutter_width + TEXT_PAD);
        let x = position.x - text_left;
        if x <= px(0.0) {
            return range.start;
        }
        let text: SharedString = doc.line_text(line).to_string().into();
        let run = TextRun {
            len: text.len(),
            font: editor_font(),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = text_system.shape_line(text, px(font_size), &[run], None);
        match shaped.index_for_x(x) {
            Some(index) => range.start + index,
            None => range.end,
        }
    }
}

pub fn editor_font() -> gpui::Font {
    gpui::font("JetBrains Mono")
}

pub fn ui_font() -> gpui::Font {
    gpui::font("Inter")
}

struct PaintedLine {
    shaped: gpui::ShapedLine,
    number: Option<gpui::ShapedLine>,
    top: Pixels,
    is_caret_line: bool,
    selection: Option<(Pixels, Pixels)>,
    matches: Vec<(Pixels, Pixels)>,
    current_match: Option<(Pixels, Pixels)>,
    caret_x: Option<Pixels>,
}

pub struct PrepaintState {
    lines: Vec<PaintedLine>,
    metrics: EditorMetrics,
}

pub struct EditorElement {
    pub app: Entity<DevNoteApp>,
}

impl IntoElement for EditorElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let app = self.app.read(cx);
        let theme = app.theme;
        let font_size = app.font_size;
        let line_height = line_height_for(font_size);
        let show_numbers = app.show_line_numbers;
        let focused = app.editor_focus_handle.is_focused(window);

        let Some(doc) = app.active_doc() else {
            return PrepaintState {
                lines: Vec::new(),
                metrics: EditorMetrics {
                    content_bounds: bounds,
                    line_height,
                    gutter_width: 0.0,
                    line_count: 0,
                },
            };
        };

        let line_count = doc.line_count();
        let gutter = gutter_width(show_numbers, line_count);

        // Visible line window (with one line of overdraw each side). Use the
        // clip mask (scroll viewport), not the window bounds - the window
        // origin has nothing to do with where this element is scrolled to.
        let mask_bounds = window.content_mask().bounds;
        let top_visible = ((mask_bounds.top() - bounds.top()) / px(line_height)).floor();
        let bottom_visible = ((mask_bounds.bottom() - bounds.top()) / px(line_height)).ceil();
        let first = (top_visible.max(0.0) as usize).saturating_sub(1);
        let last = ((bottom_visible.max(0.0) as usize) + 1).min(line_count);

        let selection = doc.selection();
        let has_selection = doc.has_selection();
        let caret_line = doc.cursor_line();
        let caret = doc.caret;
        let marked_range = doc.marked_range.clone();
        let find = &app.find;
        let text_system = window.text_system().clone();
        let number_font = ui_font();
        let number_size = px((font_size - 1.0).max(9.0));

        let mut lines = Vec::with_capacity(last.saturating_sub(first));
        for line in first..last {
            let range = doc.line_range(line);
            let text: SharedString = doc.line_text(line).to_string().into();
            let line_len = range.len();

            // Text runs: base ink, with the IME marked range underlined.
            let base = TextRun {
                len: text.len(),
                font: editor_font(),
                color: theme.ink,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs: Vec<TextRun> = if let Some(marked) = marked_range
                .as_ref()
                .filter(|m| m.start < range.end && m.end > range.start && !text.is_empty())
            {
                let local_start = marked.start.clamp(range.start, range.end) - range.start;
                let local_end = marked.end.clamp(range.start, range.end) - range.start;
                vec![
                    TextRun {
                        len: local_start,
                        ..base.clone()
                    },
                    TextRun {
                        len: local_end - local_start,
                        underline: Some(UnderlineStyle {
                            color: Some(theme.ink),
                            thickness: px(1.0),
                            wavy: false,
                        }),
                        ..base.clone()
                    },
                    TextRun {
                        len: line_len - local_end,
                        ..base
                    },
                ]
                .into_iter()
                .filter(|run| run.len > 0)
                .collect()
            } else {
                vec![base]
            };

            let shaped = text_system.shape_line(text, px(font_size), &runs, None);

            let number = if show_numbers {
                Some(text_system.shape_line(
                    (line + 1).to_string().into(),
                    number_size,
                    &[TextRun {
                        len: (line + 1).to_string().len(),
                        font: number_font.clone(),
                        color: if line == caret_line {
                            theme.body
                        } else {
                            theme.muted_soft
                        },
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                ))
            } else {
                None
            };

            // Selection quad on this line.
            let selection_quad = if has_selection {
                let start = selection.start.max(range.start);
                let end = selection.end.min(range.end);
                if start < end
                    || (start == range.start
                        && end == range.end
                        && selection.end > range.end
                        && line + 1 < line_count)
                {
                    let local_start = start - range.start;
                    let local_end = (end - range.start).min(line_len).max(local_start);
                    let mut x2 = shaped.x_for_index(local_end);
                    // Show a small tail when the newline itself is selected.
                    if selection.end > range.end {
                        x2 += px(6.0);
                    }
                    Some((shaped.x_for_index(local_start), x2))
                } else {
                    None
                }
            } else {
                None
            };

            // Search match quads intersecting this line.
            let mut matches = Vec::new();
            let mut current_match = None;
            if !find.matches.is_empty() {
                let start_index = find.matches.partition_point(|m| m.end <= range.start);
                for (index, m) in find.matches.iter().enumerate().skip(start_index) {
                    if m.start >= range.end {
                        break;
                    }
                    let local_start = m.start.max(range.start) - range.start;
                    let local_end = m.end.min(range.end) - range.start;
                    if local_start >= local_end {
                        continue;
                    }
                    let quad = (
                        shaped.x_for_index(local_start),
                        shaped.x_for_index(local_end),
                    );
                    if Some(index) == find.current {
                        current_match = Some(quad);
                    } else {
                        matches.push(quad);
                    }
                }
            }

            let caret_x = if line == caret_line && focused {
                let local = caret.saturating_sub(range.start).min(line_len);
                Some(shaped.x_for_index(local))
            } else {
                None
            };

            let top = bounds.top() + px(line as f32 * line_height);
            lines.push(PaintedLine {
                shaped,
                number,
                top,
                is_caret_line: line == caret_line,
                selection: selection_quad,
                matches,
                current_match,
                caret_x,
            });
        }

        PrepaintState {
            lines,
            metrics: EditorMetrics {
                content_bounds: bounds,
                line_height,
                gutter_width: gutter,
                line_count,
            },
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Extract everything we need before mutating the app (the read borrow
        // of `cx` must end before `self.app.update`).
        let (theme, focus_handle, font_size) = {
            let app = self.app.read(cx);
            (app.theme, app.editor_focus_handle.clone(), app.font_size)
        };
        let line_height = prepaint.metrics.line_height;
        let _ = font_size;

        // Store metrics for mouse mapping / scroll-to-caret.
        self.app.update(cx, |app, _cx| {
            app.editor_metrics = Some(prepaint.metrics);
        });

        // Register the input handler on the caret line so IME candidate
        // windows anchor to the caret.
        let caret_line_bounds = prepaint
            .lines
            .iter()
            .find(|line| line.is_caret_line)
            .map(|line| Bounds {
                origin: point(bounds.left(), line.top),
                size: size(bounds.size.width, px(line_height)),
            })
            .unwrap_or(bounds);
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(caret_line_bounds, self.app.clone()),
            cx,
        );

        let gutter = prepaint.metrics.gutter_width;

        for line in &prepaint.lines {
            let line_bounds = Bounds {
                origin: point(bounds.left(), line.top),
                size: size(bounds.size.width, px(line_height)),
            };

            // Active-line wash.
            if line.is_caret_line {
                window.paint_quad(fill(line_bounds, theme.active_line));
            }

            // Gutter.
            if gutter > 0.0 {
                let gutter_bounds = Bounds {
                    origin: line_bounds.origin,
                    size: size(px(gutter), line_bounds.size.height),
                };
                window.paint_quad(fill(gutter_bounds, theme.surface_soft));
                // Right hairline.
                window.paint_quad(fill(
                    Bounds {
                        origin: point(gutter_bounds.right() - px(1.0), line_bounds.top()),
                        size: size(px(1.0), line_bounds.size.height),
                    },
                    theme.hairline,
                ));
                if let Some(number) = &line.number {
                    let number_width = number.x_for_index(number.text.len());
                    let x = gutter_bounds.right() - px(8.0) - number_width;
                    let _ = number.paint(point(x, line_bounds.top()), px(line_height), window, cx);
                }
            }

            let text_left = line_bounds.left() + px(gutter + TEXT_PAD);

            if let Some((x1, x2)) = line.selection {
                window.paint_quad(fill(
                    Bounds {
                        origin: point(text_left + x1, line_bounds.top()),
                        size: size(x2 - x1, line_bounds.size.height),
                    },
                    theme.selection,
                ));
            }

            for (x1, x2) in line.matches.iter().chain(line.current_match.iter()) {
                let color = if line.current_match == Some((*x1, *x2)) {
                    theme.match_current
                } else {
                    theme.match_highlight
                };
                window.paint_quad(fill(
                    Bounds {
                        origin: point(text_left + *x1, line_bounds.top()),
                        size: size(*x2 - *x1, line_bounds.size.height),
                    },
                    color,
                ));
            }

            let _ = line.shaped.paint(
                point(text_left, line_bounds.top()),
                px(line_height),
                window,
                cx,
            );

            if let Some(caret_x) = line.caret_x {
                window.paint_quad(fill(
                    Bounds {
                        origin: point(text_left + caret_x, line_bounds.top() + px(2.0)),
                        size: size(px(CARET_WIDTH), line_bounds.size.height - px(4.0)),
                    },
                    theme.primary,
                ));
            }
        }
    }
}

/// Renders the editor as a scrollable region, wiring the mouse handlers that
/// map clicks/drags to document offsets. Keyboard actions are registered on
/// the root view so menu items reach them regardless of focus.
pub fn render_editor(app: &mut DevNoteApp, cx: &mut gpui::Context<DevNoteApp>) -> impl IntoElement {
    use gpui::MouseButton;
    use gpui::prelude::*;

    let line_height = line_height_for(app.font_size);
    let line_count = app.active_doc().map(|d| d.line_count()).unwrap_or(1);
    let content_height = (line_count as f32 * line_height).max(line_height);
    let focus_handle = app.editor_focus_handle.clone();
    let scroll_handle = app.editor_scroll.clone();
    let app_entity = cx.entity();

    gpui::div()
        .id("editor-scroll")
        .relative()
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&scroll_handle)
        .track_focus(&focus_handle)
        .key_context("editor")
        .cursor(gpui::CursorStyle::IBeam)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(DevNoteApp::editor_mouse_down),
        )
        .on_mouse_move(cx.listener(DevNoteApp::editor_mouse_move))
        .on_mouse_up(MouseButton::Left, cx.listener(DevNoteApp::editor_mouse_up))
        .on_mouse_up_out(MouseButton::Left, cx.listener(DevNoteApp::editor_mouse_up))
        .child(
            gpui::div()
                .id("editor-content")
                .w_full()
                .h(px(content_height))
                .child(EditorElement { app: app_entity }),
        )
}
