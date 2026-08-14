# Accessibility Notes

Screen-reader and keyboard-verification log for DevNote (Roadmap Phase 3).

## Automated checks (in CI, `bun run check`)

- `svelte-check` runs Svelte's `a11y_*` lints — **0 errors, 0 warnings** on
  the main branch. New warnings are treated as errors (CI `-D` discipline on
  the Rust side; svelte-check must stay clean).
- Focus ring: global `:focus-visible` outline in `--accent-teal` (see
  `src/app.css`). Mouse focus stays quiet (`:focus { outline: none }`).
- `prefers-reduced-motion` disables transitions/animations globally
  (`src/app.css`) and per-component (dialogs, tabs, toasts).

## Keyboard-only audit (implemented in Phase 3)

| Surface | Keys | Implementation |
|---|---|---|
| Tab bar | `←` `→` `Home` `End` move focus + activate; `Enter`/`Space` activate | roving `tabindex` (active tab = 0) — `TabBar.svelte` / `Tab.svelte` |
| Tab close button | `Tab` to reach, `Enter`/`Space` to close | native button |
| Context menu (right-click) | auto-focus first item; `↑` `↓` `Home` `End` cycle; `Tab` wraps; `Esc` closes; focus returns to the originating tab | `TabBar.svelte` |
| Language picker (status bar) | auto-focus first option; `↑` `↓` `Home` `End`; `Esc` closes; outside-click closes (blur-based closing removed — it broke keyboard nav) | `StatusBar.svelte` |
| Confirm dialog | focus first button on open; `Tab`/`Shift+Tab` trapped inside; `Esc` = Cancel; focus returns to the editor on close | `ConfirmDialog.svelte` |
| Recent-files dialog | auto-focus; `Esc` closes | `+page.svelte` |
| Go-to-line | auto-focus input; `Enter` go, `Esc` close | `+page.svelte` |
| Find/Replace | auto-focus input; `Enter`/`Shift+Enter` next/prev; `Esc` close | `FindReplace.svelte` |
| Global shortcuts | `Ctrl+N/O/S/W/F/H/G/Tab/1-9` etc. | `+page.svelte` global keydown |

No mouse traps: every modal can be dismissed with `Esc`, and keyboard focus
never leaves the app chrome.

## Screen-reader semantics (implemented)

- `role="tablist"` / `role="tab"` + `aria-selected` on the tab bar; dirty tabs
  get `aria-label="{name} (unsaved)"` (i18n-aware).
- `role="menu"` / `role="menuitem"` on the tab context menu.
- `role="alertdialog"` + `aria-modal="true"` + `aria-labelledby` /
  `aria-describedby` on the confirm dialog.
- `role="dialog"` + `aria-modal="true"` on recent-files dialog; `role="dialog"`
  on go-to-line.
- Status bar: `role="status"` with volatile values (Ln/Col/words/chars)
  `aria-hidden`; a visually-hidden `aria-live="polite"` region announces only
  meaningful changes (language / encoding / line endings) — avoiding the
  keystroke-by-keystroke chatter a naive live region would cause.
- Toast: `role="alert"`.

## Manual screen-reader session log

**Status: pending — requires a human with VoiceOver (macOS) and/or NVDA
(Windows).**

Checklist for the next session:

- [ ] Tab bar announces selected/unselected state; arrows switch tabs.
- [ ] Context menu opens with focus on the first item; screen reader reads the
      menu items as a menu.
- [ ] Confirm dialog announces itself as a dialog; Tab is trapped; focus
      returns to the editor after closing.
- [ ] Status bar announces language/encoding/line-ending changes but not every
      cursor move.
- [ ] Toast messages are announced (`role="alert"`).
- [ ] Language picker opens and closes cleanly with arrow keys.
- [ ] Find/Replace panel is reachable and usable by keyboard alone.
- [ ] Full app usable without a mouse (record any trap found).

Record: date, OS + SR version, pass/fail per item, and any fixes applied.
