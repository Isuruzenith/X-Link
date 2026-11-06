---
title: "004 — Material Web UI Conformance — Plan"
tags:
  - spec
  - sdd
---

# 004 — Plan

**Navigation**: [[specs/004-material-web-ui-conformance/spec]] | [[specs/004-material-web-ui-conformance/tasks]]

## 1. Strategy
Strangler pattern: introduce wrappers beside the old primitives, switch one primitive at a time (**props stay the same**), delete the old file when `grep` shows zero imports. Views do not change except their import lines.

## 2. Design

### 2.1 Tokens
1. Pick seed (OQ-1) → generate scheme with Material Theme Builder → export CSS → `src/styles/md-scheme.css` (light under `:root`, dark under `:root[data-theme="dark"]`).
2. `src/styles/tokens.css` defines `--xl-*` aliases and status colors ([[09-Design-System]] §2).
3. Remove the second color system (`--background`, `--foreground`, `--card`, `--primary` shadcn variables) after the last Tailwind color utility is gone.

### 2.2 Wrapper template (`src/components/md/Switch.tsx`)
```tsx
import '@material/web/switch/switch.js';
import { useEffect, useRef } from 'react';

type Props = { selected: boolean; onChange: (v: boolean) => void; disabled?: boolean; 'aria-label'?: string };

export function MdSwitch({ selected, onChange, disabled, ...a11y }: Props) {
  const ref = useRef<HTMLElement & { selected: boolean }>(null);
  useEffect(() => { if (ref.current) ref.current.selected = selected; }, [selected]);
  useEffect(() => {
    const el = ref.current; if (!el) return;
    const h = () => onChange(el.selected);
    el.addEventListener('change', h);
    return () => el.removeEventListener('change', h);
  }, [onChange]);
  return <md-switch ref={ref as never} disabled={disabled} {...a11y} />;
}
```
Add JSX typings once in `src/types/material-web.d.ts` (`declare module 'react' { namespace JSX { interface IntrinsicElements { 'md-switch': any; … } } }`, or the typing helper the library docs describe for your React version).

### 2.3 Mapping old → new

| Old primitive (`components/ui/*`) | New | Notes |
| :--- | :--- | :--- |
| `Button` (variants default/outline/ghost/destructive) | `MdButton` variants → `filled`, `outlined`, `text`, `tonal`; destructive = filled with `--md-filled-button-container-color: var(--xl-err)` | keep `variant`/`size` props |
| `Switch` | `MdSwitch` | |
| `Select` / `FormSelect` | `MdSelect` (`md-outlined-select`) | options as children |
| `Input` / `FormInput` | `MdTextField` (`md-outlined-text-field`) | suffix → `slot="trailing-icon"` |
| `Badge` | `md-assist-chip` (read-only) or `.xl-badge` tokenised span | badges are not interactive; keep span if chip adds noise |
| `Sheet` | `.xl-sheet` on `<dialog>` + `md-elevation` | side-sheet behavior kept |
| `Table` | `.xl-table` tokenised grid | not in Material Web |
| `ScrollArea` | `.scroll-region` | |
| `Separator` | `md-divider` | |
| `SegmentedControl` | `md-outlined-segmented-button-set` (labs) or filter chips (OQ-2) | |
| Toast | `.xl-toast` tokenised | |

### 2.4 Status component
`<StatusPill state="connected|connecting|disconnected|error" label>` — single implementation used by `NavRail`, Dashboard header and Connect button, so AC-007 holds by construction.

### 2.5 Bundle budget check
`npx vite-bundle-visualizer` before/after; fail PR if AC-009 exceeded.

## 3. Files touched
`src/components/md/*` (new), `src/components/ui/*` (shrinks), `src/components/form/*`, `src/styles/{md-scheme,tokens}.css`, all `*View.tsx` imports, `ToastContainer`, `NavRail`, `TrafficChart` colors.

## 4. Risks
| Risk | Mitigation |
| :--- | :--- |
| Material Web maintenance mode | wrappers (D-001); pin version; no `labs` outside wrappers |
| React ↔ custom element prop/event mismatch | D-002 pattern + wrapper unit tests (render, toggle, disabled) |
| `md-select` menu clipped by `overflow:hidden` shell | menu renders in top layer via `popover`/`fixed`; verify inside each scroll region (AC-004) |
| Visual regression | 003's Playwright screenshots (T-012 there) re-baselined deliberately |

## 5. Rollout / rollback
Per-primitive PRs in the order of the tasks; each reverts independently.
