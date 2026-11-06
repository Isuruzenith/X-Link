---
title: "003 — Responsive, Fit-to-Window Layout — Tasks"
tags:
  - spec
  - sdd
---

# 003 — Tasks

**Navigation**: [[specs/003-responsive-fit-layout/spec]] | [[specs/003-responsive-fit-layout/plan]]

- [ ] **T-001** Add Playwright, `e2e/fit.spec.ts` and a `?fixtures=1` seed; run it to record the **baseline failures** — done-check: failing list attached to PR
- [ ] **T-002** Shell CSS (`layout.css`) + tokens; root overflow hidden; rail via `--xl-rail-w` — done-check: AC-001 for shell only
- [ ] **T-003** `.scroll-region` + `.xl-card` primitives; convert **Logs** — done-check: AC-001/004 Logs
- [ ] **T-004** Convert **Connections** (toolbar row + virtual list fills height) — done-check: AC-001/004/007 Connections
- [ ] **T-005** Convert **Routing** (policy/rule-set row, rules table scroller) — done-check: AC-001/004 Routing
- [ ] **T-006** Convert **Settings** (180 px secondary nav ↔ `md-tabs`) — done-check: AC-001/004 Settings
- [ ] **T-007** Convert **Profiles** (two scrollers, collapsible import) — done-check: AC-001/004/005 Profiles
- [ ] **T-008** Convert **Dashboard** (hero 120/100 px, tabbed Medium card, chart fills) — done-check: AC-001/004/005 Dashboard
- [ ] **T-009** Node-editor side sheet + dialogs sizing — done-check: AC-005 + [[specs/002-desktop-window-shell/spec]] AC-008
- [ ] **T-010** Compact drawer (browser preview) — done-check: 599×800 passes AC-001
- [ ] **T-011** Delete dead CSS (`utility-view`, `settings-layout`, `routing-layout`, `profiles-grid`, `vh` caps); add CI greps for AC-002/AC-003 — done-check: both greps clean
- [ ] **T-012** Visual regression baseline (6 views × 3 sizes × 2 themes) — done-check: AC-009
- [ ] **T-013** OS-scaling pass (125 %/150 %) and zoom 90–125 % — done-check: AC-008
- [ ] **T-014** Update vault notes; regenerate wireframes if layout deviated (`tools/gen_wireframes.py`) — done-check: links resolve

## Agent prompt
```text
Read docs/specs/003-responsive-fit-layout/{spec,plan,tasks}.md, docs/wireframes/00-window-and-grid.md and docs/wireframes/09-fit-layouts.md.
Do only the first unchecked task. Change markup/CSS only; no store, IPC or Rust changes.
After the change run: npx playwright test e2e/fit.spec.ts and paste the summary.
```
