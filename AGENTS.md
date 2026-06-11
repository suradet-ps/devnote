# text-rs
**Tauri 2.11 + Svelte 5 (TypeScript) Desktop Text Editor**

---

## 0. Project Identity

| Item | Value |
|---|---|
| App name | **text-rs** |
| Binary name | `text-rs` |
| Tauri version | 2.11 (latest stable) |
| Frontend | Svelte 5 + TypeScript + Vite |
| Styling | CSS custom properties (tokens from DESIGN.md) |
| Editor core | CodeMirror 6 |
| Rust edition | 2024 |

---

## 1. Repository Structure

```
text-rs/
├── AGENTS.md                  ← this file
├── DESIGN.md                  ← design tokens (Anthropic visual system)
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json       ← permission declarations
│   └── src/
│       ├── main.rs
│       ├── lib.rs             ← app builder, plugin registration
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── file.rs        ← open / save / save_as / recent files
│       │   └── window.rs      ← set_title, confirm_close
│       └── state/
│           ├── mod.rs
│           └── recent.rs      ← RecentFiles state (JSON persistence)
├── src/
│   ├── app.html
│   ├── lib/
│   │   ├── stores/
│   │   │   ├── tabs.ts        ← tab list, active tab
│   │   │   ├── recent.ts      ← recent files list
│   │   │   └── settings.ts    ← theme, font size, word wrap
│   │   ├── components/
│   │   │   ├── TitleBar.svelte
│   │   │   ├── TabBar.svelte
│   │   │   ├── Tab.svelte
│   │   │   ├── Editor.svelte  ← CodeMirror 6 wrapper
│   │   │   ├── StatusBar.svelte
│   │   │   ├── FindReplace.svelte
│   │   │   ├── ConfirmDialog.svelte
│   │   │   └── ContextMenu.svelte
│   │   ├── codemirror/
│   │   │   ├── setup.ts       ← editor state factory
│   │   │   ├── theme.ts       ← Sabot light + dark CM themes
│   │   │   ├── extensions.ts  ← language packs loader
│   │   │   └── keymap.ts      ← custom keybindings
│   │   └── utils/
│   │       ├── detect-lang.ts ← file extension → CodeMirror language
│   │       └── format-path.ts ← shorten paths for display
│   └── routes/
│       └── +page.svelte       ← root layout
├── static/
│   └── fonts/                 ← JetBrains Mono (editor), Inter (UI)
└── package.json
```

---

## 2. Design System (from DESIGN.md)

> AI agents **must** read `DESIGN.md` in full before touching any `.svelte`, `.css`, or `.ts` UI file.
> Every color, radius, and spacing value **must** use the CSS custom property token — **never inline hex**.

### 2.1 CSS Token Surface

Define all tokens in `src/app.html` `<style>` or a global `tokens.css`:

```css
:root {
  /* Surface */
  --canvas:              #faf9f5;
  --surface-soft:        #f5f0e8;
  --surface-card:        #efe9de;
  --surface-cream-strong:#e8e0d2;
  --surface-dark:        #181715;
  --surface-dark-elevated:#252320;
  --surface-dark-soft:   #1f1e1b;
  --hairline:            #e6dfd8;
  --hairline-soft:       #ebe6df;

  /* Brand */
  --primary:             #cc785c;
  --primary-active:      #a9583e;
  --primary-disabled:    #e6dfd8;
  --accent-teal:         #5db8a6;
  --accent-amber:        #e8a55a;

  /* Text */
  --ink:                 #141413;
  --body-strong:         #252523;
  --body:                #3d3d3a;
  --muted:               #6c6a64;
  --muted-soft:          #8e8b82;
  --on-primary:          #ffffff;
  --on-dark:             #faf9f5;
  --on-dark-soft:        #a09d96;

  /* Semantic */
  --success:             #5db872;
  --warning:             #d4a017;
  --error:               #c64545;

  /* Radius */
  --r-xs: 4px;
  --r-sm: 6px;
  --r-md: 8px;
  --r-lg: 12px;
  --r-xl: 16px;
  --r-pill: 9999px;

  /* Spacing */
  --sp-xxs:  4px;
  --sp-xs:   8px;
  --sp-sm:   12px;
  --sp-md:   16px;
  --sp-lg:   24px;
  --sp-xl:   32px;
  --sp-xxl:  48px;
}
```

