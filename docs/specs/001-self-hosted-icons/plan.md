---
title: "001 — Self-Hosted Icons — Plan"
tags:
  - spec
  - sdd
---

# 001 — Plan

**Navigation**: [[specs/001-self-hosted-icons/spec]] | [[specs/001-self-hosted-icons/tasks]]

## 1. Root-cause analysis

The source is not in the vault, so the plan lists hypotheses ranked by likelihood and a check that settles each in minutes.

| # | Hypothesis | Likelihood | Quick check on the **built** app |
| :--- | :--- | :--- | :--- |
| H1 | Font is loaded from Google Fonts (`<link>` in `index.html` or `@import` in CSS). Blocked by CSP, offline, or firewalled in the packaged app | **High** | `grep -rn "fonts.googleapis" index.html src/`; DevTools → Console shows CSP / `ERR_` for the font; Network tab shows red request |
| H2 | Font *is* bundled but `--md-icon-font` names a different family. Material Web defaults to `Material Symbols Outlined`; the app ships *Rounded* (or the Fontsource name has a `Variable` suffix) | **High** | DevTools → Elements → `md-icon` → Computed `font-family`; compare with the `@font-face` family in loaded CSS |
| H3 | CSP in `tauri.conf.json` has `default-src 'self'` without `font-src …`/`data:` and the font is inlined as a data URI by Vite (small-asset inlining) | Medium | Console: `Refused to load the font 'data:…'` |
| H4 | `.woff2` URL wrong in production (`base` path, relative `url()` in a CSS file moved by Tailwind) → 404 | Medium | Network tab 404 on `*.woff2` |
| H5 | Dev machine has Material Symbols installed system-wide, masking the problem; fails everywhere else | Low–Medium | Test on a clean VM (AC-001) |
| H6 | Font present, but file is an *Outlined static* subset missing icons used (new names) | Low | Only some icons blank; check name against allow-list |

How to open DevTools in a release build: build with `tauri build --debug`, or enable the `devtools` Cargo feature of `tauri` temporarily, then right-click → Inspect.

## 2. Design

### 2.1 Dependencies
```bash
npm i @fontsource-variable/material-symbols-rounded
npm i -D tsx   # for the check script; skip if already present
```

### 2.2 `src/main.tsx` (first imports)
```ts
import '@fontsource-variable/material-symbols-rounded';   // check package README for the axis variant you need
import './styles/tokens.css';
import { whenIconsReady } from '@/utils/fonts';
whenIconsReady();                                          // adds .icons-ready to <html>
```

### 2.3 `src/utils/fonts.ts`
```ts
export const ICON_FAMILY = 'Material Symbols Rounded Variable'; // MUST equal @font-face family in the package CSS

export async function whenIconsReady(timeoutMs = 1500) {
  const root = document.documentElement;
  try {
    await Promise.race([
      document.fonts.load(`24px "${ICON_FAMILY}"`, 'home'),
      new Promise((r) => setTimeout(r, timeoutMs)),
    ]);
  } finally {
    root.classList.add('icons-ready');   // never block the app on a font
  }
}
```

### 2.4 `src/styles/icons.css`
```css
:root { --md-icon-font: 'Material Symbols Rounded Variable'; --md-icon-size: 24px; }
md-icon { visibility: hidden; }
html.icons-ready md-icon { visibility: visible; }
md-icon, .xl-icon {
  font-variation-settings: 'FILL' 0, 'wght' 400, 'GRAD' 0, 'opsz' 24;   /* only effective axes the chosen CSS file includes */
  font-feature-settings: 'liga';
  -webkit-font-feature-settings: 'liga';
  user-select: none;
}
```

### 2.5 `src/components/ui/icon.tsx`
```tsx
import '@material/web/icon/icon.js';
import type { IconName } from './icon-names';

export function Icon({ name, size = 24, label }: { name: IconName; size?: 16 | 18 | 20 | 24 | 32; label?: string }) {
  return (
    <md-icon style={{ '--md-icon-size': `${size}px` } as React.CSSProperties}
             aria-hidden={label ? undefined : true} aria-label={label}>{name}</md-icon>
  );
}
```
`icon-names.ts` exports `export type IconName = typeof ICONS[number]` from a `const ICONS = [...] as const` seeded from [[09-Design-System]] §6.3.

