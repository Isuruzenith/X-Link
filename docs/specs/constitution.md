---
title: "Constitution — Rules That Every Spec Must Obey"
aliases: ["Constitution", "Project Principles"]
tags:
  - spec
  - sdd
  - architecture
created: 2026-11-06
updated: 2026-11-06
---

# Constitution

**Navigation**: [[specs/README]] | [[07-AI-Agent-Playbook]] | [[09-Design-System]]

A spec that violates an article below is rejected until it either complies or amends this note.

| # | Article | Meaning in practice |
| :--- | :--- | :--- |
| I | **The window is the canvas** | The document never scrolls. Content fits the window at every supported size; only named scroll regions scroll. ([[wireframes/00-window-and-grid]] §4) |
| II | **Offline by default** | The shipped app loads **no** fonts, icons, scripts, styles or images from the network. User-initiated features (subscriptions, geo lookup, flags if opted in) are the only exceptions and must degrade gracefully. |
| III | **Tokens, not values** | Colors, radii, spacing, type come from `--md-sys-*` / `--xl-*`. No raw hex, no magic pixel widths in components. |
| IV | **Material Web first** | Use an `md-*` element when one exists; build our own only for what the library lacks (rail, cards, tables, toasts, side sheet). ([[09-Design-System]] §5) |
| V | **UI work never touches contracts** | Rust commands, events, JSON schemas and the **7 invariants** in [[07-AI-Agent-Playbook]] are out of scope for UI features. If a UI need requires a contract change, it becomes its own spec. |
| VI | **Accessible or not done** | Keyboard path, focus ring, ≥ 4.5:1 contrast, status never color-only. ([[09-Design-System]] §7) |
| VII | **Testable acceptance** | Every requirement has a pass/fail check someone else can run in under five minutes. |
| VIII | **The vault is part of the product** | A change that makes a note stale updates that note in the same PR. |
| IX | **Small steps** | One task, one commit, one reviewable diff. |
| X | **Truth over docs** | When a note disagrees with the source, the source wins and the note is fixed ([[99-Vault-Changelog]] lists known drift). |
