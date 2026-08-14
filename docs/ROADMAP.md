# DevNote Roadmap

This roadmap tracks the path from the current production-grade scaffold (v1.0.0)
to a durable, maintainable first long-term-support release, and beyond. It follows
the architecture, design system, and engineering standards set out in
[AGENTS.md](../AGENTS.md), [DESIGN.md](DESIGN.md), [CONTRIBUTING.md](CONTRIBUTING.md),
and the v0.2.0 record in [CHANGELOG.md](CHANGELOG.md).

The example this roadmap was modelled on (the MenSung medical-editor roadmap) is a
*research-grade safety-critical* project: it gates every release on a golden test
suite and a zero-false-negative policy. DevNote is **not** safety-critical, but the
same engineering discipline applies — phased, verifiable, with each phase done
before the next begins, and nothing shipped until its acceptance checks pass.

> **Scope discipline.** DevNote is a *text editor*, not an IDE. The end-state target
> is: a fast, offline-first, accessible, multi-tab plain-text/code editor with
> trustworthy file I/O, session recovery, and a warm, calm visual identity. Features
> that turn it into a project explorer, terminal, or Git client are explicitly
> out of scope (see "Out of Scope" at the end). Where a capability is valuable but
> risky to scope-creep, it is listed as Future/Ecosystem only.

## Current State (baseline)

Verified directly against the repository, not assumed:

- **App identity**: Tauri 2.11 + Svelte 5 (runes) + CodeMirror 6, Rust 2024 edition,
  TypeScript 5, package manager `bun`. Version `1.0.0` in both `package.json` and
  `Cargo.toml`.
- **File I/O** (`src-tauri/src/commands/file.rs`): open / read / save / save-as,
  absolute-path validation + canonicalization (symlink-escape guard), encoding
  detection (`chardet` + `encoding_rs`, UTF-8/UTF-16 LE/BE/Windows-1252), CRLF/LF/CR
  line-ending preservation, binary (NUL) detection, 10 MB soft / 200 MB hard size
  cap, atomic saves via `tempfile::NamedTempFile` + rename, permission-denied →
  "Save Copy" fallback.
- **Window** (`commands/window.rs`): `set_window_title`, native titlebar
  (`decorations: true`), dirty-dot title `• filename — DevNote`.
- **Recent files** (`state/recent.rs`, `commands/file.rs`): persisted list (max 10),
  context-menu + OS native "Open Recent" submenu wired in `lib.rs`.
- **Session recovery** (`state/recovery.rs`, `commands/recovery.rs`): 15 s
  content-hash-coalesced autosave to `$APPDATA/devnote/recovery/`, restore-on-launch
  prompt, Cancel keeps recovery data intact.
- **Settings** (`stores/settings.svelte.ts`): `tauri-plugin-store`
  (`$APPDATA/devnote/.settings.dat`) with localStorage fallback; `theme: "system"`
  via `prefers-color-scheme`; `showStatusBar`, font size, tab size, insert spaces,
  word wrap.
- **Editor** (`lib/codemirror/*`, `components/Editor.svelte`): 10+ language packs
  loaded on demand (~110 KB initial editor bundle), dark + light themes, go-to-line,
  word-wrap / font-size / language reconfigure via `Compartment` (no editor rebuild),
  cursor coalescing via `requestAnimationFrame`.
- **Tabs** (`stores/tabs.svelte.ts`, `components/Tab*.svelte`): multi-tab, dirty
  check, middle-click close, drag-reorder, "close last → new empty tab", context menu
  (Close / Close Others / Close All / Copy Path / Reveal in File Explorer).
- **Find/Replace** (`components/FindReplace.svelte`): regex, case-sensitive,
  whole-word, match count, Enter/Shift+Enter navigation.
- **Accessibility**: ARIA roles (`tablist`, `alertdialog`, `search`, `status`,
  `menu`…), `prefers-reduced-motion`, Tab-inserts-spaces behavior.
- **OS integration**: native menu bar (File/Edit/View/Window/Help) emitting Tauri
  events; macOS "Open With" + drag-to-icon via Apple Events (`macos_events.rs`);
  file drag-and-drop; 50+ file-association extensions; shebang-based language
  detection.
