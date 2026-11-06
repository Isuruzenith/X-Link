---
title: "001 — Self-Hosted Icons & Offline-Safe Assets — Spec"
aliases: ["Icon Fix", "ICON-001"]
tags:
  - spec
  - sdd
  - frontend
  - bug
status: ready
created: 2026-11-06
---

# 001 — Self-Hosted Icons & Offline-Safe Assets

**Navigation**: [[specs/README]] | **Plan**: [[specs/001-self-hosted-icons/plan]] | **Tasks**: [[specs/001-self-hosted-icons/tasks]] | **Design rules**: [[09-Design-System]] §6

## 1. Problem

After `tauri build`, the installed app shows **no icons** (blank squares, or the raw words `speed`, `settings`, `hub`). Icons are drawn by `<md-icon>` through the `Icon` wrapper (`src/components/ui/icon.tsx`) and depend on a *Material Symbols* font that is not reliably available to the packaged WebView. The same font works in `tauri dev`.

## 2. Goals / Non-goals

- **Goal:** icons render identically in dev, in the installer build, offline, in light and dark, on Windows, macOS and Linux.
- **Goal:** make this class of bug impossible to ship again (automated offline check).
- **Goal:** remove other runtime CDN dependencies found during the audit (flags, fonts).
- **Non-goal:** redesign or rename icons. **Non-goal:** switch library to MUI.

## 3. Functional requirements

| ID | Requirement |
| :--- | :--- |
| FR-001 | The Material Symbols Rounded font SHALL ship inside the app bundle (no network fetch). |
| FR-002 | `md-icon` SHALL use that font via `--md-icon-font`, with the family string matching the bundled CSS exactly. |
| FR-003 | Raw ligature text SHALL never be visible: icons are hidden until the font has loaded, with a 1.5 s upper bound. |
| FR-004 | The Tauri CSP SHALL permit the bundled font and nothing remote (`font-src 'self' data:`). |
| FR-005 | `index.html`, CSS and TS under `src/` SHALL contain no `fonts.googleapis.com`, `fonts.gstatic.com`, or other CDN URL for runtime assets. |
| FR-006 | All icons SHALL be rendered through one `<Icon name size>` component with a typed allow-list of names. |
| FR-007 | Country flags SHALL render without network (bundled SVG set), with the current heuristic fallback to a globe icon. |
| FR-008 | `npm run check:offline` SHALL fail if `dist/` references a forbidden host. |

## 4. Acceptance criteria

| ID | Given / When / Then | Covers |
| :--- | :--- | :--- |
| AC-001 | Given a packaged installer on a clean Windows VM **with networking disabled**, when the app opens, then all six rail icons and the Connect button icon are visible | FR-001, FR-002 |
| AC-002 | Given the same build on macOS and on Ubuntu (WebKitGTK), then icons render | FR-001, FR-002 |
| AC-003 | Given a throttled-slow start (CPU 6× slowdown in DevTools), then no icon name text is ever visible during load | FR-003 |
| AC-004 | `npm run build && npm run check:offline` exits 0; adding `<link href="https://fonts.googleapis.com/…">` makes it exit non-zero | FR-005, FR-008 |
| AC-005 | DevTools console shows **zero** CSP violations while visiting all six views and opening the Node Editor | FR-004 |
| AC-006 | In DevTools, `document.fonts.check('24px "<family>"')` returns `true` after load, and Network tab shows the `.woff2` served from the app origin | FR-001 |
| AC-007 | Toggle theme light/dark and resize 720→1280: icons stay crisp, correct size (24 px default, 16/20 px via `size`) | FR-006 |
| AC-008 | Offline, the server table shows bundled flags for tags with country codes and a globe icon otherwise | FR-007 |
| AC-009 | Installer size grows by less than **3 MB** compared with the previous build | – |

## 5. Decisions

| ID | Decision | Alternatives | Why |
| :--- | :--- | :--- | :--- |
| D-001 | Self-host with `@fontsource-variable/material-symbols-rounded` | `material-symbols` package; Google CDN; SVG sprite | Smallest change; keeps `md-icon` ligature API; packaged by Vite. SVG sprite stays as plan B |
| D-002 | Gate on `document.fonts.load` + `icons-ready` class | `font-display: block` only | `block` hides text but ligature words can still flash on some engines |
| D-003 | Bundle flag SVGs for the 18 supported codes | Keep `flagcdn.com` | Constitution II (offline) |

## 6. Open questions

- **OQ-1** Which of the root-cause hypotheses in the plan is the real one? → answered by task T-001 (10-minute diagnosis on the built app).
- **OQ-2** Is Windows build using `devtools` feature in release so T-001 can run on the installed app? If not, use `tauri build --debug`.

## 7. Docs to update

[[02-Frontend-UI-Architecture]] §1 (icon bullet) · [[09-Design-System]] §6 · [[00-Home-MOC]] (Known issues) · [[99-Vault-Changelog]]
