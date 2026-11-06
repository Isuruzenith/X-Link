---
title: "003 — Responsive, Fit-to-Window Layout — Plan"
tags:
  - spec
  - sdd
---

# 003 — Plan

**Navigation**: [[specs/003-responsive-fit-layout/spec]] | [[specs/003-responsive-fit-layout/tasks]]

## 1. Approach
Replace the *mechanism* first (shell grid + attributes + scroll regions), then convert views one by one. Views are converted from simplest to most complex so each step is a small diff.

## 2. Design

### 2.1 Shell CSS (`src/styles/layout.css`)
```css
html, body, #root { height: 100%; margin: 0; overflow: hidden; }
.app-root  { display: grid; grid-template-columns: var(--xl-rail-w) minmax(0, 1fr); height: 100%;
             background: var(--xl-canvas); }
.app-navigation { width: var(--xl-rail-w); min-height: 0; overflow: hidden; }
.view-host { display: grid; grid-template-rows: auto minmax(0, 1fr); min-width: 0; min-height: 0;
             background: var(--xl-panel); }
.view-header { height: var(--xl-header-h); padding-inline: var(--xl-page-pad); display: flex; align-items: center; }
:root[data-dense="true"] .view-header .subtitle { display: none; }
.view-body  { min-height: 0; min-width: 0; padding: var(--xl-page-pad); display: grid; gap: var(--xl-gap); }
.scroll-region { overflow: auto; min-height: 0; min-width: 0; overscroll-behavior: contain; }
.xl-card { display: grid; grid-template-rows: auto minmax(0, 1fr); min-height: 0; min-width: 0;
           padding: var(--xl-card-pad, 16px); border-radius: var(--md-sys-shape-corner-large, 16px);
           background: var(--xl-card); }
@media (min-width: 1400px) { .view-body { max-width: 1280px; margin-inline: auto; width: 100%; } } /* allowed: tokens/layout file only */
```

### 2.2 View grids

| View | Expanded | Medium |
| :--- | :--- | :--- |
| Dashboard | `rows: 120px minmax(0,1fr)`; main grid `cols: minmax(0,1.1fr) minmax(0,1fr)` | hero `100px`; main grid single card with `md-tabs` *Servers / Chart / Session* |
| Profiles | `cols: 320px minmax(0,1fr)`; left `rows: auto minmax(0,1fr)` | single column; import collapsed (`md-filled-tonal-button` toggles) |
| Routing | `rows: auto minmax(0,1fr)`; top `cols: 1fr 1fr` | top row becomes two `md-assist-chip` summaries that open `md-dialog` |
| Connections | `rows: auto minmax(0,1fr)` (toolbar, list) | counter hidden |
| Logs | `rows: auto minmax(0,1fr)` | level chips → `md-outlined-select` |
| Settings | `cols: 180px minmax(0,1fr)` | `rows: auto minmax(0,1fr)` with `md-tabs` strip |

### 2.3 Tables
Grid template per table, e.g. servers: `grid-template-columns: minmax(0,1fr) 72px 64px 72px;` Row height 40 px (36 dense). Virtualize with `react-window` when `rows > 50`; list height comes from its `xl-card` via `AutoSizer`-style `ResizeObserver` (react-window v2 accepts a style height — pass `100%` of a `min-height:0` parent).

### 2.4 Chart
`TrafficChart` already uses `ResizeObserver`; remove any fixed `height` prop and let the card's `minmax(0,1fr)` row size it; add `min-height: 96px`.

### 2.5 Automated fit test (`e2e/fit.spec.ts`, Playwright against `vite preview`)
```ts
import { test, expect } from '@playwright/test';
const SIZES = [[720,520],[840,600],[1000,680],[1280,720],[1366,728],[1920,1000]];
const VIEWS = ['dashboard','profiles','routing','connections','logs','settings'];
for (const [w,h] of SIZES) for (const v of VIEWS)
  test(`${v} fits ${w}x${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto(`/#${v}`);                          // plain #name routing, see publishing rules
    const m = await page.evaluate(() => ({
      sh: document.documentElement.scrollHeight, ih: innerHeight,
      sw: document.documentElement.scrollWidth,  iw: innerWidth,
      scrollers: [...document.querySelectorAll('*')].filter(e => /(auto|scroll)/.test(getComputedStyle(e).overflowY)
                  && e.scrollHeight > e.clientHeight + 1).length }));
    expect(m.sh).toBeLessThanOrEqual(m.ih);
    expect(m.sw).toBeLessThanOrEqual(m.iw);
  });
```
The app must tolerate running without Tauri (it already has a safe environment detector in `utils/tauri.ts`); seed stores with fixture data under `?fixtures=1` for deterministic screenshots.

## 3. Conversion order
1. Shell + tokens → 2. Logs → 3. Connections → 4. Routing → 5. Settings → 6. Profiles → 7. Dashboard → 8. Node editor sheet → 9. Compact drawer.

## 4. Files touched
`src/styles/layout.css`, `src/styles/tokens.css`, every `src/views/*View.tsx`, `ViewShell.tsx`, `NavRail.tsx`, `TrafficChart.tsx`, `src/index.css`/`App.css` (delete obsolete utility classes `utility-view`, `settings-layout`, `routing-layout`, `profiles-grid` once unused), `e2e/fit.spec.ts`, `playwright.config.ts`.

## 5. Risks
| Risk | Mitigation |
| :--- | :--- |
| Hidden `100vh` / `max-height` caps resurface | AC-002/003 greps in CI |
| Virtualized lists collapse to 0 height | parent must be `min-height: 0` with a grid row `minmax(0,1fr)`; Playwright asserts list `clientHeight > 100` |
| Medium tabs change information density | Spec fixes content per tab; no data removed |
| Regression in polling views | Zero JS data-flow edits; views only change markup/CSS |

## 6. Rollout / rollback
One PR per view (steps 2–8). Each PR independently revertable; shell PR lands first and is behavior-neutral on its own.
