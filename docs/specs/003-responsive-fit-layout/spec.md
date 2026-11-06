---
title: "003 — Responsive, Fit-to-Window Layout — Spec"
aliases: ["Responsive Spec", "Fit Spec"]
tags:
  - spec
  - sdd
  - frontend
  - layout
status: ready
created: 2026-11-06
---

# 003 — Responsive, Fit-to-Window Layout

**Navigation**: [[specs/README]] | **Plan**: [[specs/003-responsive-fit-layout/plan]] | **Tasks**: [[specs/003-responsive-fit-layout/tasks]] | **Rules & budgets**: [[wireframes/00-window-and-grid]] | **Drawings**: [[wireframes/09-fit-layouts]]

## 1. Problem

Layout is driven by five different breakpoint sets and `vh`-based `max-height` values (`min(68vh, 800px)`, `max-height: 360px`). Some views scroll the whole page, tables rely on fixed `min-width`s (680 / 760 px), and nothing ensures each view fits the window. The brief: the window is fixed-size and desktop-shaped, the layout behaves like Material responsive design, and **everything fits the window size**.

## 2. Goals / Non-goals

- **Goal:** three size classes + dense mode only; all views fit with zero document scroll at every size in the test matrix.
- **Goal:** exactly one scroll region per list/table/pane, sized by its card.
- **Non-goal:** new features or new views. **Non-goal:** touching data flow, polling or stores.

## 3. Functional requirements

| ID | Requirement |
| :--- | :--- |
| FR-001 | `html, body, #root` SHALL be `height:100%; overflow:hidden`; shell and view host follow the grid in [[wireframes/00-window-and-grid]] §4. |
| FR-002 | Breakpoints SHALL exist only as `data-size` / `data-dense` attributes ([[specs/002-desktop-window-shell/spec]] FR-005). CSS SHALL NOT use raw `@media (min/max-width)` except inside the token file. |
| FR-003 | Each view SHALL declare its single scroll region with `.scroll-region`; no other element may produce a scrollbar. |
| FR-004 | No component SHALL use `vh`/`vw` units or fixed `max-height` pixel caps for layout. |
| FR-005 | Dashboard, Profiles, Routing, Connections, Logs, Settings and Node editor SHALL follow the per-view contract in [[wireframes/09-fit-layouts]] §10. |
| FR-006 | Tables SHALL use column templates with `minmax(0, …)` and ellipsis; horizontal scroll is allowed only inside a table's own region and only below 720 px content width. |
| FR-007 | Above 1400 px inner width, content SHALL be centered with `max-width: 1280px`; the rail stays at left. |
| FR-008 | Charts SHALL resize with their card (`ResizeObserver`) with no fixed pixel size. |
| FR-009 | Layout SHALL remain correct at OS text scaling 125 % and 150 % and with browser zoom 90 – 125 %. |

## 4. Acceptance criteria

| ID | Given / When / Then | Covers |
| :--- | :--- | :--- |
| AC-001 | For each size in the test matrix ([[wireframes/00-window-and-grid]] §7) and each of the 6 views: `document.documentElement.scrollHeight <= innerHeight` **and** `scrollWidth <= innerWidth` | FR-001, FR-003 |
| AC-002 | `grep -rnE "[0-9]+v[hw]\b" src/` returns no layout usages (only allowed in the splash) | FR-004 |
| AC-003 | `grep -rnE "@media" src/ --include=*.css --include=*.tsx` matches only `tokens.css` and `prefers-*` queries | FR-002 |
| AC-004 | In every view, the number of elements with computed `overflow-y: auto\|scroll` equals the view's contract (Dashboard 1, Profiles 2, Routing 1, Connections 1, Logs 1, Settings 1) | FR-003, FR-005 |
| AC-005 | At 720×520 every primary action of every view is reachable without scrolling the *page* (may be inside its own scroll region) | FR-005 |
| AC-006 | Resize 720 → 1920 → 720 continuously: no horizontal jitter, no content clipped, no layout jump at 840 | FR-002, FR-007 |
| AC-007 | Long node tag (80 chars) and long host (200 chars) never widen a card; both show ellipsis + `title` | FR-006 |
| AC-008 | OS scaling 150 % at 1000×680: AC-001 still passes | FR-009 |
| AC-009 | Visual regression: Playwright screenshots of 6 views × 3 sizes × 2 themes are within the agreed pixel-diff threshold after intentional updates | all |

## 5. Decisions

| ID | Decision | Alternatives | Why |
| :--- | :--- | :--- | :--- |
| D-001 | Size classes come from `data-size`, not from per-component media queries | Container queries everywhere | One mechanism, debuggable in DevTools, works on older WebKitGTK. Container queries allowed only as an enhancement inside a card |
| D-002 | Two independent scrollers in Profiles | One scrolling page | Import/list and server table are separate tasks; each must keep its own position |
| D-003 | Dense mode keyed on height ≤ 600 | Only width-based | 720×520 is *short*, not just narrow |

## 6. Open questions
- **OQ-1** Should Profiles' import card be collapsed by default in Expanded too? (Wireframe assumes expanded in Expanded, collapsed in Medium.)
- **OQ-2** Is virtualization already in the Dashboard server list, or only Logs/Connections? (Affects T-005.)

## 7. Docs to update
[[wireframes/README]] · each of [[wireframes/02-dashboard]]…[[wireframes/08-settings]] (add "Target" callout, done) · [[02-Frontend-UI-Architecture]] · [[99-Vault-Changelog]]
