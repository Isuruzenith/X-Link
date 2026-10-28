# X-Link audit remediation specification

- **Status:** Draft for implementation
- **Baseline:** `main` at `45ea5e7`, audited 2026-10-27
- **Source:** [X_LINK_ENGINEERING_AUDIT.md](X_LINK_ENGINEERING_AUDIT.md)
- **Target:** The existing Tauri v2 / React application in this checkout. This is a specification, not a report that the work is complete.

## 1. Goal and scope

Resolve the **16 numbered findings** in the audit's findings matrix without regressing proxy connectivity, profile persistence, or the configuration invariants in [AI_HANDOVER.md](AI_HANDOVER.md). Implement in independently reviewable phases. A phase is complete only when its requirements, automated tests, and applicable manual checks have recorded outcomes; compiling alone is not acceptance.

The audit's proposed code snippets and effort estimates are **suggestions, not acceptance criteria**. In particular, the tray's empty event cannot by itself identify the newly selected node, the TUN fallback cannot be made session-only solely in React because Rust reloads settings from disk, and subscription URLs are currently **not fetched even on initial import**. The requirements below take precedence over those suggestions where they conflict with current code.

### Non-negotiable invariants

- Preserve the existing supported VLESS/VMess/Trojan/Shadowsocks import and connect workflows. Keep WebSocket TLS ALPN forced to `http/1.1`, FakeIP settings, LAN/DNS/server route exclusions, and the five-step connection recovery with TUN-to-system rollback; see [AI_HANDOVER.md](AI_HANDOVER.md).
- Never discard a working config as an unintended side effect of a failed update or mark a profile/connection updated when its native operation failed. Explicit deletion of the last profile is allowed only after a successful disconnect and the existing user confirmation. Stop/restart, import, refresh, and reload must have observable failure states and retain the last known working configuration where feasible.
- Do not widen the WebView CSP or transmit API secrets, subscription URLs, proxy credentials, or raw node details to logs, analytics, or unapproved third parties. Network requests must use bounded timeouts and sizes.
- Keep Windows-specific integrations under `cfg(target_os = "windows")`; keep Linux/macOS building and preserve their current behavior unless a requirement explicitly changes it. No live system-proxy/TUN changes as part of automated tests.
- Treat user-provided subscription content and node hosts as untrusted. Explicitly distinguish invalid input, validation failure, temporary network failure, and successful updates.

### Evidence and completion convention

Each ID below maps to the audit's numbered finding. Record each as **not started / in progress / verified / blocked**, with test output and manual-check notes in the implementing change. When a check needs an elevated Windows desktop, another installed process, or real network access, document it as **not run** until performed; do not substitute a unit test or an assumption for live validation. The audit's historical test results are not results for this implementation.

## 2. Phase A — Connectivity and state correctness

**Gate:** A profile cannot be deleted while its proxy remains active; tray changes reach the UI only after a successful switch; system DNS detection is independent of Windows display language.

### A1 · XL-BUG-001 — Safe active-profile deletion

**Current touchpoints:** `src/stores/profileStore.ts:410`, `src/stores/connectionStore.ts:72`, `src-tauri/src/commands/proxy.rs:666`, `src-tauri/src/commands/config.rs:417`.

- Disconnect an active connected/connecting profile through a shared, awaitable operation using Tauri's actual `toggle_proxy` contract (`start: false`, `selectedOutboundTag: null`). Do not reuse `toggleConnect()` unchanged: it sets UI state to disconnected before native confirmation and swallows stop failures.
- **On stop failure**, retain the profile file and metadata, show a visible error, and reconcile UI connection state with `get_proxy_status`. On success, confirm the native stop/system-proxy cleanup before deleting. A failure to remove the backend file must not silently remove the frontend metadata. Deleting a non-active profile must not interrupt the running active profile.
- Preserve consistent `activeProfileId`, `selectedProfileId`, active config, and node list after deletion, including deletion of the last profile and selection of another profile. Do not show a successful deletion while the active file or metadata still represents the deleted profile.
- **Tests:** mock IPC and store persistence for connected success, stop rejection, backend-delete rejection, non-active deletion, and last-profile deletion. Assert native-call order and no destructive writes after failed disconnect. An explicitly authorized desktop check verifies that no sidecar or OS proxy setting is left active after deletion.