- **Resilience**: file logging to `$APPDATA/devnote/logs/devnote.log`, bottom-right
  non-blocking toasts, `Result<T, String>` everywhere, no `unwrap()` in command paths,
  hardened CSP (no inline scripts, no external fonts), `fs:*` renderer capability
  removed (file I/O only via Rust).
- **Build**: `opt-level="z"`, `lto=true`, `strip=true`, `codegen-units=1`,
  `panic="abort"`; NSIS installer config; macOS ad-hoc + sandboxed entitlement paths.

**Gaps against the stated standards (found while reading the repo):**

- `AGENTS.md` documents an architecture (custom titlebar, `fs:*` dialog plugin,
  `stores/recent.ts` readable, etc.) that **no longer matches the code** — it was
  superseded by v0.2.0. AGENTS.md must be reconciled with reality (Phase 0 task).
- **CI is complete** — `.github/workflows/ci.yml` runs frontend check/test/build,
  the Rust fmt/clippy/test/doc gate on a 3-OS matrix, a Tauri deb-build smoke on
  Linux, a `cargo-deny`/`cargo-audit` dependency gate, and a conventional-commits
  PR title check (Phase 1; the one remaining item, branch protection, is a
  repo-admin action). Note: an earlier draft of this roadmap claimed "no CI
  exists" — that was inaccurate; the workflow predates it.
- No automated **cross-platform release pipeline** (checksums, signed artifacts) —
  a tag-triggered Windows installer workflow (`release-windows.yml`) exists, but
  macOS/Linux artifacts, checksums, and signing are not wired (see Phase 9).
- No **golden/regression test suite** gating editor behavior (only unit tests for
  utils/stores/actions exist today).
- No **benchmarks** (startup, open, save-latency) even though perf is a stated goal.

---

## Phase 0: Foundation Reconciliation (done)

- [x] Tauri workspace boots, editor opens/saves, recovery + recent + settings persist
- [x] Tests exist: `bun run test` (vitest) + `cargo test --lib` (detect_line_ending,
      normalize_line_endings, ensure_extension, validate_path, is_binary)
- [x] **Reconcile `AGENTS.md` with the actual v1.0.0 architecture** — the doc still
      described the pre-v0.2.0 custom-titlebar/`fs:*` design. Updated in
      `phase-0-reconcile` (commit `03d6c5f`) to reflect native titlebar,
      `tauri-plugin-store`, recovery state, clipboard plugin, macOS Apple Events,
      no renderer `fs:*`, on-demand language packs, Svelte 5 runes stores, and the
      real `FilePayload` / command / permission layout. A spec that lies about the
      code is worse than no spec.
- [x] Add a top-level `STATUS.md` capturing the real, verified current-state table
      (per-subsystem done/open matrix + consolidated known gaps) so future phases
      have a shared ground truth. AGENTS.md now defers to STATUS.md on any conflict.
- [x] **Verification gate (all green):**
  - `bun run check` (svelte-check): **0 errors, 0 warnings** — verified.
  - `cargo clippy -- -D warnings`: **0 warnings** — verified.
  - `cargo test` (workspace lib): **6/6 passed** (`detect_line_ending`,
    `normalize_line_endings`, `ensure_extension`, `validate_path` ×2, `is_binary`).
  - **Low-memory build fix:** the dev box (≈4 GB RAM) initially crashed compiling
    the `windows` crate with `STATUS_STACK_BUFFER_OVERRUN` (0xc0000409) / "Insufficient
    quota" (os error 1453). Root cause was per-process heap exhaustion from full
    debug info, **not** a toolchain defect. Fixed by committing
    `src-tauri/.cargo/config.toml` (`.cargo/config.toml`) which sets `build.jobs = 1`
    and `profile.dev/test.debug = 0` so the workspace compiles on constrained
    machines. Release/CI builds keep full debug info and are unaffected.
  - Baseline perf numbers (bundle size, cold start, open/save latency): **not
    captured** yet — defer to Phase 7 budgets once CI exists. This is the only
    explicit carry-over; it is a measurement task, not a correctness gap.