### 2.6 CSP (`src-tauri/tauri.conf.json` → `app.security.csp`)
```text
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
font-src 'self' data:; img-src 'self' data: blob: asset: http://asset.localhost;
connect-src 'self' ipc: http://ipc.localhost http://127.0.0.1:9090 https://freeipapi.com
```
Remove any `fonts.googleapis.com` / `fonts.gstatic.com` entries. Keep the Clash API origin only if the UI fetches it directly. Keep `freeipapi.com` only if the consent-gated geo call is made from the webview rather than Rust (per [[06-IPC-Bridge-and-Data-Contracts]] it is a Rust command, so it can likely be removed). **Re-test AC-005 after any CSP edit.**

### 2.7 Offline guard `scripts/check-offline.ts`
```ts
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
const BAD = /fonts\.googleapis\.com|fonts\.gstatic\.com|flagcdn\.com|cdn\.jsdelivr\.net|unpkg\.com|cdnjs\.cloudflare\.com/;
const walk = (d: string): string[] => readdirSync(d).flatMap(f => {
  const p = join(d, f); return statSync(p).isDirectory() ? walk(p) : [p];
});
const hits = walk('dist').filter(f => /\.(html|css|js|mjs)$/.test(f))
  .flatMap(f => (readFileSync(f, 'utf8').match(BAD) ? [f] : []));
if (hits.length) { console.error('Forbidden runtime hosts in:\n' + hits.join('\n')); process.exit(1); }
console.log('offline check OK');
```
`package.json`: `"check:offline": "tsx scripts/check-offline.ts"`, and `"build": "tsc -b && vite build && npm run check:offline"`.

### 2.8 Flags
Bundle the SVGs for the 18 codes the heuristic supports (`getCountryCode`) under `src/assets/flags/` and import via `import.meta.glob('../assets/flags/*.svg', { eager: true, as: 'url' })`. Replace `https://flagcdn.com/w20/{code}.png`. If flags are judged non-essential, replace with `<Icon name="public"/>` only (decision D-003 stays valid).

### 2.9 Plan B (only if H2/H3 fixed and icons still fail on WebKitGTK)
Generate an SVG sprite from the ~60 allow-listed names at build time; `Icon` renders `<svg><use href="#name"/></svg>`. Public API unchanged; delete the font dependency.

## 3. Files touched

| File | Change |
| :--- | :--- |
| `package.json` | add font dep, `check:offline`, build chain |
| `index.html` | delete Google Fonts `<link>`s |
| `src/main.tsx` | import font + `whenIconsReady()` |
| `src/utils/fonts.ts` | new |
| `src/styles/icons.css` | new; imported from `index.css` |
| `src/components/ui/icon.tsx`, `icon-names.ts` | typed wrapper |
| `src-tauri/tauri.conf.json` | CSP |
| `scripts/check-offline.ts` | new |
| `src/assets/flags/*.svg` + flag component | new |

## 4. Risks

| Risk | Mitigation |
| :--- | :--- |
| Full variable font is large | Choose the axis-limited CSS in the package (weight only); AC-009 caps growth; plan B if exceeded |
| Family string mismatch (silent) | AC-006 asserts `document.fonts.check`; `ICON_FAMILY` constant is imported by a unit test that parses the package CSS |
| CSP too strict breaks IPC | Keep `ipc:` and `http://ipc.localhost`; verify with AC-005 on Windows (uses the `http://ipc.localhost` form) |
| Ligature flash on slow disks | FR-003 gate |

## 5. Rollout / rollback
Single PR, behind no flag. Rollback = revert; no data or contract changes.

## 6. Verification plan
AC-001/002 on three OS images, offline. AC-003 with CPU throttling. AC-004 as CI step. AC-005/006 in DevTools of a `--debug` build.