### A2 · XL-STATE-001 — Tray-to-frontend node reconciliation

**Current touchpoints:** `src-tauri/src/tray.rs:335`, `src-tauri/src/commands/proxy.rs:575`, `src/App.tsx:149`, `src/stores/profileStore.ts:197`.

- Emit a node-switch event **only after** `switch_node_hot` succeeds. Include the effective profile ID and selected node tag, or expose a native query that returns the effective tag. On failure, keep the previous selection and surface/log a useful non-secret error. Do not infer success from an event with unit payload `()`.
- Make the frontend listener update the active profile's `selectedNodeTag`, persist its metadata, refresh nodes if necessary, and update Dashboard/Config selection without requiring tab remount or restart. `refreshNodes()` alone is insufficient: it prefers the previously saved tag over the selector default (`profileStore.ts:210`). Discard stale events for another profile; clean up listeners on unmount.
- **Tests:** successful tray switch updates native selection, metadata, and displayed tag; failed switch emits no success event; stale/duplicate event is harmless; listener unsubscribes. Manually switch nodes from the tray while the window is open and verify both views and the actual selected outbound agree.

### A3 · XL-NET-001 — Locale-independent Windows DNS detection

**Current touchpoints:** `src-tauri/src/config/mod.rs:221`, `src-tauri/Cargo.toml:37`.

- Replace English `ipconfig /all` scraping with Windows IP Helper adapter/DNS data or an equivalent locale-independent OS API. Select an operational, preferably physical/default-route adapter with a usable DNS IP; skip disconnected, loopback, and X-Link's own TUN adapter. Handle IPv4/IPv6, multiple DNS entries, and API failures without panics. Preserve explicit `dnsAddress` preference and the documented fallback when none is found.
- Keep `build_route_exclude_addresses()`'s private/loopback ranges, detected local DNS, and resolved proxy-server IPs. Detection failure must be observable in diagnostics; a public fallback address is **not** evidence that a local router was detected.
- **Tests:** pure adapter-selection cases for physical+virtual, disconnected, IPv6, missing DNS, and tied candidates; route-exclusion tests with a local DNS IP. Compile the Windows branch and other CI platforms. Manually check DNS selection and generated exclusions on a non-English Windows installation or a locale-independent adapter fixture; do not change host networking automatically.

## 3. Phase B — Security and lifecycle reliability

**Gate:** Starting/stopping X-Link cannot kill another owner's `sing-box`; mode fallback does not rewrite preferences; sensitive values never leave in an external dashboard URL; local-network bypass is complete.

### B1 · XL-STATE-002 — Session-only elevation fallback

**Current touchpoints:** `src/stores/connectionStore.ts:109`, `src/stores/settingsStore.ts:81`, `src-tauri/src/commands/proxy.rs:390,666`, `src-tauri/src/commands/proxy.rs:343`.

- If the user requests TUN without elevation, connect in system mode for that session **without changing** `settings.json` or the Settings preference. Carry the mode override through native configuration generation and any restart/reload in that session; clear it when the session ends. Preserve the existing TUN failure-recovery sequence for elevated connections.
- Expose/report the **effective runtime mode** separately from the saved preference. The current `get_proxy_status().tunEnabled` derives from cached settings and can misreport a native rollback (`proxy.rs:353`); update it and any UI mode indicator to describe the actual connection. After relaunch with elevation, TUN remains the selected preference.
- **Tests:** unelevated TUN connects in system mode with zero persistent writes; reconnect/reload stays in effective system mode; elevated retry uses saved TUN; native rollback reports system mode; failure leaves truthful status.

### B2 · XL-NET-002 — Complete private-address proxy bypass

**Current touchpoint:** `src-tauri/src/os/mod.rs:1`.

- Cover `10/8`, `192.168/16`, and **all** `172.16`–`172.31` /16 ranges plus existing localhost/loopback entries in the system-proxy bypass format the OS actually accepts. Do not paste CIDR notation into a WinINet wildcard list without demonstrating support. Leave sing-box's separate TUN CIDR exclusions intact.
- **Tests:** generated bypass entries include both boundary `172.16.*` and `172.31.*`, all intermediate prefixes, and original entries. Manually verify OS proxy bypass for representative private addresses on Windows; specify platform-specific behavior where applicable.

### B3 · XL-SEC-001 — Privacy-aware node geolocation

