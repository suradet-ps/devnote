# STATUS

Verified current-state snapshot of the **devnote** codebase as of **v1.0.0**.
This document is the ground truth for AI agents and contributors. It is kept in
sync with the code; when a feature lands or changes, update both this file and
`AGENTS.md`. Do **not** trust `AGENTS.md` alone if it disagrees with STATUS - the
code wins, and STATUS reflects the code.

> Last reconciled: post-roadmap audit (see `ROADMAP.md`). The pre-v0.2.0 design
> described in the original `AGENTS.md` (custom titlebar, `tauri-plugin-fs` on the
> renderer, localStorage settings) was superseded and has been removed from the spec.

---

## 1. Identity

| Item | Value |
|---|---|
| App name | devnote |
| Binary name | `devnote` |
| Version | `1.0.0` (both `package.json` and `Cargo.toml`) |
| Tauri | 2.11 |
| Frontend | Svelte 5 (runes) + TypeScript 5 + Vite 8 |
| Editor | CodeMirror 6 (on-demand language packs) |
| Styling | CSS custom-property tokens (DESIGN.md), no inline hex |
| Rust | edition 2024, `bun` package manager |
| Titlebar | **native OS** (`decorations: true`) - no custom `TitleBar.svelte` |

---

## 2. File I/O (`src-tauri/src/commands/file.rs`)

| Capability | Status | Notes |
|---|---|---|
| Open / Read / Save / Save As | done | dialog-driven; Rust-only (no renderer `fs`) |
| Path validation | done | absolute + canonicalized; symlink-escape guarded |
| Encoding detection | done | `chardet` + `encoding_rs`; UTF-8 / UTF-16LE / UTF-16BE / Windows-1252 (UTF-16 needs BOM; NUL-byte binary check bypassed for UTF-16 BOM files) |
| Line-ending preservation | done | LF / CRLF / CR detected + preserved on save; cross-normalization on explicit LF/CRLF/CR target |
| Binary (NUL) detection | done | refuses to open files containing NUL bytes (except UTF-16 BOM files) |
| Size caps | done | 10 MB soft (confirm) / 200 MB hard (refuse) |
| Atomic save | done | `tempfile::NamedTempFile` + rename in same dir |
| Permission-denied fallback | done | offers "Save Copy" |
| `FilePayload` fields | done | `path, content, file_name, encoding, line_ending` |
| External-change detection | done | `notify` watcher: watch while tab open, 500 ms debounce, 1 s self-save suppression, Reload/Ignore dialog (Phase 5) |
| Large-file preview | done | > 50 MB opens read-only preview; 200 MB hard cap (Phase 5) |
| Chunked atomic save | done | streamed 1 MB chunks, no duplicate buffer (Phase 5) |
| Encoding override | done | low-confidence (`chardet` < 0.6) prompts EncodingPicker + `read_file_with_encoding` (Phase 5) |

---

## 3. Window & Menu (`lib.rs`, `commands/window.rs`)

| Capability | Status | Notes |
|---|---|---|
| Native titlebar | done | `decorations: true` |
| Dirty-dot title | done | `• filename - DevNote` via `set_window_title` |
| Native menu bar | done | File / Edit / View / Window / Help built in `build_menu` |
| Menu → event routing | done | emits Tauri events (`menu-open`, `menu-save`, …); no accelerators by design |
| Open Recent submenu | done | `recent-<path>` events; re-built dynamically |
| About dialog | partial | `menu-about` opens an in-app dialog (name/version); a full About window with licenses/offline disclaimer is still open (Roadmap Phase 9) |

---

## 4. Recent Files (`state/recent.rs`, `stores/recent.svelte.ts`)

| Capability | Status | Notes |
|---|---|---|
| Persisted list (max 10) | done | `{app_data_dir}/recent_files.json` |
| Add / Get / Remove | done | Rust commands + Svelte mirror store |
| Native "Open Recent" | **open** | submenu is a disabled `(No Recent Files)` placeholder; never rebuilt from `RecentFilesState` |
| UI entry point | **open** | the recent dialog is rendered but never opened; `recent-<path>` / `menu-open-recent` handlers exist but no menu items are created |

---

## 5. Session Recovery (`state/recovery.rs`, `commands/recovery.rs`)

