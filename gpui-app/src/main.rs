//! devnote - pure Rust (GPUI) port.
//!
//! The Tauri + Svelte app's Rust commands, stores and design tokens were
//! ported to a single GPUI application (see `docs/DESIGN.md` for the token
//! surface this app mirrors).

mod app;
mod document;
mod editor;
mod files;
mod find;
mod i18n;
mod settings;
mod text_input;
mod theme;

pub mod actions {
    use gpui::actions;

    actions!(
        devnote,
        [
            // File
            NewFile,
            OpenFile,
            SaveFile,
            SaveFileAs,
            CloseTab,
            NextTab,
            PrevTab,
            QuitApp,
            // Editing
            Backspace,
            DeleteForward,
            MoveLeft,
            MoveRight,
            MoveUp,
            MoveDown,
            SelectLeft,
            SelectRight,
            SelectUp,
            SelectDown,
            MoveHome,
            MoveEnd,
            MoveDocStart,
            MoveDocEnd,
            PageUp,
            PageDown,
            SelectAll,
            InsertNewline,
            InsertTab,
            Paste,
            Cut,
            Copy,
            Undo,
            Redo,
            // Find
            ToggleFind,
            ToggleReplace,
            FindNext,
            FindPrev,
            ReplaceCurrent,
            ReplaceAllMatches,
            SubmitInput,
            CloseOverlay,
            // View
            IncreaseFontSize,
            DecreaseFontSize,
            ResetFontSize,
            ToggleTheme,
        ]
    );
}

use gpui::{
    App, AppContext as _, Application, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions,
    WindowBounds, WindowOptions, px, size,
};

use app::DevNoteApp;
use settings::Settings;

fn main() {
    Application::new().run(|cx: &mut App| {
        let settings = Settings::load();

        cx.set_menus(build_menus());
        bind_keys(cx);

        let bounds = Bounds::centered(None, size(px(960.0), px(640.0)), cx);
        let window = match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(480.0), px(360.0))),
                titlebar: Some(TitlebarOptions {
                    title: Some("DevNote".into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| DevNoteApp::new(settings, cx));
                let focus_handle = view.read(cx).editor_focus_handle.clone();
                window.focus(&focus_handle);
                view
            },
        ) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("failed to open DevNote window: {error}");
                cx.quit();
                return;
            }
        };

        // Intercept window close so unsaved work can be confirmed.
        if let Ok(root) = window.update(cx, |_view, _window, cx| cx.entity()) {
            let weak = root.downgrade();
            let _ = window.update(cx, |_view, window, cx| {
                window.on_window_should_close(cx, move |_window, cx| {
                    weak.update(cx, |view, cx| view.request_close(cx))
                        .unwrap_or(true)
                });
            });
        }

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        cx.activate(true);
    });
}

fn build_menus() -> Vec<Menu> {
    use actions::*;
    vec![
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New", NewFile),
                MenuItem::action("Open...", OpenFile),
                MenuItem::separator(),
                MenuItem::action("Save", SaveFile),
                MenuItem::action("Save As...", SaveFileAs),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", Undo),
                MenuItem::action("Redo", Redo),
                MenuItem::separator(),
                MenuItem::action("Cut", Cut),
                MenuItem::action("Copy", Copy),
                MenuItem::action("Paste", Paste),
                MenuItem::action("Select All", SelectAll),
                MenuItem::separator(),
                MenuItem::action("Find...", ToggleFind),
                MenuItem::action("Find & Replace...", ToggleReplace),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Increase Font Size", IncreaseFontSize),
                MenuItem::action("Decrease Font Size", DecreaseFontSize),
                MenuItem::action("Reset Font Size", ResetFontSize),
                MenuItem::separator(),
                MenuItem::action("Toggle Theme", ToggleTheme),
                MenuItem::separator(),
                MenuItem::action("Next Tab", NextTab),
                MenuItem::action("Previous Tab", PrevTab),
            ],
        },
    ]
}

