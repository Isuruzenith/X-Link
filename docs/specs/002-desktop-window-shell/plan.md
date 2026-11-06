---
title: "002 — Desktop Window & Shell — Plan"
tags:
  - spec
  - sdd
---

# 002 — Plan

**Navigation**: [[specs/002-desktop-window-shell/spec]] | [[specs/002-desktop-window-shell/tasks]]

## 1. Current state
Window config reported as `1080×700` (architecture notes) and `950×650, min 380×550` (current wireframes). Navigation uses `220px` (notes 02/03) vs `212px` (wireframe 01). macOS inset `72px`. These are *documentation* values; T-001 reads the real file first.

## 2. Design

### 2.1 `src-tauri/tauri.conf.json`
```json
"app": {
  "windows": [{
    "label": "main", "title": "X-Link",
    "width": 1000, "height": 680, "minWidth": 720, "minHeight": 520,
    "resizable": true, "center": true, "visible": false,
    "titleBarStyle": "Overlay", "hiddenTitle": true
  }]
}
```
`titleBarStyle`/`hiddenTitle` apply on macOS only; keep as today. Platform overrides if needed: `tauri.macos.conf.json`.

### 2.2 Rust clamp (in the `setup` hook, before the first `show`)
```rust
use tauri::{LogicalSize, Manager};

fn fit_main_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    let monitor = win.current_monitor().ok().flatten()
        .or_else(|| win.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let s: LogicalSize<f64> = m.size().to_logical(m.scale_factor());
        let w = (s.width  * 0.92).clamp(720.0, 1000.0);
        let h = (s.height * 0.88).clamp(520.0, 680.0);
        let _ = win.set_size(LogicalSize::new(w, h));
        let _ = win.center();
    }
}
```
Exact method names follow Tauri 2; confirm against the pinned version (OQ-2). Call it once; do **not** call it on every `show_window` (that would fight the user's manual resize).

### 2.3 Frontend: size-class hook (`src/hooks/useWindowClass.ts`)
```ts
export function useWindowClass() {
  useEffect(() => {
    const root = document.documentElement;
    const apply = (w: number, h: number) => {
      root.dataset.size = w < 600 ? 'compact' : w < 840 ? 'medium' : 'expanded';
      root.dataset.dense = String(h <= 600);
    };
    const ro = new ResizeObserver(([e]) => apply(e.contentRect.width, e.contentRect.height));
    ro.observe(root);
    apply(root.clientWidth, root.clientHeight);
    return () => ro.disconnect();
  }, []);
}
```
Call once in `App.tsx`. Tokens react to attributes:
```css
:root                         { --xl-rail-w: 220px; --xl-page-pad: 16px; --xl-gap: 16px; --xl-header-h: 64px; }
:root[data-size="medium"]     { --xl-rail-w: 80px;  --xl-page-pad: 12px; --xl-gap: 12px; }
:root[data-size="compact"]    { --xl-rail-w: 0px;   --xl-page-pad: 12px; --xl-gap: 12px; }
:root[data-dense="true"]      { --xl-header-h: 48px; --xl-card-pad: 12px; }
```

### 2.4 macOS safe area
`<div className="mac-drag" data-tauri-drag-region />` of `height: 40px` rendered only when `isMac`, at the top of the rail and of the view host. Windows/Linux: not rendered. Replace `pt-[72px]` usages after reading source (OQ-3).

### 2.5 Show-after-ready
Existing flow: hydrate → `invoke('is_minimized_launch')` → `invoke('show_window')`. Add `await whenIconsReady()` and one `requestAnimationFrame` before `show_window` so the first visible frame is final.

### 2.6 Overlays
Side sheet: `width: min(500px, 100%)`, `inset-block: 0; inset-inline-end: 0`. Toasts: `max-width: min(360px, calc(100% - 24px))`, bottom-right with `env(safe-area-inset-*)`. Dialogs: `md-dialog` with `max-height: calc(100% - 48px)` and an internal scroll body.

## 3. Files touched
`src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs` (setup hook only), `src/hooks/useWindowClass.ts`, `src/App.tsx`, `src/styles/tokens.css`, `src/components/NavRail.tsx`, `src/components/ViewShell.tsx`, `src/components/ToastContainer.tsx`, node-editor sheet CSS.

## 4. Risks
| Risk | Mitigation |
| :--- | :--- |
| Rust edit touches the sidecar/startup path (Playbook invariants 4–5) | Change is confined to window sizing in `setup`; no process or IPC code; review checklist item |
| `current_monitor()` is `None` while hidden on some OS | fallback to `primary_monitor()`; if both `None`, keep config size |
| Clamp fights user resize | applied once per process start |
| `ResizeObserver` loop warnings | observe `documentElement`, set only data attributes (no layout-affecting writes in the callback) |

## 5. Rollout / rollback
Two commits: (a) config + clamp, (b) hook + tokens. Rollback = revert commit; no persisted state.

## 6. Verification
AC-001/002 with display-scaling settings on a Windows VM; AC-004 with screen recording; AC-006 by dragging; AC-007 on a Mac.