**Acceptance:** AGENTS.md matches the code; STATUS.md exists; frontend `check`,
Rust `clippy`, and `cargo test` are all green on a local clone (with the committed
low-memory `.cargo/config.toml`). Perf baseline remains open under Phase 7.

---

## Phase 1: Continuous Integration (the missing promise)

This phase exists because CHANGELOG claims a CI story. The workflow already exists
in the repo (`.github/workflows/ci.yml`); completed in the `phase-1-ci` PR — every
code-level gate below now runs on push/PR to `main`.

- [x] `.github/workflows/ci.yml`, runs on push/PR to `main`:
  - [x] **Frontend**: `bun install --frozen-lockfile`, `bun run check`
    (svelte-check), `bun run test` (vitest), `bun run build` (vite build).
  - [x] **Rust**: `cargo fmt --check`, `cargo check --locked`,
    `cargo clippy -- -D warnings`, `cargo test`, `cargo doc --no-deps`.
  - [x] Matrix: `ubuntu-latest`, `macos-latest`, `windows-latest` for the Rust
    checks; frontend on `ubuntu-latest`.
  - [x] Caching: `actions/cache` for `~/.cargo/{registry,git}` + `target/`
    and the bun install cache.
- [x] **Tauri build smoke**: `bun run tauri build --bundles deb` on a Linux runner
  (catches broken `tauri.conf.json`, missing icons, capability drift, bundler
  config). `--bundles deb` only: deb's tooling is guaranteed on GH runners;
  rpm needs `rpmbuild` and AppImage downloads `appimagetool`. Does **not** publish.
- [x] A `conventional-commits` PR title check (already the de-facto style in
  CHANGELOG) so the changelog/release notes can be automated later.
- [x] `cargo-deny` + `cargo-audit` gate (license + advisory), driven by
  `src-tauri/deny.toml`. Includes one documented license exception:
  `chardet` (LGPL-3.0) — see the deny.toml note and Phase 8.
- [ ] Branch protection on `main`: required status checks (strict), no force-push,
  no deletion — **repo-admin action on GitHub, not code**; enable after this PR
  merges.

**Acceptance:** A PR that breaks `check`, `clippy -D warnings`, any test,
`bun run build`, or the Tauri build is red and cannot merge. Deny/audit step is
green. (Branch protection pending manual admin enablement.)

---

## Phase 2: Golden Regression Suite (editor behavior, not just utils)

The example roadmap gates releases on a golden test suite. DevNote's equivalent is a
behavioral regression suite that captures the invariants users actually notice.
Completed in the `phase-2-golden` PR — **this phase surfaced and fixed three
pre-existing bugs** (see the notes below).

- [x] **Tabs state machine** (`stores/tabs.svelte.ts`): extended coverage for
  dirty-on-edit, `markSaved` clears dirty **and** updates fileName/language,
  close-dirty-tab rejected (returns false) + `forceCloseTab` path, reorder
  bounds, "close last → new empty tab" (untitled counter stays unique across
  closes), open-already-open path focuses existing tab, active-tab switch when
  closing middle/last/inactive tabs.
- [x] **Recovery round-trip**: `commands/recovery.rs` refactored into testable
  helpers (`write_recovery_file` / `read_recovery_file` / `clear_recovery_file`)
  driven by temp-dir tests: save → restore yields identical content + path,
  Discard (clear) removes the file, Cancel (no clear) preserves it, empty list
  and malformed JSON resolve to `None` (never block startup).
  - **Bug found & fixed**: autosave hash-coalescing never coalesced — the hash
    included `saved_at`, so every 15 s tick rewrote the file. Now hashed on
    path+content only (`utils/recovery.ts`).
