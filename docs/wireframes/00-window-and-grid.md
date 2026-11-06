---
title: "Wireframe 00 — Window, Size Classes & Fit-to-Window Grid"
aliases: ["Window Geometry", "Size Classes", "Fit Rules", "Canonical Geometry"]
tags:
  - wireframe
  - ui
  - layout
  - design-system
  - spec
created: 2026-11-06
updated: 2026-11-06
---

# Wireframe 00 — Window, Size Classes & Fit-to-Window Grid

**Navigation**: [[00-Home-MOC]] | **Design tokens**: [[09-Design-System]] | **Specs**: [[specs/002-desktop-window-shell/spec]] · [[specs/003-responsive-fit-layout/spec]] | **Layouts at scale**: [[09-fit-layouts]]

> [!IMPORTANT] Single source of truth
> This note defines the **only** window size, breakpoints and spacing numbers for X-Link. Every other wireframe, spec and CSS file must quote these values, not invent its own. Older notes used four different breakpoint sets (`1140 / 1050 / 850 / 700 / 440`) and three different window sizes (`1080×700`, `950×650`, `380×550` min). They are superseded — see the migration map in §6.

---

## 1. Window

| Property | Value | Why |
| :--- | :--- | :--- |
| Default inner size | **1000 × 680 px** (logical) | Fits the *Expanded* class with room for the 220px rail; clamped at first launch (see §2) |
| Minimum inner size | **720 × 520 px** | Smallest size where the *Medium* layout still shows every control without document scroll |
| Maximum | none | Content grows; cards use `max-width` rules from §4 |
| `resizable` | `true` (default) | Hard-fixed alternative in §2 |
| `center` | `true` | First launch only |
| Native chrome | Windows/Linux: native title bar. macOS: `titleBarStyle: "Overlay"` + drag strip | Unchanged from architecture note |

Window sizes are **logical pixels** — Windows 125 % / 150 % scaling does not change the numbers below, it changes how much of the monitor they occupy.