**Current touchpoints:** `src/stores/profileStore.ts:515`, `src-tauri/tauri.conf.json:26`, `src-tauri/src/lib.rs:27`.

- Remove WebView requests to `freeipapi.com`; keep CSP restrictive. If network geolocation is retained, expose a narrowly scoped native command using the existing Rust HTTP client, disabled by default until the user explicitly opts in to sharing **a node's public IP** with the named provider. Otherwise use local tag/flag inference. Neither passwords/hostnames nor private/loopback/link-local addresses may be sent to the provider.
- Restrict outbound requests to a fixed HTTPS provider endpoint, validate/encode the input, prevent redirects to arbitrary hosts, bound response size/time, and handle malformed or unavailable responses by clearing loading state and using the existing tag fallback. Do not log request URLs or credentials. Cache appropriately so switching views does not trigger repeat lookups.
- **Tests:** default-off, opt-in, private/hostname rejection or safe resolution, provider failure, malformed response, cache hit, and tag fallback with a local mock HTTP response; no WAN requirement. Manually verify absence of CSP violations and that consent controls the request.

### B4 · XL-SEC-002 — Dashboard without token in URL

**Current touchpoint:** `src/views/SettingsView.tsx:438`.

- Never interpolate `apiSecret` into any external URL, including an HTTPS URL. Use a secret-free HTTPS dashboard link or a locally hosted controller; if the dashboard needs a token, make entering/copying it a separate explicit user action with clear destination context. Do not send it in referrer, browser history, query string, or app logs.
- **Tests:** render with a sentinel secret; inspect link and click/open arguments and assert no sentinel appears. Manually verify navigation and controller usability when API is enabled and disabled.

### B5 · XL-SYS-001 — Ownership-scoped sidecar cleanup

**Current touchpoints:** `src-tauri/src/commands/proxy.rs:675,790`, `src-tauri/src/tray.rs:7`, `src-tauri/src/state.rs`.

- Remove image-name `taskkill /IM sing-box.exe` **and** non-Windows `pkill -f sing-box` from start/stop paths. Terminate only the `CommandChild` owned by X-Link; use graceful shutdown where available, then a bounded, owned-process kill fallback. If no child is tracked, do not guess ownership or kill unrelated processes; report a bind conflict when one exists. Keep status, active session, metrics, and OS proxy cleanup consistent on stop, exit, startup failure, and retry.
- **Tests:** injected/mock process handles ensure stop kills the tracked child only, start does not kill an untracked instance, repeated stop is safe, and failed/early exit cleans state. An authorized manual check runs another independent `sing-box` process and confirms it survives X-Link start/stop. Document how a genuinely orphaned X-Link child is handled without image-wide termination.

### B6 · XL-TEST-001 — Offline-deterministic DNS tests

**Current touchpoint:** `src-tauri/src/commands/latency.rs:208`.

- Make the default `cargo test` suite independent of Cloudflare/Google DoH. Test response parsing/selection/error cases with fixed local fixtures or mock HTTP; retain any real WAN smoke test only as an explicitly ignored, opt-in integration test.
- **Tests:** `cargo test --manifest-path src-tauri/Cargo.toml` passes in offline/restricted conditions; document the exact optional command for running WAN checks, without making it a CI gate.

## 4. Phase C — UI performance and accuracy

**Gate:** Logs remain usable at buffer capacity; theme colors render correctly; X-Link's own bound ports are not reported as external conflicts.

### C1 · XL-PERF-001 — Bounded log rendering

**Current touchpoints:** `src/views/LogsView.tsx:140`, `src/stores/logStore.ts:25`, `package.json:25`.

- Virtualize the log rows with the installed **react-window 2.x API** (or a justified compatible equivalent), and avoid re-running highlighting for off-screen rows. Account for wrapping/variable row height or constrain row layout deliberately. Preserve severity/search filters, counts, empty states, Copy All (all buffered logs, not only visible rows), Clear, scrollback, and auto-scroll semantics. A user scrolling up must not be forced to the bottom by new entries.
- **Tests:** with 2,000 entries, DOM log-row count stays bounded by viewport plus overscan; search/filter/copy/clear remain correct; adding logs while scrolled away preserves position; auto-scroll follows only when enabled or near bottom. Record an interactive responsiveness comparison on the same machine/build rather than assuming a performance gain.

