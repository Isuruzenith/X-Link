---
title: "09 — Design System: Material 3 Tokens, Components & Icons"
aliases: ["Design System", "Material Web Guide", "Icon Strategy", "UI Tokens"]
tags:
  - frontend
  - design-system
  - ui
  - spec
created: 2026-11-06
updated: 2026-11-06
---

# 09 — Design System: Material 3 Tokens, Components & Icons

**Navigation**: [[00-Home-MOC]] | **Prev**: [[08-User-Guide-and-FAQ]] | **Geometry**: [[wireframes/00-window-and-grid]] | **Specs**: [[specs/001-self-hosted-icons/spec]] · [[specs/004-material-web-ui-conformance/spec]]

This note is the contract for *how X-Link looks*. If a UI change contradicts it, change this note first (see [[specs/README]]).

---

## 1. Library choice

| Question | Decision |
| :--- | :--- |
| Which "Material" library? | **Material Web** (`@material/web`, Material 3 web components, `<md-*>` elements). Not MUI (`@mui/material`), which is a different, React-only library. |
| Why? | Already in the app, framework-agnostic custom elements, small per-component imports, first-class M3 token system (`--md-sys-*`). |
| Risk | The Material Web repo is in **maintenance mode** (no new components, bug fixes case-by-case). Mitigation: import components individually, wrap each in a thin local wrapper (`src/components/md/*`) so a swap to another library only touches wrappers, and keep **layout, navigation rail, cards, tables and toasts as our own CSS** (Material Web does not ship them). |
| Role of Tailwind v4 | **Layout utilities only** (grid, flex, spacing, `min-w-0`). No colors, radii or shadows from Tailwind — those come from tokens. |
| Role of shadcn / Base UI | Behavior only (focus trap, portal) where `md-dialog`/`md-menu` is insufficient. Their *visual* styling is removed in [[specs/004-material-web-ui-conformance/spec]]. |

## 2. Token layers

```text
 seed color ──► M3 scheme (light/dark)  ──►  --md-sys-color-*        (library tokens, generated)
                                        └──►  --xl-*  semantic aliases (app tokens, hand-written)
 components read ONLY --xl-* or --md-sys-*; never raw hex.
```

### 2.1 Required app aliases (`src/styles/tokens.css`)

| Token | Maps to | Used for |
| :--- | :--- | :--- |
| `--xl-canvas` | `--md-sys-color-surface` | Window background behind the rail |
| `--xl-panel` | `--md-sys-color-surface-container-low` | View host / content surface |
| `--xl-card` | `--md-sys-color-surface-container` | Cards |
| `--xl-card-hi` | `--md-sys-color-surface-container-high` | Dialogs, side sheet, menus |
| `--xl-on-surface` | `--md-sys-color-on-surface` | Primary text |
| `--xl-muted` | `--md-sys-color-on-surface-variant` | Secondary text, labels |
| `--xl-outline` | `--md-sys-color-outline-variant` | Dividers, card borders |
| `--xl-accent` | `--md-sys-color-primary` | Primary actions, active nav |
| `--xl-accent-container` | `--md-sys-color-secondary-container` | Active nav pill, selected rows |
| `--xl-ok` / `--xl-warn` / `--xl-err` | custom, see §2.2 | Connection and latency status |
| `--xl-rail-w`, `--xl-page-pad`, `--xl-gap`, `--xl-header-h` | from [[wireframes/00-window-and-grid]] | Layout |

### 2.2 Status colors (semantic, theme-aware)

| State | Light | Dark | Rule |
| :--- | :--- | :--- | :--- |
| Connected / fast `< 100 ms` | `#368360` | `#7bd3a4` | Always paired with text "Connected"/"Protected" |
| Connecting / medium `100–250 ms` | `#b47a29` | `#f0c06c` | Pulsing dot, text "Connecting…" |
| Disconnected | `#7e7576` | `#a99fa0` | Text "Disconnected"/"Unprotected" |
| Error / slow `≥ 250 ms` | `#b3261e` | `#f2b8b5` | Text + icon, never color alone |

