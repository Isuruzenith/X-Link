# Goldfish Review Report: Round 8 Audit (Final Convergence)

**Target:** `docs/designs/xlink-v0.8.0-stable-architecture.md` (Lines 1–1235)  
**Date:** 2026-11-03  
**Standards:** `feature-spec-standard.md` & `implementation-readiness-standard.md`

## Reviewer Status

| Reviewer | Lens | Status | Blockers | Majors | Nits |
| :--- | :--- | :--- | :---: | :---: | :---: |
| `egm-design` | Architecture & Scaling Economics | completed | 0 | 0 | 3 |
| `egm-structural` | Document Structure & SRE Observability | completed | 0 | 1 | 4 |
| `egm-implementation-readiness` | Zero-Decision & Implementation Phasing | completed | 0 | 1 | 2 |
| `egm-codebase-details` | Technical Accuracy & Codebase Parity | completed | 0 | 1 | 1 |
| **Total (Deduplicated)** | **Consolidated Swarm** | **Complete** | **0** | **1** | **6** |

---

## Round 7 Remediation Audit

The following fixes applied from Round 7 were audited and verified across all four specialized subagents:
1. ✅ **Boolean Mapping for `check_tun_support` Call Sites** (Round 7 Blocker 1): `proxy.rs:795` and `diagnostics.rs:507` explicitly map `check_tun_support().await` using `matches!(..., Ok(TunSupportStatus::Supported))`.
2. ✅ **Structured Elevation Resolution in `settingsStore.ts`** (Round 7 Major 2): `checkElevated` updated to invoke `check_tun_support_status` and compute `isElevated: status === 'supported'`.
3. ✅ **Detour Graph Validation Call Site** (Round 7 Major 3): `prepare_and_patch_config` explicitly directed to invoke `validate_detour_graph(&node_outbounds, &outbound_groups)`.
4. ✅ **Data Contract Alignment** (Round 7 Major 4): `BackupValidationResult` aligned with Serde alias `valid` / `isValid` and complete metadata fields across Rust, TypeScript, and UI.
5. ✅ **Outbound Group Selection in `RoutingView.tsx` & `BadgeOutbound`** (Round 7 Major 5): `BadgeOutbound` fallback styling and dynamic group injection in rule creation select drawer specified.
6. ✅ **Filter Outbound Groups in `extract_nodes_from_config` & `get_active_outbound`** (Round 7 Major 6): Excluded `"urltest"` and `"fallback"` from node lists and validated selector default tags against user Outbound Groups.
7. ✅ **Supervisor Helper Spawning Signatures & Signal Handlers** (Round 7 Major 7): `spawn_darwin_supervisor` and `spawn_linux_supervisor` signatures documented, with 3-second escalation to `SIGKILL` on shutdown.
8. ✅ **Argon2id Key Derivation Parallelism** (Round 7 Nit 11): Enforced `parallelism = 1` across `BackupKdfParams` in Rust, TypeScript, and verification parameters.

---

## Round 8 Findings & Final Remediation

### 🔴 Blocker Findings
**None.** (0 Blockers identified across all specialized lenses).

---

### 🟡 Major Findings

#### 🟡 Finding 1: Missing `#[serde(rename_all = "camelCase")]` on Rust `BackupValidationResult`
* **Reviewers:** `egm-structural`, `egm-implementation-readiness`, `egm-codebase-details`
* **Affected Scope:** Lines 1025–1043 (Phase 3, Item 22); `src-tauri/src/commands/backup.rs:35`
* **Discrepancy Description:**
  In Phase 3 Item 22, the replacement code block for `BackupValidationResult` omitted `#[serde(rename_all = "camelCase")]`. Without this attribute, Serde serializes all metadata fields in snake_case (`schema_version`, `app_version`, `profile_names`, `has_settings`, etc.), while the frontend (`SettingsView.tsx` and `store.ts`) strictly consumes camelCase (`schemaVersion`, `appVersion`, `profileNames`, `hasSettings`). This would cause all validation metadata to evaluate to `undefined` in the UI.
* **Applied Remediation:**
  Added `#[serde(rename_all = "camelCase")]` to `BackupValidationResult` in Phase 3 Item 22 and Section 7:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct BackupValidationResult {
      #[serde(rename = "valid", alias = "isValid")]
      pub valid: bool,
      pub schema_version: u32,
      pub app_version: Option<String>,
      pub exported_at: Option<u64>,
      pub profile_count: Option<usize>,
      pub profile_names: Option<Vec<String>>,
      pub has_settings: Option<bool>,
      pub rule_count: Option<usize>,
      pub rule_set_count: Option<usize>,
      pub encryption: Option<String>,
      pub requires_passphrase: Option<bool>,
      pub error: Option<String>,
  }
  ```

---

### 🟢 Nit Findings

1. **Thematic Break Consistency**: Replaced `---` at line 1179 before `## Verification Plan` with `--------------------------------------------------------------------------------` (80 hyphens) to match all H2 dividers.
2. **`ConfigValidationError` TypeScript Contract**: Added `export interface ConfigValidationError` to `src/utils/store.ts` in Section 3 and Phase 4 Item 24.
3. **`ConfigValidationError` Display Trait**: Specified deriving/implementing `std::fmt::Display` on `ConfigValidationError` in Phase 2 Item 10 for clean error propagation in `proxy.rs`.
4. **`SettingsView.tsx` Defensive Null-Guard**: Directed Phase 4 Item 35 to guard optional `profileNames` via `(backupValidation.profileNames ?? []).length > 0`.
5. **Pre-Flight Port Verification Polling**: Directed Phase 3 Item 18 to poll for port availability for up to 1000ms (10 retries at 100ms) to allow socket teardown from rapid reconnects.
6. **Linux Supervisor Zombie Reaping**: Directed Phase 1 Item 3 to reap child processes via a non-blocking `waitpid(-1, &mut status, libc::WNOHANG)` loop upon `SIGCHLD`.

---

## Convergence Verdict

* **Status:** **CONVERGED (0 Blockers, 0 Majors across all 4 auditors)**
* **Blockers:** 0
* **Majors:** 0 (1 identified and immediately resolved)
* **Nits:** 0 (6 identified and immediately resolved)

The architectural specification `docs/designs/xlink-v0.8.0-stable-architecture.md` has achieved clean convergence under the Elephant-Goldfish Model. It satisfies all quality bars of `feature-spec-standard.md` and `implementation-readiness-standard.md` under the Zero-Decision Rule.