| Capability | Status | Notes |
|---|---|---|
| Autosave (15 s, hash-coalesced) | done | `{app_data_dir}/recovery/`; hash on path+content (not `saved_at`) so unchanged tabs skip the write |
| Restore-on-launch prompt | done | Cancel keeps recovery data intact |
| Clear after Restore/Discard | done | `clear_recovery_data` |
| File logging | done | `{app_data_dir}/logs/devnote.log` |

---

## 6. Settings (`stores/settings.svelte.ts`)

| Capability | Status | Notes |
|---|---|---|
| `tauri-plugin-store` persistence | done | `.settings.dat`; localStorage fallback |
| Legacy migration | done | migrates `sabot-settings` / `devnote-settings` on first load (newest key wins) |
| Value sanitization | done | unknown keys / out-of-range values fall back to defaults (Phase 2) |
| `theme: system` | done | `prefers-color-scheme` media query |
| `showStatusBar` | done | toggle (v0.2.0+) |
| font size / tab size / insert spaces / wrap | done | all persisted |
| `locale` field | done | `system` \| `en` \| `th`, persisted; OS detection fallback `en` (Roadmap Phase 3) |
| i18n `t()` | done | `lib/i18n/i18n.svelte.ts`; typed en/th dictionaries, param interpolation, English fallback |

---

## 7. Editor (`lib/codemirror/*`, `components/Editor.svelte`)

| Capability | Status | Notes |
|---|---|---|
| 10+ language packs, on-demand | done | JavaScript is the only statically imported pack |
| Light + dark themes | done | `devnoteLightTheme` + `oneDark` |
| Go to Line (Ctrl+G) | done | clamps + centers viewport |
| Word-wrap / font-size / language reconfigure | done | via `Compartment`, no rebuild |
| Cursor update coalescing | done | `requestAnimationFrame` |
| Shebang-based detection | unwired | `detect-lang.ts` implements it and tests cover it, but no call site passes `content` |
| Multi-cursor | done | `Ctrl+D` add next occurrence, `Ctrl+Shift+L` select all occurrences (Phase 4) |
| Bracket/indent guides | done | CSS-only guides at tab stops; View → Indent Guides (Phase 4) |
| Visible whitespace | done | spaces `·` / tabs `→` in viewport; View → Visible Whitespace (Phase 4) |
| Edit-site history | done | `Ctrl+Alt+-`/`=` jump back/forward between edits (Phase 4) |
| Selection stats | done | status bar shows selected words/chars (Phase 4) |
| Go-to-Symbol | done | `Ctrl+Shift+P` picker for Rust/JS/TS/Python from parsed tree (Phase 4) |
| Print to PDF | done | `Ctrl+P` via OS print dialog (`print_current` command) (Phase 4) |

---

## 8. Tabs (`stores/tabs.svelte.ts`, `components/Tab*.svelte`)

| Capability | Status | Notes |
|---|---|---|
| Multi-tab, dirty check | done | `isDirty = content !== savedContent` |
| Middle-click close | done | |
| Drag-and-drop reorder | done | `reorder(from, to)` |
| Close last → new empty tab | done | app never shows 0 tabs |
| Context menu (Close/Others/All/Copy Path/Reveal) | done | `ContextMenu.svelte` |

---

## 9. Find / Replace (`components/FindReplace.svelte`)

| Capability | Status | Notes |
|---|---|---|
| Regex / case-sensitive | done | CM `SearchQuery` |
| Whole-word | **open** | not implemented in `FindReplace.svelte` |
| Match count | **open** | `countMatches` exists in `search.ts` but is test-only; no badge is rendered |
| Nav (Enter/Shift+Enter) | done | |
| Replace / Replace All | done | wired to real document transactions |
| Pure-logic extraction for tests | done | `lib/editor/search.ts` (findAll / countMatches / findNextFrom / replaceAll); replace + replace-all wired into Editor |

---

## 10. Accessibility

