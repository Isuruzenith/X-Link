---
title: "001 — Self-Hosted Icons — Tasks"
tags:
  - spec
  - sdd
---

# 001 — Tasks

**Navigation**: [[specs/001-self-hosted-icons/spec]] | [[specs/001-self-hosted-icons/plan]]

- [ ] **T-001** Diagnose on a `tauri build --debug` build: run the H1–H6 checks, record which fail in the PR description — done-check: OQ-1 answered
- [ ] **T-002** Add `@fontsource-variable/material-symbols-rounded`; import in `main.tsx`; delete any Google Fonts `<link>`/`@import` — files: `package.json`, `index.html`, `src/main.tsx` — done-check: Network tab shows `.woff2` from app origin (AC-006)
- [ ] **T-003** Set `--md-icon-font` to the exact family string; add `src/styles/icons.css` — done-check: `document.fonts.check(...)` true (AC-006)
- [ ] **T-004** Add `utils/fonts.ts` `whenIconsReady()` and the hidden-until-ready CSS — done-check: AC-003
- [ ] **T-005** Create `icon-names.ts` (allow-list) and refactor `Icon` wrapper; replace every raw `<md-icon>` in `src/` — done-check: `grep -rn "<md-icon" src/` only matches `icon.tsx`; `tsc` passes (AC-007)
- [ ] **T-006** Set CSP in `tauri.conf.json` — done-check: AC-005
- [ ] **T-007** Add `scripts/check-offline.ts` + `npm run check:offline` and wire into `build` — done-check: AC-004
- [ ] **T-008** Bundle flag SVGs, replace `flagcdn.com` — done-check: AC-008
- [ ] **T-009** Run clean-VM tests (Win/macOS/Linux), offline — done-check: AC-001, AC-002, AC-009
- [ ] **T-010** Update vault notes listed in the spec; mark ICON-001 resolved in [[00-Home-MOC]] — done-check: links resolve

## Agent prompt
```text
Read docs/specs/001-self-hosted-icons/{spec,plan,tasks}.md and docs/09-Design-System.md §6.
Do only the first unchecked task. Do not touch src-tauri/src/** (Rust) or IPC contracts.
```