- [x] **File I/O invariants** (`commands/file.rs`): encoding detection matrix
  (UTF-8 / UTF-8+BOM / UTF-16LE+BOM / UTF-16BE+BOM / Windows-1252), line-ending
  round-trip (LF/CRLF/CR + cross-normalization), binary (NUL) rejection,
  symlink canonicalization (unix), hard-cap rejection > 200 MB (sparse file),
  soft-cap opens > 10 MB, atomic save leaves no `.tmp` behind, failed save
  (readonly target, Windows) leaves the original untruncated.
  - **Bugs found & fixed**: (1) UTF-16 files could never open — the NUL-byte
    binary check rejected them before encoding detection; (2) saving as
    "UTF-16LE/BE" silently wrote UTF-8 (encoding_rs has no UTF-16 encoder, it
    always outputs UTF-8); saves now emit real UTF-16 + BOM via `encode_utf16`;
    (3) saving with `line_ending: "LF"` never normalized CRLF content.
- [x] **Find/Replace** logic: match-navigation + replace-all pure functions
  extracted into `lib/editor/search.ts` (findAll / countMatches / findNextFrom /
  replaceAll) and asserted without a DOM.
  - **Bug found & fixed**: `replace` and `replace-all` actions were **no-ops**
    in `Editor.svelte` — now wired to real document transactions.
- [x] **Settings migration**: `sanitizeSettings()` picks only known, type-checked
  keys from persisted data; unknown keys and out-of-range values fall back to
  defaults without throwing.
  - **Bug found & fixed**: the legacy `sabot-settings` key was checked before
    `devnote-settings`, so a stale legacy key could override newer settings.
    Priority is now newest-first.
- [x] **Golden cases JSON** (`tests/golden_cases.json`) gating editor behavior:
  open-1252-CRLF-csv → correct encoding/endings/no dirty dot, edit → undo →
  dirty clears, dirty-tab close rejected, save-as clears dirty + updates path,
  open-same-path focuses existing. Runs as part of `bun run test` — no separate
  invocation. CI fails if a case regresses.

**Acceptance:** Every user-visible invariant above has an automated test; a
regression in any of them fails CI. ✅ (All new tests green: 92 frontend +
23 Rust.)

---

## Phase 3: Accessibility & Internationalization Foundation

DevNote already has ARIA roles and `prefers-reduced-motion`. This phase makes a11y
and i18n first-class rather than incidental. Completed in the `phase-3-a11y-i18n`
PR; the one remaining item (manual screen-reader session) requires a human.

- [x] **Keyboard-only audit**: full app navigable with keyboard only —
  - Tab bar: roving `tabindex` + `←` `→` `Home` `End` (ARIA tabs pattern),
    `Enter`/`Space` activate.
  - Context menu: auto-focus first item, `↑`/`↓`/`Home`/`End`, Tab wraps,
    Esc closes, focus returns to the originating tab.
  - Find/Replace + Go-to-Line + Recent dialog: auto-focus, Enter/Esc.
  - Language picker: arrows + Esc; blur-based close replaced with
    outside-click (blur closing broke keyboard navigation).
  - No mouse traps anywhere; key map documented in README + `docs/a11y-notes.md`.
- [x] **Focus management**: visible focus ring using `--accent-teal` token
  (`:focus-visible`, mouse focus stays quiet); dialogs trap Tab/Shift+Tab and
  return focus to the editor on close; Esc always cancels the topmost modal.
- [x] **Screen-reader pass (automated part)**: `svelte-check` a11y lints at
  0 errors/0 warnings; `tablist`/`tab`/`aria-selected`, dirty-tab labels,
  `role="menu"`, `alertdialog` + `aria-modal` + labelled/describedby, status bar
  `role="status"` with a dedicated polite live region announcing only
  language/encoding/line-ending changes (volatile Ln/Col/words/chars are
  `aria-hidden` to avoid per-keystroke announcements), toast `role="alert"`.
- [ ] **Manual VoiceOver/NVDA session** — checklist in `docs/a11y-notes.md`,
  needs a human on macOS/Windows; run before the v1.1 LTS tag.
- [x] **i18n plumbing**: `t()` helper (`lib/i18n/i18n.svelte.ts`) with typed
  `en` + `th` dictionaries (Thai = author locale), `{param}` interpolation,
  English fallback, locale-reactive (components re-render on change).
