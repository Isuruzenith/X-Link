---
title: "004 — Material Web UI Conformance — Spec"
aliases: ["Material UI Spec", "Component Migration"]
tags:
  - spec
  - sdd
  - frontend
  - design-system
status: ready
created: 2026-11-06
---

# 004 — Material Web UI Conformance

**Navigation**: [[specs/README]] | **Plan**: [[specs/004-material-web-ui-conformance/plan]] | **Tasks**: [[specs/004-material-web-ui-conformance/tasks]] | **Design system**: [[09-Design-System]]

## 1. Problem

The UI is a blend: Material Web (`md-*`), shadcn/Radix/Base UI primitives (`Button`, `Switch`, `Select`, `Badge`, `Sheet`…), and Tailwind color utilities (`text-green-600 dark:text-green-400`, `bg-secondary`). The result is inconsistent focus rings, radii, density and theming, and two places that define colors. The goal is a UI that is recognizably **Material 3** end to end.

## 2. Goals / Non-goals

- **Goal:** every interactive control is a Material Web element (through local wrappers) or a documented custom component from [[09-Design-System]] §5.
- **Goal:** all colors/shapes/type from tokens; light/dark theme from one scheme definition.
- **Goal:** public wrapper API identical to today's primitives so views change by import path only.
- **Non-goal:** new screens, new copy, new behavior. **Non-goal:** replacing Tailwind for layout.

## 3. Functional requirements

| ID | Requirement |
| :--- | :--- |
| FR-001 | A generated M3 color scheme (light + dark) SHALL define all `--md-sys-color-*` tokens; `--xl-*` aliases per [[09-Design-System]] §2.1 SHALL be the only tokens used in app CSS. |
| FR-002 | Local wrappers in `src/components/md/` SHALL exist for: Button, IconButton, Switch, TextField, Select, Tabs, SegmentedControl, Chip, Dialog, Menu, Progress, List. |
| FR-003 | Existing component names (`Button`, `FormInput`, `FormSelect`, `FormSwitchRow`, `SegmentedControl`, `Badge`, `Sheet`, `Table`) SHALL keep their props; internals switch to wrappers. |
| FR-004 | Only used Material Web modules SHALL be imported (no `all.js`). |
| FR-005 | `NavRail`, cards, tables, toasts and the side sheet SHALL be custom but use Material state layers (`md-ripple`, `md-focus-ring`, `md-elevation`) and tokens. |
| FR-006 | Connection state SHALL be expressed with `--xl-ok/-warn/-err` plus text; latency bands use the same tokens. |
| FR-007 | Tailwind color, radius and shadow utilities SHALL be removed from `src/` (layout utilities allowed). |
| FR-008 | Typography SHALL use `md-typescale-*` roles; fonts self-hosted ([[specs/001-self-hosted-icons/spec]]). |
| FR-009 | Both themes SHALL meet WCAG AA contrast; `prefers-reduced-motion` and `forced-colors` SHALL be honored. |

## 4. Acceptance criteria

| ID | Given / When / Then | Covers |
| :--- | :--- | :--- |
| AC-001 | `grep -rnE "(text|bg|border)-(gray|green|amber|red|slate|zinc)-[0-9]+" src/` → no matches | FR-007 |
| AC-002 | `grep -rnE "#[0-9a-fA-F]{3,8}\b" src/ --include=*.tsx --include=*.css` → matches only inside `tokens.css` and chart gradient definitions that reference tokens | FR-001 |
| AC-003 | `grep -rn "@material/web/all" src/` → none; bundle analyzer shows only used components | FR-004 |
| AC-004 | Tab through the whole app (all 6 views + dialogs): every focusable control shows the same Material focus ring; nothing is unreachable | FR-005, FR-009 |
| AC-005 | Axe / Lighthouse a11y run on each view in both themes → no contrast failures | FR-009 |
| AC-006 | Switching theme repaints every surface; no component stays in the old theme (check cards, dialog, side sheet, toasts, chart) | FR-001 |
| AC-007 | Connect → Connected: button, rail indicator and header pill change state together and each has text, not color only | FR-006 |
| AC-008 | `FormSwitchRow` toggling, `FormSelect` choosing, `SegmentedControl` filtering: existing unit/e2e tests pass unchanged | FR-003 |
| AC-009 | Production JS+CSS grows by ≤ 150 KB gzip vs. before the migration | FR-004 |

## 5. Decisions

| ID | Decision | Alternatives | Why |
| :--- | :--- | :--- | :--- |
| D-001 | Wrapper layer `src/components/md/*` | Use `<md-*>` directly in views | Library is in maintenance mode; wrappers localize any future swap and fix React typing/events once |
| D-002 | Thin React wrappers using `ref` + `addEventListener` for `change`/`input`/`closed` | Rely on React's custom-element event props | Version-independent and explicit |
| D-003 | Keep Base UI/Radix only where Material Web has no equivalent behavior (portal/focus for the side sheet) — styling stripped | Remove entirely | Avoid re-implementing focus trapping |
| D-004 | Theme generated with Material Theme Builder / `@material/material-color-utilities` from one seed, committed as `tokens.css` | Hand-tune | Reproducible, AA-safe baselines |

## 6. Open questions
- **OQ-1** Seed color: keep the current brand primary? (Extract from `--md-sys-color-primary` in the current `material-theme.css`.)
- **OQ-2** Is `labs/segmentedbuttonset` acceptable, or use `md-chip-set` with filter chips for the log-level filter?

## 7. Docs to update
[[09-Design-System]] · [[02-Frontend-UI-Architecture]] §1 · [[03-UI-Components-and-Views]] §3.2 · [[99-Vault-Changelog]]
