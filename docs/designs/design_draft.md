# X-Link sing-box configuration recovery

- **Author:** Isuruzenith
- **Status:** Approved design — implementation not started.
- **Date:** 2026-11-11

---

## The Problem

X-Link can generate a configuration that passes its initial sing-box check but fails on a later startup attempt. A retry disables uTLS on a Reality node even though the Reality client requires uTLS. Another retry adds `domain_resolver` to the root `proxy` selector, which does not accept that dial field. The result is repeated process exits and a generic failure after five attempts, rather than a useful explanation of the first failure.

X-Link also inserts or replaces TLS ALPN values that were not requested by the imported profile. This can change the behavior of transports such as gRPC, WebSocket, and HTTPUpgrade. A fallback must not silently alter the server settings supplied by the profile.

## Overview

Plan a single configuration pipeline for imported profiles and startup candidates. Build each candidate from a common generated configuration, preserve profile-specified transport and TLS settings, check the exact candidate with the bundled sing-box binary, and only then make it active. Treat a configuration error as a reason to stop and report the field that failed, not as a reason to repeat the same invalid startup.

## Goals

1. Preserve a profile's explicit ALPN values. If ALPN is absent, leave it absent; do not insert or override it in the generator or a fallback.
2. Never disable uTLS on a Reality client. Ensure Reality imports have the settings required by the bundled sing-box version, or reject them with a specific validation error before startup.
3. Apply resolver settings only where the pinned sing-box schema permits them. Never add dial fields to selector or URL-test outbounds.
4. Derive independent fallback candidates from the same baseline configuration, rather than patching the previous failed attempt.
5. Run the bundled sing-box configuration check on every candidate before activation. Preserve the previous active configuration when a candidate is invalid.
6. Cover the supplied VLESS Reality + gRPC case and existing TLS/transport behavior with regression tests, without copying private URI credentials into logs or diagnostics.

## Non-Goals

- Change the bundled sing-box version or introduce a new runtime dependency.
- Invent sing-box mappings for `spx` or `support-x25519mlkem768` without verifying support in the pinned binary.
- Guarantee that a configuration check proves remote server reachability, a successful Reality handshake, or the absence of DNS leaks.
- Redesign the profile UI or rewrite unrelated routing, TUN, and subscription features.

## Technical Plan

### Configuration flow

```text
Imported URI / Clash profile / stored sing-box node
                    |
                    v
        Preserve node-specific fields
                    |
                    v
        Generate shared baseline config
                    |
                    +--> baseline candidate --> sing-box check --> activate --> start/probe
                    |
                    +--> eligible recovery candidate from baseline
                         --> sing-box check --> activate --> start/probe
```

Keep the existing import adapters and generator as the input path. The generated baseline is immutable during one start operation. A recovery candidate is a fresh copy of that baseline with only the setting needed for that recovery changed. The candidate does not inherit changes from a failed recovery. Re-run static validation and the bundled sing-box `check` on the exact candidate before it replaces `active.json`. Configuration errors stop candidate execution and preserve the previously valid active file. Runtime probe errors and invalid configuration errors must be reported separately. `sing-box check` cannot prove the remote endpoint will connect.

### TLS and transport contract

The importer keeps explicit TLS, transport, and ALPN properties from the source. The generator must not supply a missing ALPN or replace an explicit one for any transport. Reality imports must include usable uTLS and a public key; validate the complete Reality client settings before startup. A recovery candidate cannot change Reality public key, short ID, SNI, fingerprint, transport, flow, ALPN, or other server protocol fields. Do not infer new sing-box properties from unrecognized URI query parameters. Tests against the pinned sing-box binary settle any uncertain protocol-specific behavior; a failure should be explained rather than bypassed by changing the profile.

### DNS and TUN recovery boundaries

A DNS recovery candidate may change only the resolver and DNS bootstrap configuration needed to resolve proxy server hostnames. Use the common route resolver for outbound hostname resolution; do not attach a dial field to a selector or URL-test group. Only attempt this recovery after a failure attributable to DNS resolution, and do not route DNS directly if the user's DNS-leak protection or routing constraints prohibit that change. A TUN compatibility candidate applies only to a TUN-specific failure and does not modify TLS or transport fields. A schema, missing-field, or unsupported-feature error fails immediately without consuming the remaining startup retries. Keep any runtime retry cap explicit; do not assume that five fixed attempts are necessary for all node types.

