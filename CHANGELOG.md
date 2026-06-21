# Changelog

## v0.2.0 — Production Grade Upgrade

### Breaking Changes

#### OS Integration & Architecture
- **Native menu system**: Replaced custom UI menus with OS-native menu bar using Tauri 2 `Menu` API
  - File: New Tab, Open..., Open Recent, Save, Save As..., Close Tab, Quit
  - Edit: Undo, Redo, Cut, Copy, Paste, Select All, Find..., Find & Replace..., Go to Line...
  - View: Word Wrap (check), Status Bar (check), Zoom In/Out/Reset
  - Window: Minimize, Maximize
  - Help: About devnotes
  - Keyboard accelerators use `CmdOrCtrl` for cross-platform compatibility
- **Native titlebar**: Changed from custom titlebar (`decorations: false`) to OS-native titlebar (`decorations: true`)
  - Title updates reflect: `[dirty dot] filename — devnotes`

#### Settings & Persistence
- **Settings now use `tauri-plugin-store`** instead of `localStorage`
  - Settings file: `$APPDATA/devnotes/.settings.dat`
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
- **Large file protection**: Warning dialog for files > 10MB before opening; hard 200MB cap
- **Binary file detection**: Refuses to open files containing NUL bytes
- **Atomic saves**: Write to uniquely-named temp file in same dir, then rename (prevents data loss on crash)
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
- **Language reconfigure via Compartment**: Setting a language no longer destroys+rebuilds the editor
- **Cursor update coalescing**: requestAnimationFrame batches cursor updates to one per frame

#### Status Bar
- **Encoding display**: Shows detected encoding (UTF-8, etc.)
- **Line ending display**: Shows CRLF/LF
- **Language picker dropdown**: Click language badge to change syntax highlighting

#### Accessibility
- **ARIA roles**: `alertdialog`, `tablist`, `tab`, `menu`, `menuitem`, `search`, `status`, `alert`
- **`prefers-reduced-motion`**: Disables all CSS transitions/animations when set
- **Tab key behavior**: Tab inserts spaces/tab in editor, doesn't steal focus

#### Error Handling & Resilience
- **Session recovery**: Auto-saves unsaved tab contents every 15 seconds to `$APPDATA/devnotes/recovery/`
  - Only writes when content has changed (hash check)
- **Recovery on restart**: Detects recovery files and offers to restore unsaved tabs
  - Cancel leaves recovery data intact for next launch
- **File logging**: Errors and info logged to `$APPDATA/devnotes/logs/devnotes.log`
  - Falls back to stderr if log file cannot be opened
- **Toast notifications**: Non-blocking error toasts (bottom-right, 4s duration)
- **No `unwrap()` in command paths**: All Rust commands return `Result<T, String>`
  - Logging init uses eprintln! fallback instead of `unwrap_or(/dev/null`) (which crashes on Windows)
- **Type-safe catch blocks**: All `catch (e: any)` replaced with `catch (e: unknown)` + errorMessage helper

#### Build Quality
- **Release optimizations**: `opt-level = "z"`, `lto = true`, `strip = true`, `codegen-units = 1`
- **CSP tightened**: No external font sources, blocks inline scripts
- **NSIS installer config**: Current user install, language selector, Start Menu folder
- **Bundle metadata**: copyright, category (DeveloperTool), short/long descriptions
- **Renderer capability hardening**: fs:* capabilities removed; file I/O is exclusively via Rust commands
  - Prevents any XSS or compromised dependency from writing to arbitrary paths
- **Path validation**: `read_file`, `read_file_with_encoding`, `add_recent_file`, and `check_file_size` require absolute, canonicalized, existing file paths
- **Dynamic language loading**: All CodeMirror language packs are loaded on demand
  - Initial bundle for the editor page: ~110 KB (down from ~1 MB)

### New Dependencies

#### Rust (Cargo.toml)
| Crate | Version | Purpose |
|---|---|---|
| `tauri-plugin-shell` | 2 | Reveal in File Explorer, open URLs |
| `tauri-plugin-store` | 2 | Persistent key-value settings storage |
| `encoding_rs` | 0.8 | Character encoding detection/decoding |
| `chardet` | 0.2 | Auto-detect text file encoding |
| `log` | 0.4 | Logging facade |
| `simplelog` | 0.12 | File-based log output |
| `tempfile` | 3 | Atomic save via NamedTempFile |

#### JavaScript (package.json)
| Package | Version | Purpose |
|---|---|---|
| `@tauri-apps/plugin-shell` | ^2 | Shell operations (open paths) |
| `@tauri-apps/plugin-store` | ^2 | Persistent settings store |
| `@tauri-apps/plugin-clipboard-manager` | ^2 | OS clipboard operations |
| `vitest` | ^2 | Unit test runner |
| `happy-dom` | (dev) | Browser-like env for tests |

### Migration Notes

1. **Settings**: `sabot-settings` localStorage key is migrated to the Tauri store automatically on first load. If the store is unavailable, falls back to localStorage with the new `devnotes-settings` key.
2. **Custom titlebar removed**: `TitleBar.svelte` is no longer used; app now uses native OS titlebar.
3. **Menu handles changed**: Menu events flow from Rust `lib.rs` via `app.emit()` to frontend event listeners.
4. **FilePayload updated**: Now includes `encoding` and `line_ending` fields (snake_case to match Rust).
5. **Recovery data**: Recovery files are cleared only after the user accepts Restore or Discard; Cancel keeps them for next launch.

### Test Suite

- Rust unit tests: `cargo test --lib` covers `detect_line_ending`, `normalize_line_endings`, `ensure_extension`, `validate_path`, `is_binary`.
- JS unit tests: `bun run test` covers `detectLanguage`, `ensureExtension`, `errorMessage`, `tabsStore` state machine, and the `EditorAction` discriminated union bus.
- CI: `.github/workflows/ci.yml` runs `bun run test`, `bun run check`, `cargo fmt --check`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo doc`.

