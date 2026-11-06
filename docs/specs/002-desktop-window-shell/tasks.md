---
title: "002 — Desktop Window & Shell — Tasks"
tags:
  - spec
  - sdd
---

# 002 — Tasks

**Navigation**: [[specs/002-desktop-window-shell/spec]] | [[specs/002-desktop-window-shell/plan]]

- [ ] **T-001** Read the real `tauri.conf.json`, `NavRail.tsx`, `ViewShell.tsx`; fill OQ-1 and OQ-3 answers into the spec — done-check: spec open questions resolved
- [ ] **T-002** Update window block in `tauri.conf.json` (§2.1) — done-check: AC-003
- [ ] **T-003** Add `fit_main_window` in the `setup` hook; call before first show — done-check: AC-001, AC-002
- [ ] **T-004** Add `useWindowClass` + token attributes; call from `App.tsx` — done-check: AC-006
- [ ] **T-005** Rail: 220 / 80 / drawer by size class (CSS only, no JS width logic) — done-check: AC-006
- [ ] **T-006** macOS safe-area strip + drag region; remove `pt-[72px]` — done-check: AC-007
- [ ] **T-007** Show-after-ready sequence (`whenIconsReady` + rAF) — done-check: AC-004, AC-009
- [ ] **T-008** Overlay sizing rules (sheet, toast, dialog) — done-check: AC-008
- [ ] **T-009** Flip `resizable:false` locally and run the 003 fit checks at 1000×680 — done-check: AC-005
- [ ] **T-010** Update vault notes listed in the spec — done-check: links resolve, no `1080×700` / `950×650` left (`grep -rn "1080" docs/`)

## Agent prompt
```text
Read docs/specs/002-desktop-window-shell/{spec,plan,tasks}.md and docs/wireframes/00-window-and-grid.md.
Do only the first unchecked task. Rust changes are limited to window sizing in the setup hook.
```