### File ownership and activation

The import adapters own parsing and explicit field mapping. The generator owns shared configuration assembly and preservation of profile data. The startup controller owns candidate selection, validation, activation, and error reporting. Keep validation on the same bundled sing-box `1.13.14` executable used to start the service. Stage each candidate in the configuration directory, check it, and commit it without losing the prior active configuration if the commit fails. Reuse or extend the existing transactional file-write mechanism instead of assuming that replacing an existing file with a simple rename works on Windows.

### Test boundaries

Unit tests cover absent and explicit ALPN, Reality uTLS invariants, candidate independence, and the outbound types that may accept resolver fields. An integration test uses a redacted fixture based on the supplied VLESS Reality + gRPC profile and checks generated candidates with the bundled binary without dialing the actual server. A local manual test checks whether the real profile connects, because neither unit tests nor `sing-box check` verify a remote handshake.

## Alternatives Considered

### Alternative A: Hardcode transport-aware ALPN defaults

Insert `["h2"]` for gRPC and `["http/1.1"]` for WS/HTTPUpgrade when the source profile has no ALPN.

- **Pros**: Explicit ALPN avoids relying on sing-box internal defaults.
- **Cons**: The hardcoded choice can conflict with certain server stacks. When sing-box changes its internal default, the hardcoded value lags. It adds a maintenance surface for every transport variant.
- **Reason for Rejection**: The sing-box binary already negotiates ALPN when the field is absent. Inserting a default silently overrides that negotiation and introduces a failure mode that does not exist in the original profile.

### Alternative B: Cumulative patching of active.json

Read the active configuration, apply one change (fingerprint, uTLS, DNS, TUN), write it back, and retry. Repeat on each failure, stacking changes from previous attempts.

- **Pros**: No extra file writes; reuses the same active path.
- **Cons**: Each failed candidate changes the next one. A DNS patch that adds `domain_resolver` to every non-direct outbound hits groups that reject the field. A uTLS-off patch breaks Reality. The exact configuration started is different from the one that passed `sing-box check`. Diagnosis requires reconstructing which patches were applied.
- **Reason for Rejection**: This is the current behavior and the direct cause of the reported errors. Independent candidates are easier to reason about and test.

### Alternative C: Pre-flight validation against a JSON schema extracted from the pinned binary

Parse the sing-box source or documentation to produce a JSON schema, then validate the configuration against it before invoking `sing-box check`.

- **Pros**: Would catch unknown-field errors without starting a process.
- **Cons**: Maintaining a schema in sync with the binary is a separate project. The binary's own `check` command already performs the validation, including cross-field constraints that a schema cannot express. A schema mismatch would produce false positives or false negatives.
- **Reason for Rejection**: Running `sing-box check` on the candidate achieves the same result without a separate artifact to maintain.

## Detailed Implementation Plan

### Phase 1 — Preserve imported TLS and transport intent