Light values reuse the colors already documented in [[wireframes/01-shell-navigation]]; dark values are new and must reach **≥ 4.5:1** against `--xl-card`.

### 2.3 Theme switching

`data-theme="light|dark"` on `<html>` (already written by `setTheme`). Both schemes live in one file; `color-scheme: light dark` is set so native scrollbars and form controls follow. First paint reads `localStorage.theme` synchronously (existing behavior) — keep it.

## 3. Typography

| Role | Class | Size / weight | Use |
| :--- | :--- | :--- | :--- |
| View title | `md-typescale-headline-small` | 24 / 400 | `ViewShell` `<h1>` |
| Card title | `md-typescale-title-medium` | 16 / 500 | Card headers |
| Body | `md-typescale-body-medium` | 14 / 400 | Default |
| Label | `md-typescale-label-large` | 14 / 500 | Buttons, tabs, nav |
| Mono | app class `.xl-mono` | 13 / 400 | Hosts, ports, UUIDs, logs |

Fonts are **self-hosted** (e.g. `@fontsource-variable/inter` and `@fontsource-variable/jetbrains-mono`, or the Roboto Flex the app already ships). No `fonts.googleapis.com` in the app bundle — only the public landing page (`docs/index.html`) may use it.

## 4. Shape, elevation, motion

- Shape: cards `--md-sys-shape-corner-large` (16px), buttons/fields use library defaults, view host `0 0 24px 24px` bottom corners on Expanded only.
- Elevation: flat by default (tonal surfaces). Only dialogs, menus and the side sheet get `md-elevation` level 3.
- Motion: `150–250 ms` standard easing; honor `prefers-reduced-motion` (disable pulse, fade and chart transitions).
- Density: base grid `4px`; touch/click targets ≥ `40px` high in Expanded, `36px` in dense mode (desktop pointer app).

## 5. Component map — what is Material Web, what is ours

| UI need | Implementation | Import / source | Notes |
| :--- | :--- | :--- | :--- |
| Primary / secondary buttons | `md-filled-button`, `md-filled-tonal-button`, `md-outlined-button`, `md-text-button` | `@material/web/button/*` | Connect button = `md-filled-button` with `--md-filled-button-container-height: 56px`, color driven by status |
| Icon buttons | `md-icon-button` (+ `md-filled-tonal-icon-button`) | `@material/web/iconbutton/*` | Need `aria-label` always |
| Icons | `<md-icon>` through our `<Icon name>` wrapper | `@material/web/icon/icon.js` + self-hosted font | See §6 |
| Switch | `md-switch` | `@material/web/switch/switch.js` | Inside `FormSwitchRow` |
| Text field | `md-outlined-text-field` | `@material/web/textfield/*` | Password reveal = trailing `md-icon-button` |
| Select | `md-outlined-select` + `md-select-option` | `@material/web/select/*` | |
| Segmented control | `md-outlined-segmented-button-set` (**labs**) | `@material/web/labs/segmentedbuttonset/*` | Labs API may change; wrapper isolates it |
| Tabs (node editor, settings in Medium) | `md-tabs` + `md-primary-tab` | `@material/web/tabs/*` | |
| Chips (ports, protocols, filters) | `md-assist-chip`, `md-filter-chip` | `@material/web/chips/*` | |
| Lists (profiles, menus) | `md-list`, `md-list-item`, `md-menu`, `md-menu-item` | `@material/web/list/*`, `menu/*` | |
| Dialog (diff preview, confirm, passphrase) | `md-dialog` | `@material/web/dialog/dialog.js` | Built-in focus trap and Esc |
| Progress | `md-circular-progress`, `md-linear-progress` | `@material/web/progress/*` | |
| Divider / ripple / focus ring | `md-divider`, `md-ripple`, `md-focus-ring` | `@material/web/*` | |
| **Navigation rail / drawer** | **Custom** `NavRail` | local CSS + `md-list-item`-style items | Not in Material Web |
| **Cards** | **Custom** `.xl-card` | local CSS + `md-elevation` | `labs/card` exists but is preview; avoid |
| **Data tables** | **Custom** + `react-window` | local | Not in Material Web |
| **Toasts / snackbar** | **Custom** `ToastContainer` | local | Not in Material Web |
| **Side sheet** | **Custom** `<dialog>`-based | local | Replaces Radix `Sheet` styling |
| Tooltips | native `title` + `aria-label` for now | – | No `md-tooltip` |
| Charts | **Custom** SVG `TrafficChart` | local | Colors from `--xl-*` |

