---
title: "NNN — <Feature name> — Tasks"
tags: [spec, sdd]
---

# NNN — Tasks

Rule: one task per commit. Tick only when the done-check passes.

- [ ] **T-001** <imperative title> — files: `…` — done-check: AC-00X (`command or manual step`)

## Agent prompt
```text
Read specs/NNN-…/spec.md, plan.md, tasks.md. Do only the first unchecked task. Stop after ticking it.
```