1. **`src-tauri/src/config/adapters/raw_uri.rs`** — Keep the existing VLESS URI parsing of Reality public key, short ID, `fp` and gRPC `serviceName`. Add focused tests using synthetic credentials for a Reality + gRPC URI with `fp=chrome`, `serviceName=teams`, and no `alpn`, plus a URI with explicit ALPN. Assert the resulting `tls.utls`, `tls.reality`, `transport.service_name`, and ALPN presence/absence; do not map unknown `spx` or `support-x25519mlkem768` query parameters without schema evidence. Require a usable fingerprint for Reality if the importer is responsible for rejecting incomplete Reality URIs; otherwise let Phase 2 reject them before startup.
2. **`src-tauri/src/config/adapters/clash.rs`** — Extend existing TLS mapping tests for `client-fingerprint`/`fingerprint`, `reality-opts`, and an explicit ALPN. Keep the source's ALPN untouched; absent ALPN must remain absent. This prevents fixing the URI path while leaving an equivalent Clash import broken.
3. **`src-tauri/src/config/generator.rs`** — Remove the TLS-wide default `alpn: ["http/1.1"]` insertion and the WS/HTTPUpgrade branch that overwrites ALPN. Preserve the HTTPUpgrade Host-header normalization and unrelated transport adjustments. Extend `prepare_node_outbound` to reject incomplete Reality client settings (TLS disabled, insecure TLS, blank public key, or uTLS absent/disabled/without a usable fingerprint) with a node-specific validation error. Keep this check scoped to Reality, not ordinary TLS or group outbounds. Replace the existing ALPN self-healing and HTTPUpgrade ALPN tests with absence/preservation tests for gRPC, WS, HTTPUpgrade and other TLS outbounds; retain existing UDP transport-specific behavior where applicable. Assert that generation does not change an explicit ALPN, even when it would conflict with the transport, because resolving that conflict belongs to an explicit profile edit rather than silent rewriting.

**Phase 1 exit:** A generated profile with the synthetic VLESS Reality + gRPC fixture has Reality/uTLS/gRPC properties, no injected ALPN, and a specific error when Reality is missing required client fields.

### Phase 2 — Make candidates independent and activate only validated bytes

4. **`src-tauri/src/commands/proxy.rs`** — Separate baseline generation from candidate activation in `prepare_and_patch_config`: retain the final generated `serde_json::Value`/serialized bytes for one startup operation, with no mutation of `active.json` by retry logic. Extract a small pure candidate builder accepting baseline plus a typed recovery kind and returning new candidate bytes; each kind starts from the same baseline, never a prior candidate. Remove `patch_config_for_fallback` and the TUN closure's in-place write. Treat fingerprint switching and disabling uTLS as *not* automatic recovery: they modify server-protocol intent, and uTLS-off is invalid for Reality. Initially support only an eligible DNS-resolution candidate and a TUN-compatibility candidate. Keep baseline as the first candidate. If an attempted recovery would be identical to baseline, skip it rather than starting an extra process.
5. **`src-tauri/src/commands/proxy.rs`** — Stage each candidate beside `active.json`, call the existing async `validate_singbox_config` on that staged path, and commit exactly the checked bytes before spawning the sidecar. Do not start on a candidate that fails static validation or `check`; report its category and diagnostic and stop the startup, leaving the last valid active config intact. Remove the early unvalidated `active.json` creation in `prepare_and_patch_config` when no profile is selected. Keep a snapshot of the prior active bytes for rollback if a staged commit fails; inspect existing profile-switch restoration behavior before changing its call sites. Ensure the startup process has stopped and released the old config before an activation/rollback on Windows, and clean up staged/backup files on both success and failure.
6. **`src-tauri/src/commands/config.rs`** — Reuse or narrowly extend its transaction/restore helpers for a single-file candidate commit. Review `commit_files_transactionally_with` on Windows: its current rename to an existing target may fail, so implement a Windows-safe replace/rollback sequence that preserves a recoverable backup and propagates rollback failure explicitly. Apply the corrected helper to the existing profile-switch commits as well; never remove the old target without a recoverable copy. Add tests for existing-target replacement, an injected rename/commit failure, rollback and cleanup. No new crate dependency is expected.
7. **`src-tauri/src/config/validator.rs`** — Reuse `validate_singbox_config(app, staged_path)` for each candidate rather than introducing a second JSON validator or a duplicated sidecar invocation. Keep `check` and startup on the same bundled executable and compatible flags. In proxy orchestration, retain the precise `sing-box check` error; do not flatten schema failures into a generic retry message.

**Phase 2 exit:** For every attempted candidate, the bytes given to `sing-box check` are the bytes activated and started; a rejected candidate cannot overwrite the previously valid active file. No retry writes directly to `active.json`.

### Phase 3 — Restrict recovery to the failure it can address

