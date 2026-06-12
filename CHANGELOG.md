# Changelog

## v0.2.0 — Production Grade Upgrade

### Breaking Changes

#### OS Integration & Architecture
- **Native menu system**: Replaced custom UI menus with OS-native menu bar using Tauri 2 `Menu` API
  - File: New Tab, Open..., Open Recent, Save, Save As..., Close Tab, Quit
  - Edit: Undo, Redo, Cut, Copy, Paste, Select All, Find..., Find & Replace..., Go to Line...
  - View: Word Wrap (check), Status Bar (check), Zoom In/Out/Reset
  - Window: Minimize, Maximize
  - Help: About text-rs
  - Keyboard accelerators use `CmdOrCtrl` for cross-platform compatibility
- **Native titlebar**: Changed from custom titlebar (`decorations: false`) to OS-native titlebar (`decorations: true`)
  - Title updates reflect: `[dirty dot] filename — text-rs`
- **Window state persistence**: Added `tauri-plugin-window-state` for persistent window size/position

#### Settings & Persistence
- **Settings now use `tauri-plugin-store`** instead of `localStorage`
  - Settings file: `$APPDATA/text-rs/.settings.dat`
  - Falls back to localStorage if store plugin unavailable
- **New settings**: `showStatusBar` (toggle), `theme` defaults to `"system"`
- **System theme detection**: `theme: "system"` listens to `prefers-color-scheme` media query
- **Dark mode CSS**: Custom property overrides via `[data-theme="dark"]`

#### File I/O
- **File associations expanded**: 50+ file extensions registered (.rs, .ts, .py, .md, .json, .yaml, .xml, .go, .rb, .sh, .c, .cpp, .java, .php, .vue, .svelte, .log, .csv, .ini, .cfg, .conf, .env, .gitignore, .editorconfig, .dockerfile, etc.)
- **File drag-and-drop**: Drag files onto the editor window to open them
- **Open-with deep links**: OS passes file path on launch (double-click .txt in Finder/Explorer)
- **Encoding detection**: Uses `chardet` crate to detect UTF-8, UTF-16 LE/BE, Windows-1252
- **Line ending preservation**: Detects and preserves CRLF/LF/CR on save
- **Large file protection**: Warning dialog for files > 10MB before opening
- **Atomic saves**: Write to temp file then rename (prevents data loss on crash)
- **Permission error handling**: Catch permission denied errors, offer "Save Copy" alternative

#### Tab System
- **Close last tab → creates new empty tab**: App never shows 0 tabs
- **Middle-click tab close**
- **Drag-and-drop tab reordering**
- **Enhanced context menu**: Close, Close Others, Close All, Copy Path, Reveal in File Explorer

#### Editor
- **Go to Line dialog** (Ctrl+G): Clamped to valid range, centers viewport on target line
- **Shebang-based language detection**: Files without extension check `#!/usr/bin/env python3` etc.
- **Word wrap toggle**: Uses CodeMirror `Compartment` for instant reconfiguration without recreating editor
- **Dynamic font size**: Reconfigured via Compartment, no editor rebuild needed

#### Status Bar
- **Selection info**: Shows selected chars/lines count
- **Encoding display**: Shows detected encoding (UTF-8, etc.)
- **Line ending display**: Shows CRLF/LF
- **Language picker dropdown**: Click language badge to change syntax highlighting

#### Accessibility
- **ARIA roles**: `alertdialog`, `tablist`, `tab`, `menu`, `menuitem`, `search`, `status`, `alert`
- **`prefers-reduced-motion`**: Disables all CSS transitions/animations when set
- **Focus management**: Return focus to editor after closing panels/dialogs
- **Tab key behavior**: Tab inserts spaces/tab in editor, doesn't steal focus

#### Error Handling & Resilience
- **Session recovery**: Auto-saves unsaved tab contents every 30 seconds to `$APPDATA/text-rs/recovery/`
- **Recovery on restart**: Detects recovery files and offers to restore unsaved tabs
- **File logging**: Errors and info logged to `$APPDATA/text-rs/logs/text-rs.log`
- **Toast notifications**: Non-blocking error toasts (bottom-right, 4s duration)
- **No `unwrap()`**: All Rust commands return `Result<T, String>`

#### Build Quality
- **Release optimizations**: `opt-level = "z"`, `lto = true`, `strip = true`, `codegen-units = 1`
- **CSP tightened**: No external font sources, blocks inline scripts
- **NSIS installer config**: Current user install, language selector, Start Menu folder
- **Bundle metadata**: copyright, category (DeveloperTool), short/long descriptions

### New Dependencies

#### Rust (Cargo.toml)
| Crate | Version | Purpose |
|---|---|---|
| `tauri-plugin-shell` | 2 | Reveal in File Explorer, open URLs |
| `tauri-plugin-store` | 2 | Persistent key-value settings storage |
| `tauri-plugin-window-state` | 2 | Persist window size/position |
| `encoding_rs` | 0.8 | Character encoding detection/decoding |
| `chardet` | 0.2 | Auto-detect text file encoding |
| `notify` | 7 | File system watcher |
| `log` | 0.4 | Logging facade |
| `simplelog` | 0.12 | File-based log output |

#### JavaScript (package.json)
| Package | Version | Purpose |
|---|---|---|
| `@tauri-apps/plugin-shell` | ^2 | Shell operations (open paths) |
| `@tauri-apps/plugin-store` | ^2 | Persistent settings store |
| `@tauri-apps/plugin-clipboard-manager` | ^2 | OS clipboard operations |

### Migration Notes

1. **Settings reset**: Old localStorage key `sabot-settings` is migrated to Tauri store automatically on first load.
2. **Custom titlebar removed**: `TitleBar.svelte` is no longer used; app now uses native OS titlebar.
3. **Menu handles changed**: Menu events now flow from Rust `lib.rs` via `app.emit()` to frontend event listeners.
4. **FilePayload updated**: Now includes `encoding` and `line_ending` fields.
5. **Recovery data**: Recovery files are cleaned up on clean exit; only persist after crashes.