fn bind_keys(cx: &mut App) {
    use actions::*;

    let mut bindings = vec![
        // -- global (work regardless of focus) --------------------------
        KeyBinding::new("ctrl-n", NewFile, None),
        KeyBinding::new("cmd-n", NewFile, None),
        KeyBinding::new("ctrl-o", OpenFile, None),
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("ctrl-s", SaveFile, None),
        KeyBinding::new("cmd-s", SaveFile, None),
        KeyBinding::new("ctrl-shift-s", SaveFileAs, None),
        KeyBinding::new("cmd-shift-s", SaveFileAs, None),
        KeyBinding::new("ctrl-w", CloseTab, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("ctrl-q", QuitApp, None),
        KeyBinding::new("cmd-q", QuitApp, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PrevTab, None),
        KeyBinding::new("ctrl-f", ToggleFind, None),
        KeyBinding::new("cmd-f", ToggleFind, None),
        KeyBinding::new("ctrl-h", ToggleReplace, None),
        KeyBinding::new("cmd-alt-f", ToggleReplace, None),
        KeyBinding::new("f3", FindNext, None),
        KeyBinding::new("shift-f3", FindPrev, None),
        KeyBinding::new("escape", CloseOverlay, None),
        KeyBinding::new("ctrl-=", IncreaseFontSize, None),
        KeyBinding::new("ctrl-plus", IncreaseFontSize, None),
        KeyBinding::new("ctrl--", DecreaseFontSize, None),
        KeyBinding::new("ctrl-minus", DecreaseFontSize, None),
        KeyBinding::new("ctrl-0", ResetFontSize, None),
        KeyBinding::new("cmd-=", IncreaseFontSize, None),
        KeyBinding::new("cmd--", DecreaseFontSize, None),
        KeyBinding::new("cmd-0", ResetFontSize, None),
        // -- editor context ---------------------------------------------
        KeyBinding::new("backspace", Backspace, Some("editor")),
        KeyBinding::new("delete", DeleteForward, Some("editor")),
        KeyBinding::new("left", MoveLeft, Some("editor")),
        KeyBinding::new("right", MoveRight, Some("editor")),
        KeyBinding::new("up", MoveUp, Some("editor")),
        KeyBinding::new("down", MoveDown, Some("editor")),
        KeyBinding::new("shift-left", SelectLeft, Some("editor")),
        KeyBinding::new("shift-right", SelectRight, Some("editor")),
        KeyBinding::new("shift-up", SelectUp, Some("editor")),
        KeyBinding::new("shift-down", SelectDown, Some("editor")),
        KeyBinding::new("home", MoveHome, Some("editor")),
        KeyBinding::new("end", MoveEnd, Some("editor")),
        KeyBinding::new("ctrl-home", MoveDocStart, Some("editor")),
        KeyBinding::new("ctrl-end", MoveDocEnd, Some("editor")),
        KeyBinding::new("pageup", PageUp, Some("editor")),
        KeyBinding::new("pagedown", PageDown, Some("editor")),
        KeyBinding::new("ctrl-a", SelectAll, Some("editor")),
        KeyBinding::new("cmd-a", SelectAll, Some("editor")),
        KeyBinding::new("enter", InsertNewline, Some("editor")),
        KeyBinding::new("tab", InsertTab, Some("editor")),
        KeyBinding::new("ctrl-v", Paste, Some("editor")),
        KeyBinding::new("cmd-v", Paste, Some("editor")),
        KeyBinding::new("ctrl-c", Copy, Some("editor")),
        KeyBinding::new("cmd-c", Copy, Some("editor")),
        KeyBinding::new("ctrl-x", Cut, Some("editor")),
        KeyBinding::new("cmd-x", Cut, Some("editor")),
        KeyBinding::new("ctrl-z", Undo, Some("editor")),
        KeyBinding::new("cmd-z", Undo, Some("editor")),
        KeyBinding::new("ctrl-shift-z", Redo, Some("editor")),
        KeyBinding::new("cmd-shift-z", Redo, Some("editor")),
        KeyBinding::new("ctrl-y", Redo, Some("editor")),
        // -- find/replace inputs ----------------------------------------
        KeyBinding::new("backspace", Backspace, Some("TextInput")),
        KeyBinding::new("delete", DeleteForward, Some("TextInput")),
        KeyBinding::new("left", MoveLeft, Some("TextInput")),
        KeyBinding::new("right", MoveRight, Some("TextInput")),
        KeyBinding::new("shift-left", SelectLeft, Some("TextInput")),
        KeyBinding::new("shift-right", SelectRight, Some("TextInput")),
        KeyBinding::new("home", MoveHome, Some("TextInput")),
        KeyBinding::new("end", MoveEnd, Some("TextInput")),
        KeyBinding::new("ctrl-a", SelectAll, Some("TextInput")),
        KeyBinding::new("cmd-a", SelectAll, Some("TextInput")),
        KeyBinding::new("ctrl-v", Paste, Some("TextInput")),
        KeyBinding::new("cmd-v", Paste, Some("TextInput")),
        KeyBinding::new("ctrl-c", Copy, Some("TextInput")),
        KeyBinding::new("cmd-c", Copy, Some("TextInput")),
        KeyBinding::new("ctrl-x", Cut, Some("TextInput")),
        KeyBinding::new("cmd-x", Cut, Some("TextInput")),
        KeyBinding::new("enter", SubmitInput, Some("TextInput")),
    ];
    let _ = &mut bindings;

    cx.bind_keys(bindings);
}