8. **`src-tauri/src/commands/proxy.rs`** — Replace the numeric `attempt` mapping with explicit recovery eligibility. Invalid config, missing Reality/uTLS fields, unsupported feature, sidecar compatibility failure, and spawn failure stop immediately. Only a narrowly identified DNS-resolution failure can request the DNS candidate; only a TUN-specific startup/probe failure in TUN mode can request the TUN candidate. A generic HTTP probe failure must *not* be inferred to be DNS. If existing health/exit signals cannot reliably distinguish DNS from other failures, leave DNS recovery disabled for that signal and return the diagnostic rather than speculating. Record the attempted candidate and reason in sanitized logs, and report the last meaningful error instead of always saying five attempts occurred. Bound the candidate set by eligible distinct recoveries; avoid a fixed retry count that creates no-op attempts.
9. **`src-tauri/src/commands/proxy.rs`** — For an eligible DNS candidate, adjust only the necessary DNS server/bootstrap and `route.default_domain_resolver` fields on a fresh baseline. Do not set `domain_resolver` on every non-direct outbound: the root `proxy` selector and URL-test groups do not accept that dial field. Leave proxy TLS, gRPC transport, ALPN and selection unchanged. Before using a public direct-detoured bootstrap or adding TUN route exclusions, check the existing DNS-leak protection and routing policy; when the policy forbids it, skip recovery and surface the DNS failure instead. Do not rewrite all DNS rules with a `domain` key; change only the specific proxy-hostname resolution rule after identifying it. If there is no safe or identifiable rule, skip the candidate. Implement TUN compatibility by applying the existing `apply_tun_compatibility_profile` to a new baseline copy only for an eligible TUN failure, then check it before activation.
10. **`src-tauri/src/constants.rs`** — Remove or revise `MAX_FALLBACK_ATTEMPTS` and `MAX_CONFIG_PATCH_ATTEMPTS` once the numeric retry flow is replaced, after checking all references. Preserve shared DNS constants only if the constrained DNS candidate still needs them. Avoid constants that advertise retries no longer performed.

**Phase 3 exit:** The startup path cannot construct the two reported invalid configs; failures that lack a safe, attributable recovery stop with their original diagnostic, and eligible recovery candidates remain independent of one another.

### Phase 4 — Regression and executable verification

11. **Tests in `raw_uri.rs`, `clash.rs`, `generator.rs`, `commands/proxy.rs`, and `commands/config.rs`** — Cover import and generation for absent/explicit ALPN, Reality with/without uTLS and fingerprint, gRPC service name, selector/URL-test preservation, DNS privacy gating, candidate independence, candidate eligibility, and transactional restoration. Update the existing proxy DNS fallback test that currently expects `proxy_node["domain_resolver"]`; assert instead that the group has no dial-only resolver field and the permitted route resolver is correct. Test process-exit classification separately from probe failures where practical. Use synthetic credentials, non-routable fixture endpoints, and redacted assertions; never commit or print the supplied private URI.
12. **Existing bundled sidecar and test setup (no version change)** — Run Rust formatting and targeted Rust tests, then run the affected backend test suite if the environment permits. Generate the synthetic Reality + gRPC configuration and invoke the bundled `sing-box 1.13.14 check` on its staged path with the same data-directory and compatibility flags as the app. Test baseline and each eligible candidate; an unknown-field diagnostic must fail the test. On Windows, additionally exercise existing-target commit and rollback. If sidecar or Cargo dependencies are unavailable, state which checks could not run rather than reporting them as passed.
13. **Manual acceptance** — In a local authorized run with the real profile, verify startup and actual connectivity without logging its URI, public key/UUID combination, or credentials. Compare redacted generated fields to the profile; record whether the connection works separately from the static `check` result. Verify that a forced invalid candidate leaves the previous active config intact, and that both cited fatal errors are absent. A successful static check alone is not acceptance of a working remote connection.

**Acceptance criteria:** Explicit ALPN stays exact; missing ALPN is omitted. Reality always has enabled uTLS and a usable fingerprint or is rejected before startup. gRPC retains its supplied service name. Selectors and URL-test groups never gain dial-only `domain_resolver`. Each started config is checked first; recoveries neither accumulate changes nor bypass DNS privacy/routing policy; failed candidate validation preserves the prior active file. Production code remains untouched until this document is approved and implementation is separately requested.
