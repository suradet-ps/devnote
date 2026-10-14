# devnote-gpui

A **pure Rust** port of devnote, built with [GPUI](https://crates.io/crates/gpui)
(Zed's GPU-accelerated UI framework) instead of Tauri + Svelte + CodeMirror.

The goal is a notepad-class editor: open/save text files with the same
encoding and line-ending guarantees as the Tauri build, wrapped in the same
design system (`docs/DESIGN.md`). No webview, no JavaScript - the only
"web" thing left is the CSS-derived token values, translated to Rust in
`src/theme.rs`.

```
gpui-app/
├── src/
│   ├── main.rs        ← window, menus, key bindings, app bootstrap
│   ├── app.rs         ← root view: tabs, file ops, find panel, dialogs, toasts,
│   │                     status bar, EntityInputHandler (IME text input)
│   ├── editor.rs      ← custom GPUI Element that paints lines, line numbers,
│   │                     selection, search matches and the caret (virtualized)
│   ├── document.rs    ← buffer, caret/selection, undo history, line index
│   ├── files.rs       ← encoding detection, atomic save, line endings,
│   │                     recent-files/settings persistence
│   ├── find.rs        ← find/replace engine (literal + regex), ported from
│   │                     src/lib/editor/search.ts
│   ├── text_input.rs  ← single-line input for the find/replace fields
│   ├── settings.rs    ← persisted settings (theme, font size, locale)
│   ├── i18n.rs        ← en/th dictionaries, ported from src/lib/i18n
│   └── theme.rs       ← design tokens from docs/DESIGN.md / src/app.css
└── Cargo.toml
```

## Run

```bash
cargo run            # dev build
cargo test           # unit tests (document, files, find)
cargo clippy --all-targets -- -D warnings
```

The editor font is `JetBrains Mono`, UI font `Inter` (same as the Svelte
build; install them for best results, otherwise the system fallback is used).

## Parity with the Tauri build

| Feature | Status |
|---|---|
| Multi-tab editing, dirty indicator, window title | ✅ |
| Open / Save / Save As (native dialogs) | ✅ |
| Encoding detection (BOM + chardet) and re-encode on save | ✅ (UTF-8, UTF-16LE/BE, windows-1252) |
| Line-ending preservation (LF/CRLF/CR) | ✅ |
| Atomic save (temp file + rename) | ✅ |
| Binary / >200 MB / >50 MB read-only preview guards | ✅ |
| Find & Replace (case toggle, regex, next/prev, replace all) | ✅ |
| Recent files (persisted, welcome-screen list) | ✅ |
| Settings (theme light/dark/system, font size, locale) | ✅ |
| Keyboard shortcuts (see `AGENTS.md` §8) | ✅ (subset; see gaps) |
| IME / Thai input | ✅ (`EntityInputHandler`) |
| Unsaved-changes confirm dialog incl. window close interception | ✅ |
| Toasts for errors / low-confidence encoding | ✅ |
| Status bar (language, encoding, EOL, Ln/Col, words, chars) | ✅ |
| Native menu bar (File/Edit/View) | ✅ |
| **Not yet ported** | |
| Syntax highlighting (CodeMirror languages) | ❌ plain text only |
| Multi-cursor, go-to-line, go-to-symbol, edit-jump history | ❌ |
| Indent guides, visible whitespace, soft wrap | ❌ |
| Print / export to PDF | ❌ |
| Crash-recovery autosave | ❌ |
| Encoding picker for low-confidence reads | ❌ (toast only) |
| File watcher (external change detection) | ❌ |
| Linux CI build | ❌ untested (Windows + macOS in CI) |

## Attribution

`src/text_input.rs` is adapted from GPUI's `examples/input.rs` (the `gpui`
crate is Apache-2.0, matching the terms this project already relies on).
Everything else is a port of this repository's own Tauri code or written for
this crate.