### C2 · XL-UI-001 — Valid theme colors for Canvas

**Current touchpoints:** `src/components/TrafficChart.tsx:63`, `src/index.css:98`.

- Assign only browser-resolved color values to Canvas styles, not literal `var(--foreground)` tokens. Preserve distinct upload/download/grid strokes and redraw on theme changes; do not force full style calculation per traffic sample if colors have not changed.
- **Tests:** resolved strokes are valid colors in light and dark themes, no raw `var(` is passed to Canvas, and changing theme redraws with updated colors. Manually inspect the existing chart in both themes.

### C3 · XL-UI-002 — Accurate port conflict display

**Current touchpoints:** `src/App.tsx:193`, `src-tauri/src/commands/system.rs:59`.

- Differentiate X-Link's own currently bound mixed/HTTP/SOCKS ports from ports owned by other processes. A global `isConnected` skip is **not** sufficient if another configured port can still be externally occupied. While disconnected, detect real conflicts before starting; when ownership cannot be determined, show an unknown/unchecked state rather than silently claiming the port is free.
- **Tests:** disconnected/free, disconnected/external conflict, connected/own listener, connected/external conflict, and connection-transition cases. Manually compare the Settings indication against a separate port-owning process.

## 5. Phase D — Subscription and maintenance

**Gate:** URL profiles can be imported/refreshed safely; connected configuration changes have a documented apply policy; tests and release metadata reflect real functionality.

### D1 · XL-FEAT-001 — Fetch and refresh subscriptions

**Current touchpoints:** `src/stores/profileStore.ts:355,375`, `src/views/ConfigView.tsx`, `src-tauri/src/commands/config.rs:93`.

- A pasted URL is currently passed as raw bytes to `adapt()`; no HTTP fetch occurs. Implement the **initial fetch as well as Refresh**. Fetch on the native side with a bounded HTTPS client, no redirects, and response-size/time limits. Never log full subscription URLs (they may carry tokens). Support the already accepted raw URI list, base64, Clash YAML, and sing-box JSON formats; reject empty/unsupported data.
- Accept **HTTPS URLs with a public destination only** for both initial import and refresh; reject HTTP, loopback, link-local, and private destinations by default. Do not follow redirects. Validate the actual address connected to, not only an earlier DNS lookup, so DNS rebinding cannot bypass the destination check. Return a clear error for unsupported URLs rather than treating a URL as raw subscription bytes.
- Refresh the selected subscription **in place**: retain profile ID, name, URL and import origin; update nodes, count, and `lastUpdated` only after fetch, parse, config generation, and real sing-box validation succeed. Preserve the selected node tag if it still exists; otherwise select a valid fallback visibly. Refreshing a non-active profile must not overwrite `active.json` or interrupt the active connection. For an active connected profile, stage the candidate and perform the **validated cold restart** specified in D2 (not an assumed hot reload). Commit the replacement profile/metadata only when the running proxy uses it; on restart failure restore the previous config and attempt to restore the previous connection. If rollback also fails, report disconnected status and preserve the old profile/metadata rather than claiming refresh succeeded.
- **Tests:** initial HTTPS import, valid changed response, unchanged tags, removed selected tag, invalid/empty response, timeout, invalid generated config, refresh of a non-active profile, and failure while connected. Assert previous disk config and profile metadata remain usable on every failure. The UI button is shown only for subscription profiles, prevents double clicks, and reports success/failure.

### D2 · XL-ARCH-001 — Apply connected configuration changes safely

**Current touchpoints:** `src-tauri/src/commands/change_classifier.rs:1`, `src-tauri/src/commands/proxy.rs:630`, `src/views/RoutingView.tsx`, `src/stores/settingsStore.ts`.

- Apply in-scope settings and routing edits while connected through a **validated, controlled cold restart**; while disconnected, save and apply on the next start. Do not wire `classify_change`'s current `HotReload` path for DNS, outbound, or route changes without separate proof of sing-box API semantics. Remove or revise the dead classifier so its documented behavior does not mislead a future caller. Settings that cannot safely take effect until the next connection must be visibly labeled as deferred, not reported as applied.
- Validate a candidate config before stopping the current process. Keep the prior working config until the new process passes startup checks; on failure restore the prior config and attempt to restore its connection. If recovery fails, report disconnected status and an actionable error. Avoid routing rules that reference missing rule sets without an actionable error. Preserve active node and effective runtime mode across restarts.
- **Tests:** restart-versus-defer policy cases (ports/TUN versus route/DNS/node), successful connected apply, validation/restart/rollback failure, and disconnected edit. An authorized desktop test changes a routing rule and verifies the **running** engine's behavior rather than only `active.json` contents.