**Register only what you use** — `import '@material/web/button/filled-button.js'`, never `@material/web/all.js` (bundle size, startup time).

## 6. Icons — the strategy that survives `tauri build`

### 6.1 Why icons vanish in the built app

`<md-icon>` draws an icon by writing its **name as text** (`speed`, `settings`) and relying on a **Material Symbols font ligature** to turn it into a glyph. If the font is missing the user sees the raw word, a blank box, or nothing. In `tauri dev` this often works because a CDN stylesheet loads over the internet with no strict CSP; in the packaged app the WebView serves files from the app's own origin, offline-capable, with whatever CSP `tauri.conf.json` sets. Full diagnosis in [[specs/001-self-hosted-icons/spec]].

### 6.2 Rules

1. **Self-host the font.** `npm i @fontsource-variable/material-symbols-rounded`, import its CSS once in `main.tsx`. Vite bundles the `.woff2` into `dist/assets`.
2. **Point `md-icon` at it.** Set `--md-icon-font` to the *exact* `font-family` string declared by the package CSS (check the file in `node_modules`; Fontsource variable packages usually append `Variable`, e.g. `'Material Symbols Rounded Variable'`). Material Web's default is `Material Symbols Outlined`, so a mismatch leaves icons blank.
3. **No CDN** for fonts, icons, flags or scripts at runtime. A CI check (`npm run check:offline`) fails the build if `dist/` contains `fonts.googleapis.com`, `fonts.gstatic.com`, `flagcdn.com`, `cdn.jsdelivr`, or `unpkg`.
4. **CSP allows only what we ship**: `font-src 'self' data:`.
5. **Gate first paint on the font**: await `document.fonts.load('24px "<family>"')` (with a 1.5 s timeout) before removing the splash; add class `icons-ready` to `<html>`. Until then `md-icon` is `visibility:hidden` so ligature text never flashes.
6. **One `<Icon name size>` wrapper.** No raw `<md-icon>` in views. The wrapper owns size, `aria-hidden`, and the allow-list of icon names (a TypeScript union generated from the names the app uses) so typos fail at compile time.
7. **Fallback plan B**, if bundle size or font quirks bite on WebKitGTK: build-time SVG sprite from the same ~60 names; wrapper API stays identical.

### 6.3 Icon names used by the shell (allow-list seed)

`speed`, `folder_open`, `alt_route`, `hub`, `terminal`, `settings`, `power_settings_new`, `check_circle`, `error`, `warning`, `info`, `close`, `menu`, `search`, `content_copy`, `delete`, `edit`, `refresh`, `difference`, `arrow_downward`, `arrow_upward`, `dns`, `lan`, `visibility`, `visibility_off`, `add`, `more_vert`.

## 7. Accessibility baseline

- Contrast ≥ 4.5:1 text, ≥ 3:1 UI components, in both themes.
- Every icon-only control has `aria-label`; decorative icons `aria-hidden="true"`.
- Status is never color-only (text + icon).
- Focus ring visible (`md-focus-ring`); full keyboard path through nav → header actions → content.
- Dialogs trap focus and restore it to the trigger; Esc closes.
- `prefers-reduced-motion` and `forced-colors` respected.

## 8. Do / Don't

| Do | Don't |
| :--- | :--- |
| `color: var(--xl-muted)` | `color: #9ca3af`, `text-gray-400` |
| `<Icon name="speed" />` | `<md-icon>speed</md-icon>` in a view |
| `import '@material/web/switch/switch.js'` | `import '@material/web/all.js'` |
| Give a flex child `min-w-0` | Fix overflow with `overflow-x: scroll` on `body` |
| Update this note, then code | Introduce a new radius/shadow ad hoc |