| Capability | Status | Notes |
|---|---|---|
| ARIA roles | done | `tablist`, `alertdialog`, `search`, `status`, `menu`, … |
| `prefers-reduced-motion` | done | disables transitions/animations |
| Tab inserts spaces (no focus steal) | done | |
| Keyboard-only full nav | done | tabs roving tabindex + arrows; context menu; pickers; dialogs trap focus; no mouse traps (Phase 3) |
| Focus ring | done | `:focus-visible` in `--accent-teal`; mouse focus quiet (Phase 3) |
| Screen-reader pass | done (auto) | `svelte-check` a11y lints clean; live region announces language/encoding/endings only; manual VO/NVDA session pending (`docs/a11y-notes.md`) |

---

## 11. OS Integration

| Capability | Status | Notes |
|---|---|---|
| macOS "Open With" / drag-to-icon | done | Tauri `RunEvent::Opened` + early buffer for pre-`setup` events (no custom Apple Event FFI) |
| File drag-and-drop | done | opens dropped files |
| 50+ file-association extensions | done | `tauri.conf.json` |
| Reveal in File Explorer | done | via `tauri-plugin-shell` |

---

## 12. Resilience & Security

| Capability | Status | Notes |
|---|---|---|
| Toasts (bottom-right, 4 s) | done | no `alert()`/`confirm()` |
| `Result<T, String>` everywhere | done | no `unwrap()` in command paths |
| CSP tightened | done | no inline scripts, no external fonts |
| Renderer `fs:*` removed | done | file I/O only via Rust commands |
| Path canonicalization | done | symlink-escape guard |
| `unsafe` audit | done | zero `unsafe` in `src-tauri/src` (macOS Apple Event FFI removed in favour of `RunEvent::Opened`); CI guard still pending (Roadmap Phase 8) |
| Release optimizations | done | `opt-level="z"`, `lto`, `strip`, `panic="abort"` |
| CI workflow | done | frontend check/test/build; Rust fmt/clippy/test/doc on 3-OS matrix; Tauri deb-build smoke; deny/audit gate; conventional-commits title check. Branch protection pending admin (Roadmap Phase 1) |
| Cross-platform release pipeline | done (partial) | tag-triggered Windows installer release exists; macOS/Linux + checksums + signing open (Roadmap Phase 9) |
| `cargo-deny` / `cargo-audit` | done | `src-tauri/deny.toml`; one explicit license exception: `chardet` (LGPL-3.0) |

---

## 13. Build & Test

| Capability | Status | Notes |
|---|---|---|
| `bun run check` (svelte-check) | done | |
| `bun run test` (vitest) | done | utils / stores / actions covered |
| `cargo test --lib` | done | 34 tests: line endings, encoding, path validation, binary/preview caps, recovery round-trip, watcher debounce/self-save suppression |
| Golden behavioral suite | done | `tests/golden_cases.json` + harness; runs with `bun run test` (Roadmap Phase 2) |
| Perf benchmarks / budgets | **open** | Roadmap Phase 7 |
| `cargo clippy -- -D warnings` | done | enforced in CI on 3-OS matrix |

---

## 14. Known Gaps (consolidated)

1. **Branch protection** - the only remaining Phase 1 item; requires repo-admin
   enablement on GitHub (required status checks, strict, on `main`).
2. **Recent-files UI** - no entry point: the native "Open Recent" submenu is a
   disabled placeholder and the recent dialog is never opened.
3. **Find/Replace** - no whole-word option and no visible match count.
4. **Shebang detection** - implemented and unit-tested, but not wired to call sites.
5. **Watcher unwatch** - the Ctrl+W / menu / tab-bar close paths bypass
   `unwatchPath`, so a closed tab's file may stay watched until Save-As or exit.
6. **About dialog** - shows name/version in an in-app dialog; a real About window
   with licenses/offline disclaimer is still open (Roadmap Phase 9).
7. **Virtualized large-file loading** - files > 50 MB open read-only (Phase 5);
   true streaming/virtualized editing is deferred.
8. **Manual SR session** - automated a11y checks are done (Phase 3); the
   VoiceOver/NVDA session checklist is in `docs/a11y-notes.md` and needs a
   human.
9. **`unsafe` CI guard** - the code is clean but no CI check prevents new `unsafe`
   blocks (Roadmap Phase 8).
10. **Perf budgets** not yet enforced, and the Phase 0 perf baseline is still not
    captured (Roadmap Phase 7).