### 2.2 Typography

- **UI chrome** (tab bar, toolbar, status bar, menus): `Inter`, 13–14px, weight 400–500
- **Editor area**: `JetBrains Mono`, configurable size (default 14px), weight 400
- Display labels (app title): may use `Cormorant Garamond` or `EB Garamond` as open-source substitute for Copernicus

### 2.3 Surface Modes

| Zone | Surface Token | Notes |
|---|---|---|
| Window chrome (titlebar, tabbar) | `--surface-dark` | Dark navy, `--on-dark` text |
| Editor background (light theme) | `--canvas` | Warm cream |
| Editor background (dark theme) | `--surface-dark` | Consistent with chrome |
| Status bar | `--surface-dark-elevated` | Slightly lighter dark |
| Find/Replace panel | `--surface-card` | Cream card, hairline border |
| Dialogs / modals | `--canvas` with `--hairline` border | |
| Active tab | `--canvas` (light) / `--surface-dark-elevated` (dark) | |
| Inactive tab | `--surface-dark-soft` | |

---

## 3. Tauri Configuration

### 3.1 `tauri.conf.json` (key fields)

```json
{
  "app": {
    "windows": [{
      "title": "text-rs",
      "width": 1200,
      "height": 800,
      "minWidth": 600,
      "minHeight": 400,
      "decorations": false,
      "transparent": false
    }]
  },
  "bundle": {
    "identifier": "th.hospital.sabot.text-rs",
    "productName": "text-rs"
  }
}
```

`decorations: false` → we draw our own custom title bar with window controls.

### 3.2 Required Plugins

```toml
# src-tauri/Cargo.toml [dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog  = "2"
tauri-plugin-fs      = "2"
tauri-plugin-shell   = "2"          # optional: open in file manager
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
tokio       = { version = "1", features = ["full"] }
```

### 3.3 Permissions (`capabilities/default.json`)

```json
{
  "permissions": [
    "fs:read-all",
    "fs:write-all",
    "dialog:open",
    "dialog:save",
    "core:window:allow-set-title",
    "core:window:allow-close",
    "core:window:allow-minimize",
    "core:window:allow-maximize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-start-dragging"
  ]
}
```

---

## 4. Rust Commands Reference

All commands live in `src-tauri/src/commands/`. Every command **must** return `Result<T, String>` — never panic, map all errors with `.map_err(|e| e.to_string())`.

### 4.1 `commands/file.rs`

```rust
// Signatures only — AI writes full implementation

/// Open file dialog → returns (path, content)
#[tauri::command]
pub async fn open_file(
    app: tauri::AppHandle,
) -> Result<Option<FilePayload>, String>

/// Read file from known path (e.g. from recent files)
#[tauri::command]
pub async fn read_file(path: String) -> Result<FilePayload, String>

/// Save content to existing path
#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<(), String>

/// Save As dialog → returns chosen path
#[tauri::command]
pub async fn save_file_as(
    app: tauri::AppHandle,
    content: String,
    suggested_name: Option<String>,
) -> Result<Option<String>, String>

/// Append path to recent-files list (max 10, persisted to AppData)
#[tauri::command]
pub async fn add_recent_file(
    state: tauri::State<'_, RecentFilesState>,
    path: String,
) -> Result<(), String>

/// Get current recent-files list
#[tauri::command]
pub async fn get_recent_files(
    state: tauri::State<'_, RecentFilesState>,
) -> Result<Vec<String>, String>

/// Remove a path from recent-files (e.g. file no longer exists)
#[tauri::command]
pub async fn remove_recent_file(
    state: tauri::State<'_, RecentFilesState>,
    path: String,
) -> Result<(), String>
```