- [x] **Bilingual-ready settings**: `settings.locale` (`system` | `en` | `th`),
  persisted, defaults to `system` (OS locale detection, falls back to `en`).

**Acceptance:** Keyboard-only + reduced-motion pass ✅; SR session logged —
**pending manual run**; `en`+`th` strings resolve through `t()` ✅ (tested).

---

## Phase 4: Editor Power-User Features (still editor-shaped)

Everything here stays inside the "text editor" box — no project tree, no terminal.

- [ ] **Multi-cursor beyond CM default**: column/selection-based multi-edit exposed
  via a menu + keybinding (already available in CodeMirror, currently not surfaced).
- [ ] **Selection history / jump-to-last-edit**: navigate between edit sites.
- [ ] **Bracket/indent guides**: render-only, toggle in View menu, backed by a
  `Compartment`.
- [ ] **Whitespace rendering toggle**: show/hide tabs+spaces (Compartment).
- [ ] **Word count + character count + selection stats**: StatusBar already shows
  words/chars; add selection-relative counts when a selection exists.
- [ ] **Go-to-Symbol** for languages with a parsed tree (Rust/TS/Python): a
  command-palette-style `Ctrl+P`-like jump list driven by CodeMirror's language
  data, editor-local only (no workspace indexing).
- [ ] **Print to PDF**: use Tauri's webview print for a quick "export current tab as
  PDF" — offline, no network.

**Acceptance:** Each feature toggleable, documented in README shortcuts, covered by
at least a smoke test where pure logic exists.

---

## Phase 5: File I/O Hardening & Large-Doc Performance

- [ ] **Streaming open for huge files**: files in the 10–200 MB band currently load
  fully into memory + CodeMirror. Add a lazy/partial read path (read head + tail,
  virtualized view) or at minimum a hard "read-only preview" mode for > 50 MB so the
  app never OOMs on a log file. (Real field use: opening large `.log`/`.csv`.)
- [ ] **Incremental save**: for very large dirty files, diff-and-append or chunked
  write instead of full rewrite, keeping the atomic-rename guarantee.
- [ ] **Watch + external change detection**: if a file open in a tab changes on disk
  (another program), show a "File changed externally — Reload / Ignore" prompt. Uses
  `notify` (Rust) behind a capability; never auto-reloads (data-loss risk).
- [ ] **Encoding override**: when `chardet` is low-confidence, let the user pick the
  decoding from the StatusBar encoding badge before content is committed.
- [ ] **Unsaved-files safety net**: closing the whole window with dirty tabs prompts
  Save All / Don't Save / Cancel (AGENTS.md §9) — verify the `close-requested`
  intercept is wired on all three OSes, not just via menu Quit.

**Acceptance:** 200 MB file opens or is safely refused; external-change prompt works;
window-close dirty check verified on macOS/Windows/Linux.

---

## Phase 6: Settings, Theming & Personalization

- [ ] **Theme system completion**: `light` / `dark` / `system` all resolve correctly
  with `[data-theme]` token overrides; add a third "Sepia" preset reusing the cream
  surface tokens from DESIGN.md (no new hex, only token remap).
- [ ] **Editor font + UI font pickers**: choose JetBrains Mono / Inter / system,
  persisted; respect `static/fonts/` presence at build (offline, no CDN).
- [ ] **Settings UI**: a real settings window (not just menu checks) — tabs for
  Editor / Appearance / Files / Advanced, all bound to `settings.svelte.ts`, with a
  "Reset to defaults" and live preview.
- [ ] **Per-tab overrides**: font size / word wrap / language can be set per tab and
  persist with the tab's recovery data.
- [ ] **Import/Export settings**: write settings JSON to a user-chosen file and
  restore (offline portability between machines).

**Acceptance:** All three themes render with zero inline hex; settings UI round-trips
through export/import.

---

## Phase 7: Performance Budgets & Profiling (the Phase 9 of the example)

The example roadmap demands verified, not claimed, budgets. DevNote gets the same.

