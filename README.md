# DevNote

```
██████╗ ███████╗██╗   ██╗███╗   ██╗ ██████╗ ████████╗███████╗
██╔══██╗██╔════╝██║   ██║████╗  ██║██╔═══██╗╚══██╔══╝██╔════╝
██║  ██║█████╗  ██║   ██║██╔██╗ ██║██║   ██║   ██║   █████╗
██║  ██║██╔══╝  ╚██╗ ██╔╝██║╚██╗██║██║   ██║   ██║   ██╔══╝
██████╔╝███████╗ ╚████╔╝ ██║ ╚████║╚██████╔╝   ██║   ███████╗
╚═════╝ ╚══════╝  ╚═══╝  ╚═╝  ╚═══╝ ╚═════╝    ╚═╝╚══════╝
```

---

## ◆ PULSE

An editor should get out of the way of the note. DevNote is a minimal,
opinionated desktop text editor - Tauri 2 shell, CodeMirror 6 engine,
cream canvas and coral accent - built for the person who wants multi-tab
editing, honest file handling, and a warm page, without a project tree
or a terminal in the way. It edits. It saves safely. It remembers where
you were.

| P0-P2 ▣ | P3 ▣ | P4 ▣ | P5 ▣ | P6-P10 ☐ |
|---|---|---|---|---|

*Foundation, CI, the golden regression suite, accessibility, power-user
features, and file I/O hardening are sealed. Settings depth, performance
budgets, security hardening, and the first stable release stand open.*

> Built with Tauri 2 + Svelte 5 + CodeMirror 6, Rust 2024 on one side and
> TypeScript on the other - macOS-first, warm by design.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

Two runtimes, one command.

```
⟫ bun install
⟫ bun run tauri dev
```

The release artifact:

```
⟫ bun run tauri build
```

Signing notes for macOS: ad-hoc identity by default (`entitlements-dev.plist`,
no sandbox - required for Finder "Open With" under ad-hoc signing); pass
`--production` to `scripts/post-build-macos.sh` for the sandboxed
entitlements used under a paid Developer ID.

<details>
<summary>Prerequisites</summary>

- [Rust](https://rustup.rs/) (latest stable)
- [bun](https://bun.sh/) - `curl -fsSL https://bun.sh/install | bash`
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

</details>

---

## ◆ ANATOMY

Two sides of one window, one opinion: files are handled honestly.

- **Edits** - CodeMirror 6 powers the page: 10+ language packs loaded on
  demand, regex find & replace, multi-cursor, go-to-symbol, and
  bracket-aware guides - an editor's editor, no project tree in sight.
- **Guards** - file I/O is the fortress: encoding detection with
  confidence scores (UTF-8, UTF-16 BOM variants, windows-1252), line-ending
  round-trips, atomic chunked writes that leave no `.tmp` behind, a 200 MB
  hard cap with a read-only preview mode for the 50-200 MB band, and
  external-change detection via `notify` - debounced, self-save-suppressed,
  never auto-reloading without consent.
- **Remembers** - tabs with dirty indicators, recovery writes every 15 s
  (hashed on path + content, never on time), recent files, and settings
  that sanitize unknown keys instead of crashing on them.
- **Watches** - the window close is intercepted: dirty tabs trigger the
  Save All / Don't Save / Cancel flow on every platform - a failed save
  aborts the close, always.
- **Warms** - the design system in `docs/DESIGN.md`: cream canvas, coral
  accent, tokens in CSS custom properties - dark mode is a theme toggle,
  not a markup fork.

---

## ◆ RITUALS

**The core ceremony** - the daily note:

1. `Ctrl+N` for a fresh tab, `Ctrl+O` for a file. Open-already-open
   focuses the existing tab instead of duplicating it.
2. Type. `Ctrl+D` grows the cursor to the next occurrence, `Ctrl+Alt+-`
   jumps back to the last edit site, `Ctrl+P` prints the page to PDF.
3. `Ctrl+S` saves atomically - rename, never truncate. The dirty dot
   clears; the path updates.
4. Close with `Ctrl+W`. Dirty tabs are asked, not assumed.

**The ceremony of honesty** - a file changed on disk is reported, never
silently reloaded. A large file opens as a read-only preview with a badge
instead of a lie about editability. A low-confidence encoding asks which
one it is before a single byte is committed.

**The ceremony of safety** - every save is atomic, every close is
guarded, every unsaved word is recoverable. The editor may be minimal;
its promises are not.

---

## ◆ ECHOES

**Where this artifact is heading**

```
P0-P2 ▸ foundation, CI, golden regression suite (92 frontend + 23 Rust) ▸ sealed
P3-P5 ▸ a11y & i18n, power-user features, file I/O hardening ──────────── ▸ sealed
P6    ▸ settings window, themes, per-tab overrides ────────────────────── ▸ open
P7    ▸ performance budgets, CI-enforced ──────────────────────────────── ▸ open
P8    ▸ unsafe audit, CSP regression, reproducible builds ─────────────── ▸ open
P9-P10 ▸ macOS DMG release wiring, onboarding docs ─────────────────────── ▸ open
```

**Raising the artifact** - the ground truth lives in `docs/STATUS.md`;
the path in `docs/ROADMAP.md`; the rules in `AGENTS.md` and
`docs/AGENTS-RUST.md`. Gates before any PR: `bun run check`, vitest,
`cargo clippy -- -D warnings`, `cargo test`, and the golden cases in
`tests/golden_cases.json`.

**Status** - CI runs the full matrix (frontend + Rust, three OSes) plus a
Tauri build smoke on every push. [Watch the gates](.github/workflows).

---

```
  ─────────────────────────────────────────
   A minimal editor is not a small promise.
   It is a small surface with a deep floor.
  ─────────────────────────────────────────
```

Licensed under the [MIT License](LICENSE).