**`FilePayload` struct:**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FilePayload {
    pub path: String,
    pub content: String,
    pub file_name: String,  // just the filename part
}
```

### 4.2 `commands/window.rs`

```rust
/// Update the native window title
#[tauri::command]
pub fn set_window_title(window: tauri::Window, title: String) -> Result<(), String>
```

### 4.3 `state/recent.rs`

```rust
pub struct RecentFilesState(pub Mutex<Vec<String>>);
// Persisted to: {app_data_dir}/recent_files.json
// Max entries: 10 (oldest removed when full)
```

---

## 5. Frontend State Architecture

### 5.1 Tab Model (`stores/tabs.ts`)

```typescript
export interface Tab {
  id: string           // uuid v4
  path: string | null  // null = untitled
  fileName: string     // display name, e.g. "untitled-1", "main.rs"
  content: string      // current text content
  savedContent: string // last saved content (for dirty check)
  language: string     // CodeMirror language key, e.g. "rust", "typescript"
  cursorLine: number
  cursorCol: number
  scrollTop: number
}

// Derived: isDirty = tab.content !== tab.savedContent
```

**Store methods:**

| Method | Description |
|---|---|
| `newTab()` | Create untitled tab, make active |
| `openTab(payload: FilePayload)` | If path already open → focus that tab; else create new |
| `closeTab(id)` | Check dirty → show ConfirmDialog if needed |
| `setActive(id)` | Switch active tab |
| `updateContent(id, content)` | Update content (marks dirty) |
| `markSaved(id, path)` | Update savedContent + path after save |
| `closeAll()` | Used on window close — check all dirty tabs |

### 5.2 Recent Files Store (`stores/recent.ts`)

```typescript
// Mirrors Rust state — calls get_recent_files on mount
// Calls add_recent_file after every open/save-as
export const recentFiles: Readable<string[]>
export function refreshRecentFiles(): Promise<void>
```

### 5.3 Settings Store (`stores/settings.ts`)

```typescript
export interface Settings {
  theme: 'light' | 'dark' | 'system'
  fontSize: number          // default 14
  fontFamily: string        // default 'JetBrains Mono'
  wordWrap: boolean         // default false
  showLineNumbers: boolean  // default true
  tabSize: number           // default 2
  insertSpaces: boolean     // default true
}
// Persisted to localStorage key: 'sabot-settings'
```

---

## 6. Component Specifications

### 6.1 `TitleBar.svelte`

- Background: `--surface-dark`
- Text: `--on-dark`, 13px Inter 500
- Shows: `[AppIcon] text-rs — {activeFileName}{dirtyDot}`
  - `dirtyDot` = `•` when active tab is dirty
- Right side: minimize / maximize / close buttons (custom SVG, color `--on-dark-soft`)
- Drag region: `data-tauri-drag-region` on the bar itself
- Close button: emits `window-close-request` → frontend checks dirty tabs before calling `appWindow.close()`

### 6.2 `TabBar.svelte`

- Background: `--surface-dark`
- Tab height: 36px
- Inactive tab: background `--surface-dark-soft`, text `--on-dark-soft`
- Active tab: background `--canvas` (light theme) or `--surface-dark-elevated` (dark theme), text `--on-dark` or `--ink`
- Tab shows: `{fileName}{dirtyDot}` + close `×` button on hover
- Overflow: horizontal scroll, no wrapping
- `+` button at end of tab list → calls `newTab()`
- Right-click on tab → `ContextMenu` with: Close, Close Others, Close All, Copy Path

### 6.3 `Editor.svelte`

Wraps a CodeMirror 6 `EditorView`. Props:

```typescript
export let tabId: string
export let content: string
export let language: string
export let settings: Settings
```

- On `content` prop change from outside (tab switch): update CM state without triggering the `onChange` loop
- On user edit: dispatch `content-change` event → tabs store `updateContent()`
- Saves cursor position and scroll on tab blur; restores on tab focus
- The editor fills `100%` of the remaining viewport height (flex-grow)
- CodeMirror theme: see Section 7

### 6.4 `StatusBar.svelte`

- Background: `--surface-dark-elevated`, text `--on-dark-soft`, 12px Inter
- Left: `{language}` badge (e.g. "Rust")
- Center: encoding (`UTF-8`), line endings (`LF` / `CRLF`)
- Right: `Ln {line}, Col {col}` | `{wordCount} words` | `{charCount} chars`
- Clicking language badge → opens language picker dropdown

### 6.5 `FindReplace.svelte`

Activated by `Ctrl+F` (find only) or `Ctrl+H` (find+replace).

- Renders as a panel **inside** the editor area, top-right, floating over content
- Background: `--surface-card`, border: `1px solid --hairline`, radius: `--r-lg`
- Contains:
  - Find input + `[↑] [↓]` match navigation buttons + `✕` close
  - Replace input (shown only in replace mode) + `[Replace] [Replace All]` buttons
  - Toggle options: `[Aa]` case sensitive, `[.*]` regex, `[ab]` whole word
- Match count badge: `3 of 12` in `--muted` color
- Uses CodeMirror's built-in `searchKeymap` + `SearchQuery` for the actual search logic
- Keyboard: `Enter`/`Shift+Enter` = next/prev match, `Escape` = close

### 6.6 `ConfirmDialog.svelte`

Modal dialog for "Unsaved changes" prompt.

- Background: `--canvas`, border: `1px solid --hairline`, radius: `--r-lg`
- Backdrop: `rgba(20,20,19,0.4)`
- Buttons: `[Save]` (primary coral), `[Don't Save]` (secondary), `[Cancel]`
- Uses Svelte's `createEventDispatcher` — resolves a Promise returned by `showConfirm()`

```typescript
// Usage in tabs store:
const result = await showConfirm({
  title: "Save changes?",
  message: `"${tab.fileName}" has unsaved changes.`
}) // → 'save' | 'discard' | 'cancel'
```

---

## 7. CodeMirror 6 Setup

### 7.1 Required packages

```json
"@codemirror/state": "^6",
"@codemirror/view": "^6",
"@codemirror/commands": "^6",
"@codemirror/search": "^6",
"@codemirror/language": "^6",
"@codemirror/lang-javascript": "^6",
"@codemirror/lang-typescript": "^6",
"@codemirror/lang-rust": "^6",
"@codemirror/lang-python": "^6",
"@codemirror/lang-html": "^6",
"@codemirror/lang-css": "^6",
"@codemirror/lang-markdown": "^6",
"@codemirror/lang-json": "^6",
"@codemirror/lang-sql": "^6",
"@codemirror/lang-xml": "^6",
"@codemirror/lang-vue": "^6",
"@codemirror/lang-cpp": "^6",
"@codemirror/lang-java": "^6",
"@codemirror/lang-php": "^6",
"@codemirror/theme-one-dark": "^6",
"codemirror": "^6"
```

### 7.2 Base extensions (`codemirror/setup.ts`)

```typescript
export function createEditorExtensions(settings: Settings, theme: 'light' | 'dark') {
  return [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightSpecialChars(),
    history(),
    foldGutter(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    indentOnInput(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    bracketMatching(),
    closeBrackets(),
    autocompletion(),
    rectangularSelection(),
    crosshairCursor(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...completionKeymap,
      ...lintKeymap,
      indentWithTab,
    ]),
    settings.wordWrap ? EditorView.lineWrapping : [],
    theme === 'dark' ? oneDark : textRsLightTheme,
    EditorView.theme({ '&': { fontSize: `${settings.fontSize}px` } }),
  ]
}
```

### 7.3 text-rs Light Theme (`codemirror/theme.ts`)

Map DESIGN.md tokens to CodeMirror theme spec:

```typescript
export const textRsLightTheme = EditorView.theme({
  '&': {
    backgroundColor: 'var(--canvas)',
    color: 'var(--ink)',
  },
  '.cm-content': { caretColor: 'var(--primary)' },
  '.cm-cursor': { borderLeftColor: 'var(--primary)' },
  '.cm-selectionBackground': { backgroundColor: 'rgba(204,120,92,0.15)' },
  '.cm-activeLine': { backgroundColor: 'rgba(204,120,92,0.06)' },
  '.cm-activeLineGutter': { backgroundColor: 'rgba(204,120,92,0.10)' },
  '.cm-gutters': {
    backgroundColor: 'var(--surface-soft)',
    color: 'var(--muted-soft)',
    borderRight: '1px solid var(--hairline)',
  },
  '.cm-lineNumbers .cm-gutterElement': { color: 'var(--muted-soft)' },
  '.cm-foldPlaceholder': { backgroundColor: 'var(--surface-card)' },
}, { dark: false })
```

### 7.4 Language Detection (`utils/detect-lang.ts`)

```typescript
const EXT_MAP: Record<string, string> = {
  rs: 'rust', ts: 'typescript', tsx: 'typescript',
  js: 'javascript', jsx: 'javascript',
  py: 'python', html: 'html', css: 'css',
  md: 'markdown', json: 'json', sql: 'sql',
  xml: 'xml', vue: 'vue', cpp: 'cpp', cc: 'cpp',
  java: 'java', php: 'php', toml: 'toml', yaml: 'yaml',
  // fallback → plain text
}

export function detectLanguage(path: string | null): string {
  if (!path) return 'text'
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return EXT_MAP[ext] ?? 'text'
}
```

---

## 8. Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+N` | New tab |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save (Save As if untitled) |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+W` | Close current tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+1`–`Ctrl+9` | Jump to tab by index |
| `Ctrl+F` | Find |
| `Ctrl+H` | Find & Replace |
| `F3` | Find next |
| `Shift+F3` | Find previous |
| `Ctrl+G` | Go to line |
| `Ctrl+Z` | Undo (CodeMirror history) |
| `Ctrl+Shift+Z` | Redo |
| `Ctrl++` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset font size to default |
| `Alt+Z` | Toggle word wrap |

Register global shortcuts (tab switching, new/open) via Svelte `window` event listener in `+page.svelte`. Editor-internal shortcuts (undo, find, indent) are handled by CodeMirror keymaps.

---

## 9. Window Close Flow

```
User clicks ✕ (or Alt+F4)
  │
  ▼
frontend intercepts CloseRequested event (Tauri)
  │
  ▼
Are there any dirty tabs?
  │ No → appWindow.close()
  │ Yes
  ▼
ConfirmDialog: "You have {n} unsaved file(s). Save before closing?"
  ├── [Save All] → save all dirty tabs → close
  ├── [Don't Save] → close without saving
  └── [Cancel] → abort, window stays open
```

Implementation: listen to `tauri://close-requested` event with `event.preventDefault()` to intercept, then handle in Svelte.

---

## 10. Recent Files

- Stored on Rust side: `{app_data_dir}/recent_files.json` — list of absolute paths (max 10)
- Menu: shown in a dropdown from a "Recent" button in the toolbar, or accessible via `File` menu if implemented
- On open from recent: call `read_file(path)` → if file no longer exists, call `remove_recent_file(path)` and show a toast "File not found"
- On app start: call `get_recent_files()` → populate `recentFiles` store

---

## 11. Dirty Flag Rules

- `isDirty(tab)` = `tab.content !== tab.savedContent`
- Dirty indicator: `•` appended to tab name, e.g. `main.rs •`
- Title bar shows `•` before app name when active tab is dirty: `• text-rs — main.rs`
- Dirty check triggers on: tab close, window close, open new file in same tab (not applicable here — we always open in new tab)
- After successful save: `markSaved(id, path)` — sets `savedContent = content`

---

## 12. File Operations Detail

### Open File
1. Call `invoke('open_file')` → dialog appears
2. If `null` returned → user cancelled, do nothing
3. Call `openTab(payload)` → if path already open, focus that tab
4. Call `add_recent_file(path)`

### Save (Ctrl+S)
1. If `tab.path === null` → redirect to Save As flow
2. Call `invoke('save_file', { path, content })`
3. On success: `markSaved(id, path)`
4. Update window title (remove dirty indicator)

### Save As (Ctrl+Shift+S)
1. Call `invoke('save_file_as', { content, suggestedName: tab.fileName })`
2. If `null` returned → user cancelled
3. On success: `markSaved(id, newPath)`, `add_recent_file(newPath)`, update tab name

---

## 13. Error Handling & Toasts

- All `invoke()` calls wrapped in `try/catch`
- On error: show a toast notification (non-blocking, 4 seconds)
  - Toast style: background `--surface-dark`, text `--on-dark`, border-left `4px solid --error`, radius `--r-md`
  - Toast position: bottom-right
- Never use `alert()` or `confirm()` — all dialogs are custom Svelte components

---

## 14. Coding Standards

### Rust
- All commands `async` (use `tokio::fs` for I/O, not `std::fs`)
- No `unwrap()` in production paths — use `?` + `map_err`
- All Tauri state wrapped in `Mutex<T>` or `RwLock<T>`
- `#[derive(Debug, serde::Serialize, serde::Deserialize)]` on all shared structs
- Clippy clean: `cargo clippy -- -D warnings`

### TypeScript / Svelte
- Svelte 5 runes syntax (`$state`, `$derived`, `$effect`) — **not** legacy `$:` reactive syntax
- All `invoke()` calls typed with explicit return type generics
- No `any` — use proper types for all Tauri payloads
- Stores use `$state` / `$derived` rune pattern exclusively — **never** legacy `writable/readable` from `svelte/store`
- Component props typed with TypeScript interfaces

### General
- No inline styles — use CSS custom property tokens only
- All user-facing strings in Thai or English (app is bilingual-ready — use a `t()` helper stub)
- All file paths handled as strings (no `URL` objects — Tauri paths are OS strings)

---

## 15. Build & Dev Commands

```bash
# Install dependencies
npm install

# Dev server (hot reload)
npm run tauri dev

# Type check
npm run check

# Production build
npm run tauri build

# Rust lint
cd src-tauri && cargo clippy

# Rust test
cd src-tauri && cargo test
```

---

## 16. Out of Scope (Future Phases)

- Spell check / grammar
- Plugin / extension system
- Git integration
- Remote file access (SFTP, S3)
- Collaborative editing
- Terminal pane
- File tree / project explorer (Phase 2 candidate)
- Minimap
- Multiple cursors beyond CM default

---

## 17. AI Agent Instructions

1. **Always read `DESIGN.md` first** before writing any UI code.
2. **Never use inline hex values** — always `var(--token-name)`.
3. **Never use `any`** in TypeScript.
4. **Implement features in order**: File I/O → Tabs → Find/Replace → Recent Files → Syntax Highlight. Each phase must be working before the next begins.
5. **When adding a Tauri command**, update `commands/mod.rs` AND register in `lib.rs` `.invoke_handler()`.
6. **When adding a permission**, update `capabilities/default.json`.
7. **Dirty flag must be checked** before any destructive action (close tab, close window, open in same tab).
8. **CodeMirror state lives inside `Editor.svelte`** — do not store CM `EditorView` in a Svelte store (it is not serializable). Use events to communicate content changes out.
9. **Tab IDs are `crypto.randomUUID()`** — never use array index as ID.
10. **All dialogs are Svelte components** — never use browser `alert/confirm/prompt`.