## 2. Window configuration (target)

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "X-Link",
        "width": 1000,
        "height": 680,
        "minWidth": 720,
        "minHeight": 520,
        "resizable": true,
        "center": true,
        "visible": false
      }
    ]
  }
}
```

- `visible: false` + the existing `show_window` call after hydration prevents a white flash and a wrong-size first paint.
- **Clamp on first show** (Rust `setup` hook): `inner = min(1000×680, 92 % of the current monitor work area)`, never below the minimum. This matters on 1920×1080 @ 150 % (a 1280×720 logical desktop): 680 px of webview + ~32 px title bar + taskbar would not fit.
- **Hard-fixed variant** (if a truly non-resizable window is wanted): set `"resizable": false`, `"maximizable": false`, and `"minWidth"/"maxWidth"` = `"minHeight"/"maxHeight"` = the default. The layout rules below still hold; only the *Expanded* tier is ever reachable. See decision D-002 in [[specs/002-desktop-window-shell/spec]].

## 3. Size classes (Material 3 window size classes, by inner width)

| Class | Inner width | Reachable in Tauri? | Navigation | `--rail-w` | Page padding | Card gap |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Compact** | `< 600px` | No (min is 720). Browser dev preview only | Modal drawer `260px` + top app bar menu button | `0` | `12px` | `12px` |
| **Medium** | `600 – 839px` (in Tauri: `720 – 839`) | Yes | Icon-only rail `80px`, labels as tooltips/`aria-label` | `80px` | `12px` | `12px` |
| **Expanded** | `≥ 840px` | Yes (default) | Standard rail `220px` with labels | `220px` | `16px` | `16px` |

Height modifier: `max-height: 600px` ⇒ **dense** — header drops its subtitle row (`64px → 48px`), card padding `16px → 12px`. This is what keeps the 720×520 window usable.

Implementation hook: one `useWindowClass()` hook (ResizeObserver on `documentElement`) sets `data-size="compact|medium|expanded"` and `data-dense="true|false"` on `<html>`. CSS reads those attributes; components never read `window.innerWidth` themselves.

## 4. Fit-to-window rules (non-negotiable)

1. **The document never scrolls.** `html, body, #root { height: 100%; overflow: hidden; }`.
2. Shell is a grid: `grid-template-columns: var(--rail-w) minmax(0, 1fr); height: 100%`.
3. Every grid/flex child on the path to a scroller carries `min-width: 0; min-height: 0`. (Missing `min-height: 0` is the #1 reason a "fixed" window still overflows.)
4. A view is `grid-template-rows: auto minmax(0, 1fr)` — header, then one body.
5. **Only named regions scroll**: `.scroll-region { overflow: auto; min-height: 0; }` — server list, rules table, connections list, logs, settings content pane, node-editor body. Everything else must fit.
6. Tables/lists with many rows are virtualized (`react-window`) and sized by their card, never by their row count.
7. Text truncates (`text-overflow: ellipsis`) — it never widens a card. Long tags/hosts get `title` tooltips.
8. Use `height: 100%` chains, **not** `100vh` (WebKitGTK/WKWebView toolbar quirks).
9. No fixed pixel widths on content cards; use `minmax(0, 1fr)` columns with the clamps in §5.

```css
html, body, #root { height: 100%; margin: 0; overflow: hidden; }
.app-root   { display: grid; grid-template-columns: var(--rail-w) minmax(0, 1fr); height: 100%; }
.view-host  { display: grid; grid-template-rows: auto minmax(0, 1fr); min-width: 0; min-height: 0; }
.view-body  { min-height: 0; padding: var(--page-pad); display: grid; gap: var(--gap); }
.scroll-region { overflow: auto; min-height: 0; overscroll-behavior: contain; }
```

## 5. Vertical budgets (what "fits" means)

| Region | Expanded 1000×680 | Min 720×520 (Medium + dense) |
| :--- | :--- | :--- |
| View header | `64px` | `48px` |
| Body padding (top+bottom) | `32px` | `24px` |
| **Usable body height** | **`584px`** | **`448px`** |
| Dashboard hero strip | `120px` | `100px` |
| Dashboard main grid | `584 − 120 − 16 = 448px` | `448 − 100 − 12 = 336px` |
| Server row height | `40px` → ~10 rows visible | `36px` → ~8 rows visible |

| Region | Expanded | Medium |
| :--- | :--- | :--- |
| Rail | `220px` | `80px` |
| Usable body width | `1000 − 220 − 32 = 748px` | `720 − 80 − 24 = 616px` |

> [!WARNING] macOS overlay title bar
> Older notes reserve a `72px` top inset on macOS. Traffic lights need roughly `28–40px` of vertical clearance, so a `72px` inset would eat 12 % of the 580px budget. Treat `40px` as the target and **verify against `NavRail.tsx` / `ViewShell.tsx`** before changing it (open question OQ-3 in [[specs/002-desktop-window-shell/spec]]).

## 6. Migration map from legacy breakpoints

| Legacy tier in notes 02–08 | Legacy width | → Canonical class |
| :--- | :--- | :--- |
| Desktop Wide | `> 1140px` / `> 1050px` | Expanded (≥ 840) — with the *wide* clamp: content `max-width: 1280px` centered when window > 1400 |
| Desktop Standard / Tablet | `701 – 1140px` / `851 – 1140px` | Expanded (840+) or Medium (720–839) depending on width |
| Tablet / Narrow | `≤ 850px` | Medium |
| Mobile Narrow | `≤ 700px` | Compact (browser preview only) |
| Extreme Narrow | `≤ 440px` | Compact (same rules; no extra tier) |

The per-view tables in [[wireframes/02-dashboard]] … [[wireframes/08-settings]] document **today's CSS**. The target is the three classes above, tracked as tasks in [[specs/003-responsive-fit-layout/tasks]].

## 7. Test matrix (must pass with zero document scroll)

| Inner size | Class | Typical real-world case |
| :--- | :--- | :--- |
| 720 × 520 | Medium + dense | Minimum |
| 840 × 600 | Expanded boundary | Snap-half on a 1680 monitor |
| 1000 × 680 | Expanded | Default |
| 1280 × 720 | Expanded | 1080p @ 150 % maximized |
| 1366 × 728 | Expanded | Budget laptop |
| 1920 × 1000 | Expanded + wide clamp | 1080p @ 100 % maximized |
| 599 × 800 *(browser only)* | Compact | Dev-preview responsive check |