- [ ] **Benchmark harness**: a small script (or `cargo bench` for Rust paths +
  a `vitest`/node timing helper for frontend) measuring: cold start to first paint,
  open a 1 MB / 10 MB / 200 MB file, save a 1 MB dirty file, tab switch latency.
- [ ] **Budgets (CI-enforced)**:
  - Cold start < 400 ms on a reference machine (Tauri + CodeMirror warm).
  - Open 1 MB file < 150 ms; 10 MB < 800 ms.
  - Save (atomic) 1 MB < 100 ms.
  - Editor initial JS bundle < 150 KB gzipped (currently ~110 KB baseline — guard it).
  - Installed app footprint < 25 MB on disk (Tauri webview shared with OS).
- [ ] **Bundle-size budget step**: fail the build if the produced binary/installer
  exceeds the disk budget (mirror the example's "fails the build if exceeded").
- [ ] **Profiling on constrained hardware**: run the above on a throttled CI runner
  (or a documented low-end reference machine) at least once; record numbers in
  `docs/perf-baseline.md`.
- [ ] **Lazy language-pack audit**: confirm no language pack is pulled into the
  initial chunk; assert via a bundle report in CI.

**Acceptance:** Budgets are enforced in CI; baseline doc exists; no regression merges
without a noted exception.

---

## Phase 8: Security & Supply-Chain Hardening (the Phase 10 of the example)

- [ ] **`unsafe` audit**: DevNote currently uses zero `unsafe` in app code (Tauri
  internals aside). Document that explicitly and add a `grep`-style CI guard / clippy
  lint confirming no `unsafe` blocks are introduced in `src-tauri/src`.
- [ ] **Capability least-privilege review**: re-confirm `capabilities/default.json`
  grants only what the UI uses (no `fs:*` on renderer, dialog/shell/store scoped).
  Re-audit after every new plugin.
- [ ] **CSP regression test**: a CI step asserts `tauri.conf.json` CSP has no
  `unsafe-inline` / no `*` connect-src / no external font src.
- [ ] **Reproducible build check**: same commit + same inputs → byte-identical
  installer artifact (at least for the non-code-signed, ad-hoc path). Document how to
  verify.
- [x] **Dependency hygiene gate**: `cargo-deny` + `cargo-audit` green in CI,
  driven by `src-tauri/deny.toml` (permissive licenses only, with one explicit,
  documented exception: `chardet` LGPL-3.0 — added in Phase 1).
- [ ] **Dependency bump policy**: Renovate or manual bump policy recorded in
  CONTRIBUTING.md.
- [ ] **Auto-update decision**: Tauri's updater requires signed manifests + a server.
  Decide explicitly: **out of scope for v1 LTS** (offline-first field use means
  manual download). Document the "check for updates" policy (manual, explicit, no
  silent network) even if not implemented, matching the example's
  explicit-confirmation rule.

**Acceptance:** No `unsafe` in app crate; CSP + capabilities audited and CI-guarded;
deny/audit green; reproducible-build notes published.

---

## Phase 9: Packaging, Installers & First Stable Release

- [x] **Windows installer** — NSIS configured in `tauri.conf.json`
  (`targets: "all"`, `nsis` section) and shipped by
  `.github/workflows/release-windows.yml` (tag-triggered, `tauri-action`,
  publishes a GitHub Release with the installer).
- [x] **macOS DMG** — configured in `tauri.conf.json` (custom background,
  ad-hoc signing `signingIdentity: "-"`, entitlements) + `scripts/post-build-macos.sh`
  for entitlement selection. Not yet wired into a release workflow.
- [ ] **Linux**: `.deb` config exists (`targets: "all"`); no `.AppImage`
  verification or CI build yet.
- [ ] **Code signing**: document the macOS Developer-ID + notarization path and the
  Windows Authenticode path; provide a `.github/workflows/release.yml` that signs when
  secrets are present and falls back to ad-hoc otherwise (never fails the build for
  missing secrets on forks).
- [ ] **Release workflow completion** (tag-triggered): builds all three platforms,
  generates `SHA256SUMS.txt`, attaches artifacts + checksums to a GitHub Release,
  reuses the CI lint/test gates as required checks. (Windows-only flow exists today.)
- [ ] **Field-deployment guide** (`docs/DEPLOY.md`): how to copy the binary to an
  offline machine, verify the checksum, report a bug from the field (matches the
  example's field-deployment guide intent, scaled to an editor).
- [ ] **`v1.0.0` → `v1.1.0` LTS tag**: at this point all prior phases' acceptance
  checks pass; CHANGELOG updated with the reconciliation + CI + a11y work.
- [ ] **About dialog**: wire `menu-about` (currently emitted, not handled) to a real
  About window showing version, licenses, and the offline disclaimer.

**Acceptance:** A tagged release produces signed-or-ad-hoc installers for all three
OSes with published checksums; deploy guide exists.

---

## Phase 10: Documentation & Onboarding

- [ ] **AGENTS.md → authoritative spec**: finish Phase 0 reconciliation; keep it the
  single source of truth for AI agents and contributors.
- [ ] **CONTRIBUTING.md**: already exists — extend with the test/CI/commit/lint
  commands, the "no scope creep beyond editor" rule, and the i18n string-extraction
  process.
- [ ] **DESIGN.md**: already the visual bible — add the Sepia preset + focus-ring
  token if Phase 6 ships; keep the "no inline hex" rule enforced via a CI style check
  (a tiny `grep` forbidding `#` color literals in `.svelte`/`.css`).
- [ ] **README.md**: update Features/Shortcuts to match v1.1 reality; add the
  offline-first + no-telemetry statement explicitly (privacy posture).
- [ ] **Architecture doc** (`docs/ARCHITECTURE.md`): IPC boundary (Rust commands only,
  no renderer `fs`), state ownership (Rust owns files/recovery/recent; Svelte owns
  tab UI state; CodeMirror view never stored in a store), and the recovery flow.

**Acceptance:** Docs match code; a style CI step forbids inline hex; architecture doc
explains the IPC boundary.

---

## Future / Ecosystem (explicitly out of scope for v1 LTS)

These are valuable but would turn DevNote into something larger than a text editor.
Each is listed so the line is drawn *consciously*, not by accident:

- **Project explorer / file tree** (Phase 2 candidate in the original AGENTS.md) —
  deferred; risks becoming an IDE. Revisit only if a distinct "DevNote Projects" mode
  is scoped separately.
- **Terminal pane** — out of scope; conflicts with offline-safety + attack surface.
- **Git integration** — out of scope for v1; possible as a separate plugin later.
- **Minimap** — nice-to-have, deferred past LTS.
- **Collaborative / remote editing** — out of scope.
- **Spell check / grammar** — deferred; would pull in a dictionary dependency.
- **Plugin / extension system** — deferred; the example roadmap also lists this as
  future. Only consider after the IPC boundary + capability model are frozen.
- **Snippet library / macro recording** — possible lightweight addition, post-LTS.
- **ARM / Linux-ARM builds (Raspberry Pi class)** — possible once x64 LTS is stable.
- **Additional UI languages** (French, Arabic, Dzongkha, …) — enabled by Phase 3's
  i18n plumbing; blocked only on translator contributions.
- **Auto-update** — explicitly deferred (offline-first); manual download only.

---

## How phases relate (dependency order)

```
Phase 0 (reconcile docs/status)
   │
   ▼
Phase 1 (CI — blocks everything else)
   │
   ▼
Phase 2 (golden regression suite)
   │
   ├─► Phase 3 (a11y + i18n)
   ├─► Phase 4 (editor power features)
   ├─► Phase 5 (file I/O hardening)
   ├─► Phase 6 (settings/theming)
   │
   ▼
Phase 7 (perf budgets)  ──needs──► Phase 1 (CI to enforce)
   │
   ▼
Phase 8 (security hardening)
   │
   ▼
Phase 9 (packaging + v1.1 LTS release)
   │
   ▼
Phase 10 (docs/onboarding)
```

Each phase is "done" only when its acceptance checks pass in CI (or, where manual,
are documented as done). No phase is merged as complete on intent alone.
