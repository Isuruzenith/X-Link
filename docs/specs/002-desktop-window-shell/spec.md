---
title: "002 — Desktop Window & Shell — Spec"
aliases: ["Window Spec", "Fixed Window"]
tags:
  - spec
  - sdd
  - frontend
  - tauri
status: ready
created: 2026-11-06
---

# 002 — Desktop Window & Shell

**Navigation**: [[specs/README]] | **Plan**: [[specs/002-desktop-window-shell/plan]] | **Tasks**: [[specs/002-desktop-window-shell/tasks]] | **Geometry**: [[wireframes/00-window-and-grid]]

## 1. Problem

The app should feel like a proper **desktop window of a known size**, not a web page. Today the vault documents three different sizes (`1080×700`, `950×650`, min `380×550`), the minimum is phone-sized (so the desktop UI is forced to contort), the macOS top inset is `72px`, and nothing guarantees the first paint is at the right size.

## 2. Goals / Non-goals

- **Goal:** one default size, one minimum, applied identically on Windows, macOS, Linux.
- **Goal:** the window never opens larger than the user's screen (1080p @ 150 % case).
- **Goal:** no white flash / resize jump at startup.
- **Goal:** the shell (rail + view host + overlays) exposes `data-size` / `data-dense` so [[specs/003-responsive-fit-layout/spec]] can be pure CSS.
- **Non-goal:** custom-drawn title bar on Windows/Linux. **Non-goal:** remembering window size between runs (see D-003).

## 3. Functional requirements

| ID | Requirement |
| :--- | :--- |
| FR-001 | Default inner size SHALL be 1000×680 logical px; minimum 720×520. |
| FR-002 | On first show the window SHALL be clamped to ≤ 92 % of monitor width and ≤ 88 % of monitor height (never below the minimum) and centered. |
| FR-003 | The window SHALL be created hidden and shown by the existing `show_window` flow only after stores hydrate and the icon font is ready ([[specs/001-self-hosted-icons/spec]] FR-003). `--minimized` launch behavior is unchanged. |
| FR-004 | `resizable` SHALL be a single config switch; with `false` the app SHALL still satisfy all layout ACs at the default size. |
| FR-005 | The shell SHALL set `data-size="compact\|medium\|expanded"` and `data-dense="true\|false"` on `<html>` from the inner size, updating on resize with `ResizeObserver` (no `window.resize` polling). |
| FR-006 | On macOS, the top safe area SHALL be the minimum clearance for the traffic lights (target 40 px) and the strip SHALL be a drag region (`data-tauri-drag-region`). Windows/Linux SHALL have zero extra inset. |
| FR-007 | Rail width SHALL follow `--xl-rail-w` = 220 (Expanded) / 80 (Medium) / 0 + drawer (Compact). |
| FR-008 | Overlays (Node editor sheet, dialogs, toasts) SHALL be positioned against the window, respect the size class, and never extend outside the viewport. |

## 4. Acceptance criteria

| ID | Given / When / Then | Covers |
| :--- | :--- | :--- |
| AC-001 | Fresh install on a 1920×1080 @ 100 % monitor → window opens 1000×680, centered, fully on-screen | FR-001, FR-002 |
| AC-002 | Fresh install on 1920×1080 @ 150 % (1280×720 logical) → window height ≤ 88 % of 720 = 633 px, title bar and bottom edge both visible | FR-002 |
| AC-003 | Try to drag the window smaller than 720×520 → it stops at 720×520 | FR-001 |
| AC-004 | Cold start (record screen): no white frame, no visible resize, first visible frame already shows the final layout and icons | FR-003 |
| AC-005 | Set `resizable:false` → no resize handle/maximize; at 1000×680 every view passes the fit checks in [[specs/003-responsive-fit-layout/spec]] | FR-004 |
| AC-006 | Resize from 1000 → 800 → 1000: `<html data-size>` flips `expanded → medium → expanded`; rail animates 220 ↔ 80 px without layout jump | FR-005, FR-007 |
| AC-007 | macOS: traffic lights never overlap the brand block or the view title; dragging the top strip moves the window | FR-006 |
| AC-008 | Open the Node editor at 720×520: sheet is 500 px wide, header and footer pinned, nothing clipped | FR-008 |
| AC-009 | `--minimized` launch + tray "Show" opens the window at the clamped size, not 0×0 | FR-003 |

## 5. Decisions

| ID | Decision | Alternatives | Why |
| :--- | :--- | :--- | :--- |
| D-001 | Default 1000×680, min 720×520 | 950×650 / 1080×700 | 1000 is Expanded with margin; 720 is the smallest width where Medium works; 520 height is the dense-mode floor |
| D-002 | **Constrained resizable** by default, hard-fixed via one flag | Always hard-fixed | Hard-fixed windows are unusable on small screens and block accessibility zoom. If a truly fixed window is required, flip `resizable` (FR-004) — the layout already supports it |
| D-003 | Do not persist window size | `tauri-plugin-window-state` | The brief is a *fixed*, predictable window. Revisit if users ask |
| D-004 | Compute clamp in Rust `setup` | CSS only | The OS creates the window before CSS exists; Rust is the only place that can resize before first paint |

## 6. Open questions

- **OQ-1** Does the current build use `visible:false`? (Playbook mentions `show_window`; confirm in `tauri.conf.json`.)
- **OQ-2** Is `Monitor::work_area()` available in the pinned Tauri 2.x? If yes use it instead of the 92 %/88 % heuristic.
- **OQ-3** Why `72px` on macOS? If it hides a real requirement (e.g. rail brand offset), document it; otherwise reduce to 40 px.

## 7. Docs to update
[[00-Home-MOC]] (window values) · [[02-Frontend-UI-Architecture]] · [[wireframes/01-shell-navigation]] · [[04-UI-Wireframes]] banner · [[99-Vault-Changelog]]
