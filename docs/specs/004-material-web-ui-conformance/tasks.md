---
title: "004 — Material Web UI Conformance — Tasks"
tags:
  - spec
  - sdd
---

# 004 — Tasks

**Navigation**: [[specs/004-material-web-ui-conformance/spec]] | [[specs/004-material-web-ui-conformance/plan]]

- [ ] **T-001** Generate M3 scheme from seed; add `md-scheme.css`, `tokens.css`; theme toggle drives both — done-check: AC-006
- [ ] **T-002** Add JSX typings + `MdSwitch`; swap `Switch` internals — done-check: AC-008 (switch)
- [ ] **T-003** `MdButton` / `MdIconButton`; swap `Button`; destructive variant via token — done-check: AC-008 (button)
- [ ] **T-004** `MdTextField`, `MdSelect`; swap `FormInput`, `FormSelect` — done-check: AC-008 (form)
- [ ] **T-005** `SegmentedControl` (resolve OQ-2) — done-check: log/connection filters work, AC-008
- [ ] **T-006** `StatusPill` shared component in NavRail, Dashboard header, Connect button — done-check: AC-007
- [ ] **T-007** `md-dialog` for diff preview, confirm, passphrase; focus returns to trigger — done-check: AC-004
- [ ] **T-008** `.xl-sheet`, `.xl-toast`, `.xl-table`, `.xl-card` tokenised — done-check: AC-004, AC-006
- [ ] **T-009** Remove Tailwind color/radius/shadow utilities; chart colors from tokens — done-check: AC-001, AC-002
- [ ] **T-010** Delete unused `components/ui/*` and shadcn variables — done-check: `grep` for old imports empty
- [ ] **T-011** Typography roles and self-hosted fonts — done-check: no `font-family` literals outside `tokens.css`
- [ ] **T-012** A11y pass (axe, keyboard, forced-colors, reduced-motion) both themes — done-check: AC-004, AC-005
- [ ] **T-013** Bundle check — done-check: AC-009, AC-003
- [ ] **T-014** Update vault notes — done-check: links resolve

## Agent prompt
```text
Read docs/specs/004-material-web-ui-conformance/{spec,plan,tasks}.md and docs/09-Design-System.md.
Do only the first unchecked task. Keep each wrapper's props identical to the primitive it replaces.
```