### D3 · XL-BUILD-001 — Version and YAML dependency hygiene

**Current touchpoints:** `src-tauri/Cargo.toml:3,24`, `src-tauri/src/config/adapters/clash.rs:132`, `release-please-config.json`.

- Align the native package version with the current app/release version (`0.8.0` at this baseline) and ensure the release automation updates it on later releases; the current `extra-files` list updates `tauri.conf.json` but not Cargo. Replace unmaintained `serde_yaml` with a maintained, compatible YAML parser after reviewing its API/security support; update `Cargo.lock` and preserve Clash import behavior and errors. If no compatible maintained parser can pass the fixtures and multi-platform build, mark **the dependency-hygiene part of D3 blocked** with evidence instead of claiming the requirement complete or forcing an unsafe upgrade.
- **Tests:** version equality across `package.json`, `tauri.conf.json`, `.release-please-manifest.json`, and `Cargo.toml`; existing Clash fixtures plus malformed YAML cases; multi-platform compilation. Verify the release configuration updates the native version in a test/dry-run or documented review.

### D4 · XL-TEST-002 — Replace meaningless smoke test

**Current touchpoint:** `src-tauri/src/commands/system.rs:412`.

- Replace `assert_eq!(2 + 2, 4)` with a deterministic test of version-output parsing or command-result handling. If actual binary execution is valuable, keep that as a separate optional integration test requiring a sidecar; default CI must not depend on installation or WAN access.
- **Tests:** valid/malformed/missing version output and process failure are covered; no tautological smoke assertion remains.

## 6. Cross-cutting test and delivery plan

1. **Before code changes:** capture a fresh baseline with `git status --short`, `npm run lint`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, and `cargo test --manifest-path src-tauri/Cargo.toml`. Record failures as baseline failures; do not claim the audit's old results as newly verified. Avoid launching live TUN/system-proxy operations for baseline checks.
2. **Testability:** `package.json` currently has no frontend test script/framework. Add a small frontend unit/component test harness appropriate for React 19/Vite 8; mock Tauri IPC, events, persistence, and timers. Put core policy in testable functions rather than testing only rendered markup. Rust tests must not depend on public WAN or administrator privileges. Extend CI (`.github/workflows/ci.yml`) to run the frontend tests once present.
3. **Sequence:** implement Phase A first, then Phase B, C, D. Split changes by requirement or cohesive dependency; do not mix a format parser migration into proxy lifecycle changes. Run targeted tests after each requirement and the full local lint/build/Rust suite at each phase gate. Validate on Windows and retain CI's Linux/macOS matrix.
4. **Manual validation:** use an explicitly authorized Windows test session for start/stop, tray sync, DNS adapter selection, private bypass, occupied ports, and real connected reload. Capture starting system-proxy settings and restore them; use disposable profiles and test nodes, not production subscriptions or secrets. Perform no network/environment changes merely to satisfy a static test.
5. **Exit evidence:** for each ID, link changed files/tests, record actual commands and results, document any skipped manual checks/blockers, and confirm no unrelated working-tree changes were overwritten. A phase is not "verified" when a required check failed or was skipped.

## 7. Audit observations outside this remediation contract

The audit also notes dashboard badges/tab remounts, duplicated polling, browser clipboard use, synchronous hostname resolution, wildcard Clash API CORS with an empty default secret, Hysteria2/TUIC editor-only support, route rules referencing absent sets, and unbounded architecture differences. They are **not** silently included in the 16-ID completion claim. Log them as separate issues with a security-first priority for the unauthenticated wildcard API; do not advertise Hysteria2/TUIC as supported until parsing, generation, validation, and connection tests exist. The connected-settings/routing and missing-rule-set aspects that overlap D2 **are** within D2's apply/error policy.

Likewise, this spec targets the audited Tauri checkout; it does not replace or amend any separately planned pure-Rust migration. Reconcile overlapping work before implementing a requirement on another branch.
